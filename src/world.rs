use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use mopa::Any;
use {Index, Generation, Entity, StorageBase, Storage};


/// Abstract component type. Doesn't have to be Copy or even Clone.
pub trait Component: Any + Sized {
    /// Associated storage type for this component.
    type Storage: Storage<Self> + Any + Send + Sync;
}


/// A custom entity iterator. Needed because the world doesn't really store
/// entities directly, but rather has just a vector of Index -> Generation.
pub struct EntityIter<'a> {
    guard: RwLockReadGuard<'a, Vec<Generation>>,
    index: usize,
}

impl<'a> Iterator for EntityIter<'a> {
    type Item = Entity;
    fn next(&mut self) -> Option<Entity> {
        loop {
            match self.guard.get(self.index) {
                Some(&gen) if gen > 0 => {
                    let ent = Entity(self.index as Index, gen);
                    self.index += 1;
                    return Some(ent)
                },
                Some(_) => self.index += 1, // continue
                None => return None,
            }
        }
    }
}

/// Helper builder for entities.
pub struct EntityBuilder<'a>(Entity, &'a World);

impl<'a> EntityBuilder<'a> {
    /// Add a component value to the new entity.
    pub fn with<T: Component>(self, value: T) -> EntityBuilder<'a> {
        self.1.write::<T>().insert(self.0, value);
        self
    }
    /// Finish entity construction.
    pub fn build(self) -> Entity {
        self.0
    }
}


struct Appendix {
    next: Entity,
    add_queue: Vec<Entity>,
    sub_queue: Vec<Entity>,
}

fn find_next(gens: &[Generation], lowest_free_index: usize) -> Entity {
    if let Some((id, gen)) = gens.iter().enumerate().skip(lowest_free_index).find(|&(_, g)| *g <= 0) {
        return Entity(id as Index, 1 - gen);
    }

    if lowest_free_index > gens.len() {
        return Entity(lowest_free_index as Index, 1);
    } else {
        return Entity(gens.len() as Index, 1);
    }
}

/// Entity creation iterator. Will yield new empty entities infinitely.
/// Useful for bulk entity construction, since the locks are only happening once.
pub struct CreateEntityIter<'a> {
    gens: RwLockWriteGuard<'a, Vec<Generation>>,
    app: RwLockWriteGuard<'a, Appendix>,
}

impl<'a> Iterator for CreateEntityIter<'a> {
    type Item = Entity;
    fn next(&mut self) -> Option<Entity> {
        let ent = self.app.next;
        assert!(ent.get_gen() > 0);
        if ent.get_gen() == 1 {
            assert!(self.gens.len() == ent.get_id());
            self.gens.push(ent.get_gen());
            self.app.next.0 += 1;
        } else {
            self.app.next = find_next(&self.gens, ent.get_id() + 1);
        }
        Some(ent)
    }
}

/// A custom entity iterator for dynamically added entities.
pub struct DynamicEntityIter<'a> {
    guard: RwLockReadGuard<'a, Appendix>,
    index: usize,
}

impl<'a> Iterator for DynamicEntityIter<'a> {
    type Item = Entity;
    fn next(&mut self) -> Option<Entity> {
        let ent = self.guard.add_queue.get(self.index);
        self.index += 1;
        ent.map(|e| *e)
    }
}


trait StorageLock: Any + Send + Sync {
    fn del_slice(&self, &[Entity]);
}

mopafy!(StorageLock);

impl<S: StorageBase + Any + Send + Sync> StorageLock for RwLock<S> {
    fn del_slice(&self, entities: &[Entity]) {
        let mut guard = self.write().unwrap();
        for &e in entities.iter() {
            guard.del(e);
        }
    }
}


/// The world struct contains all the data, which is entities and their components.
/// The methods are supposed to be valid for any context they are available in.
pub struct World {
    generations: RwLock<Vec<Generation>>,
    components: HashMap<TypeId, Box<StorageLock>>,
    appendix: RwLock<Appendix>,
}

impl World {
    /// Create a new empty world.
    pub fn new() -> World {
        World {
            generations: RwLock::new(Vec::new()),
            components: HashMap::new(),
            appendix: RwLock::new(Appendix {
                next: Entity(0, 1),
                add_queue: Vec::new(),
                sub_queue: Vec::new(),
            }),
        }
    }
    /// Register a new component type.
    pub fn register<T: Component>(&mut self) {
        let any = RwLock::new(T::Storage::new());
        self.components.insert(TypeId::of::<T>(), Box::new(any));
    }
    /// Unregister a component type.
    pub fn unregister<T: Component>(&mut self) -> Option<T::Storage> {
        self.components.remove(&TypeId::of::<T>()).map(|boxed|
            match boxed.downcast::<RwLock<T::Storage>>() {
                Ok(b) => (*b).into_inner().unwrap(),
                Err(_) => panic!("Unable to downcast the storage type"),
            }
        )
    }
    fn lock<T: Component>(&self) -> &RwLock<T::Storage> {
        let boxed = self.components.get(&TypeId::of::<T>()).unwrap();
        boxed.downcast_ref().unwrap()
    }
    /// Lock a component for reading.
    pub fn read<'a, T: Component>(&'a self) -> RwLockReadGuard<'a, T::Storage> {
        self.lock::<T>().read().unwrap()
    }
    /// Lock a component for writing.
    pub fn write<'a, T: Component>(&'a self) -> RwLockWriteGuard<'a, T::Storage> {
        self.lock::<T>().write().unwrap()
    }
    /// Return the entity iterator.
    pub fn entities<'a>(&'a self) -> EntityIter<'a> {
        EntityIter {
            guard: self.generations.read().unwrap(),
            index: 0,
        }
    }
    /// Return the dynamic entity iterator. It goes through entities that were
    /// dynamically created by systems but not yet merged.
    pub fn dynamic_entities<'a>(&'a self) -> DynamicEntityIter<'a> {
        DynamicEntityIter {
            guard: self.appendix.read().unwrap(),
            index: 0,
        }
    }
    /// Return the entity creation iterator. Can be used to create many
    /// empty entities at once without paying the locking overhead.
    pub fn create_iter<'a>(&'a self) -> CreateEntityIter<'a> {
        CreateEntityIter {
            gens: self.generations.write().unwrap(),
            app: self.appendix.write().unwrap(),
        }
    }
    /// Create a new entity instantly, with locking the generations data.
    pub fn create_now<'a>(&'a self) -> EntityBuilder<'a> {
        let mut app = self.appendix.write().unwrap();
        let ent = app.next;
        assert!(ent.get_gen() > 0);
        if ent.get_gen() == 1 {
            let mut gens = self.generations.write().unwrap();
            assert!(gens.len() == ent.get_id());
            gens.push(ent.get_gen());
            app.next.0 += 1;
        } else {
            let gens = self.generations.read().unwrap();
            app.next = find_next(&gens, ent.get_id() + 1);
        }
        EntityBuilder(ent, self)
    }
    /// Delete a new entity instantly, with locking the generations data.
    pub fn delete_now(&self, entity: Entity) {
        for comp in self.components.values() {
            comp.del_slice(&[entity]);
        }
        let mut gens = self.generations.write().unwrap();
        let mut gen = &mut gens[entity.get_id() as usize];
        assert!(*gen > 0);
        let mut app = self.appendix.write().unwrap();
        if entity.get_id() < app.next.get_id() {
            app.next = Entity(entity.0, *gen+1);
        }
        *gen *= -1;
    }
    /// Create a new entity dynamically.
    pub fn create_later(&self) -> Entity {
        let mut app = self.appendix.write().unwrap();
        let ent = app.next;
        app.add_queue.push(ent);
        app.next = find_next(&*self.generations.read().unwrap(), ent.get_id() + 1);
        ent
    }
    /// Delete an entity dynamically.
    pub fn delete_later(&self, entity: Entity) {
        let mut app = self.appendix.write().unwrap();
        app.sub_queue.push(entity);
    }
    /// Merge in the appendix, recording all the dynamically created
    /// and deleted entities into the persistent generations vector.
    /// Also removes all the abandoned components.
    pub fn merge(&self) {
        // We can't lock Appendix and components at the same time,
        // or otherwise we deadlock with a system that tries to process
        // newly added entities.
        // So we copy dead components out first, and then process them separately.
        let mut temp_list = Vec::new(); //TODO: avoid allocation
        {
            let mut gens = self.generations.write().unwrap();
            let mut app = self.appendix.write().unwrap();
            for ent in app.add_queue.drain(..) {
                while gens.len() <= ent.get_id() {
                    gens.push(0);
                }
                assert_eq!(ent.get_gen(), 1 - gens[ent.get_id()]);
                gens[ent.get_id()] = ent.get_gen();
            }
            let mut next = app.next;
            for ent in app.sub_queue.drain(..) {
                assert_eq!(ent.get_gen(), gens[ent.get_id()]);
                temp_list.push(ent);
                if ent.get_id() < next.get_id() {
                    next = Entity(ent.0, ent.1 + 1);
                }
                gens[ent.get_id()] *= -1;
            }
            app.next = next;
        }
        for comp in self.components.values() {
            comp.del_slice(&temp_list);
        }
    }
}

/// System fetch-time argument. The fetch is executed at the start of the run.
/// It contains a subset of World methods that make sense during initialization.
pub struct FetchArg<'a>(&'a World);

impl<'a> FetchArg<'a> {
    /// Construct the new arg, not supposed to be used.
    #[doc(hidden)]
    pub fn new(w: &'a World) -> FetchArg<'a> {
        FetchArg(w)
    }
    /// Lock a component for reading.
    pub fn read<T: Component>(&self) -> RwLockReadGuard<'a, T::Storage> {
        self.0.read::<T>()
    }
    /// Lock a component for writing.
    pub fn write<T: Component>(&self) -> RwLockWriteGuard<'a, T::Storage> {
        self.0.write::<T>()
    }
    /// Return the entity iterator.
    pub fn entities(self) -> EntityIter<'a> {
        self.0.entities()
    }
}
