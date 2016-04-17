use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use mopa::Any;
use {Index, Generation, Entity, StorageBase, Storage};


/// Abstract component type. Doesn't have to be Copy or even Clone.
pub trait Component: Any + Sized {
    /// Associated storage type for this component.
    type Storage: Storage<Component=Self> + Any + Send + Sync;
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
                Some(&gen) if gen.is_alive() => {
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
    /// Adds a `Component` value to the new `Entity`.
    pub fn with<T: Component>(self, value: T) -> EntityBuilder<'a> {
        self.1.write::<T>().insert(self.0, value);
        self
    }
    /// Finishes entity construction.
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
    if let Some((id, gen)) = gens.iter().enumerate().skip(lowest_free_index).find(|&(_, g)| !g.is_alive()) {
        return Entity(id as Index, gen.raised());
    }

    let new_index = if lowest_free_index > gens.len() {
        lowest_free_index as Index
    } else {
        gens.len() as Index
    };

    Entity(new_index, Generation(1))
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
        assert!(ent.get_gen().is_alive());
        if ent.get_gen().is_first() {
            assert_eq!(self.gens.len(), ent.get_id());
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
        ent.cloned()
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


/// The `World` struct contains all the data, which is entities and their components.
/// All methods are supposed to be valid for any context they are available in.
pub struct World {
    generations: RwLock<Vec<Generation>>,
    components: HashMap<TypeId, Box<StorageLock>>,
    appendix: RwLock<Appendix>,
}

impl World {
    /// Creates a new empty `World`.
    pub fn new() -> World {
        World {
            generations: RwLock::new(Vec::new()),
            components: HashMap::new(),
            appendix: RwLock::new(Appendix {
                next: Entity(0, Generation(1)),
                add_queue: Vec::new(),
                sub_queue: Vec::new(),
            }),
        }
    }
    /// Registers a new component type.
    pub fn register<T: Component>(&mut self) {
        let any = RwLock::new(T::Storage::new());
        self.components.insert(TypeId::of::<T>(), Box::new(any));
    }
    /// Unregisters a component type.
    pub fn unregister<T: Component>(&mut self) -> Option<T::Storage> {
        self.components.remove(&TypeId::of::<T>()).map(|boxed|
            match boxed.downcast::<RwLock<T::Storage>>() {
                Ok(b) => (*b).into_inner().unwrap(),
                Err(_) => panic!("Unable to downcast the storage type"),
            }
        )
    }
    fn lock<T: Component>(&self) -> &RwLock<T::Storage> {
        let boxed = self.components.get(&TypeId::of::<T>())
            .expect("Tried to perform an operation on type that was not registered");
        boxed.downcast_ref().unwrap()
    }
    /// Locks a component's storage for reading.
    pub fn read<T: Component>(&self) -> RwLockReadGuard<T::Storage> {
        self.lock::<T>().read().unwrap()
    }
    /// Locks a component's storage for writing.
    pub fn write<T: Component>(&self) -> RwLockWriteGuard<T::Storage> {
        self.lock::<T>().write().unwrap()
    }
    /// Returns the entity iterator.
    pub fn entities(&self) -> EntityIter {
        EntityIter {
            guard: self.generations.read().unwrap(),
            index: 0,
        }
    }
    /// Returns the dynamic entity iterator. It iterates over entities that were
    /// dynamically created by systems but not yet merged.
    pub fn dynamic_entities(&self) -> DynamicEntityIter {
        DynamicEntityIter {
            guard: self.appendix.read().unwrap(),
            index: 0,
        }
    }
    /// Returns the entity creation iterator. Can be used to create many
    /// empty entities at once without paying the locking overhead.
    pub fn create_iter(&self) -> CreateEntityIter {
        CreateEntityIter {
            gens: self.generations.write().unwrap(),
            app: self.appendix.write().unwrap(),
        }
    }
    /// Creates a new entity instantly, locking the generations data.
    pub fn create_now(&self) -> EntityBuilder {
        let mut gens = self.generations.write().unwrap();
        let mut app = self.appendix.write().unwrap();
        let ent = app.next;
        assert!(ent.get_gen().is_alive());
        if ent.get_gen().is_first() {
            assert_eq!(gens.len(), ent.get_id());
            gens.push(ent.get_gen());
            app.next.0 += 1;
        } else {
            assert!(!gens[ent.get_id()].is_alive());
            gens[ent.get_id()] = ent.get_gen();
            app.next = find_next(&gens, ent.get_id() + 1);
        }
        EntityBuilder(ent, self)
    }
    /// Deletes a new entity instantly, locking the generations data.
    pub fn delete_now(&self, entity: Entity) {
        for comp in self.components.values() {
            comp.del_slice(&[entity]);
        }
        let mut gens = self.generations.write().unwrap();
        let mut gen = &mut gens[entity.get_id()];
        gen.die();
        let mut app = self.appendix.write().unwrap();
        if entity.get_id() < app.next.get_id() {
            app.next = Entity(entity.0, gen.raised());
        }
    }
    /// Creates a new entity dynamically.
    pub fn create_later(&self) -> Entity {
        let gens = self.generations.read().unwrap();
        let mut app = self.appendix.write().unwrap();
        let ent = app.next;
        app.add_queue.push(ent);
        app.next = find_next(&gens, ent.get_id() + 1);
        ent
    }
    /// Deletes an entity dynamically.
    pub fn delete_later(&self, entity: Entity) {
        let mut app = self.appendix.write().unwrap();
        app.sub_queue.push(entity);
    }
    /// Returns `true` if the given `Entity` is alive.
    pub fn is_alive(&self, entity: Entity) -> bool {
        debug_assert!(entity.get_gen().is_alive());
        let gens = self.generations.read().unwrap();
        entity.get_gen() == gens[entity.get_id()]
    }
    /// Merges in the appendix, recording all the dynamically created
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
                    gens.push(Generation(0));
                }
                assert_eq!(ent.get_gen(), gens[ent.get_id()].raised());
                gens[ent.get_id()] = ent.get_gen();
            }
            let mut next = app.next;
            for ent in app.sub_queue.drain(..) {
                let gen = &mut gens[ent.get_id()];
                if gen.is_alive() {
                    assert_eq!(ent.get_gen(), *gen);
                    gen.die();
                    temp_list.push(ent);
                    if ent.get_id() < next.get_id() {
                        next = Entity(ent.0, gen.raised());
                    }
                } else {
                    let mut g = ent.get_gen();
                    g.die();
                    debug_assert_eq!(g, *gen);
                }
            }
            app.next = next;
        }
        for comp in self.components.values() {
            comp.del_slice(&temp_list);
        }
    }
}

/// System fetch-time argument. The fetch is executed at the start of the run.
/// It contains a subset of `World` methods that make sense during initialization.
pub struct FetchArg<'a>(&'a World);

impl<'a> FetchArg<'a> {
    /// Constructs a new `FetchArg`, not supposed to be used.
    #[doc(hidden)]
    pub fn new(w: &'a World) -> FetchArg<'a> {
        FetchArg(w)
    }
    /// Locks a `Component` for reading.
    pub fn read<T: Component>(&self) -> RwLockReadGuard<'a, T::Storage> {
        self.0.read::<T>()
    }
    /// Locks a `Component` for writing.
    pub fn write<T: Component>(&self) -> RwLockWriteGuard<'a, T::Storage> {
        self.0.write::<T>()
    }
    /// Returns the entity iterator.
    pub fn entities(self) -> EntityIter<'a> {
        self.0.entities()
    }
}
