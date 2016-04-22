use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use std::sync::atomic::{AtomicUsize, Ordering};
use mopa::Any;
use {Index, Generation, Entity, StorageBase, Storage};
use bitset::{AtomicBitSet, BitSet, BitSetLike, BitSetOr, MAX_EID};
use join::{Open, Get};


/// Abstract component type. Doesn't have to be Copy or even Clone.
pub trait Component: Any + Sized {
    /// Associated storage type for this component.
    type Storage: Storage<Component=Self> + Any + Send + Sync;
}


/// A custom entity guard used to hide the the fact that Generations
/// is lazily created and updated. For this to be useful it _must_
/// be joined with a component. This is because the Generation table
/// includes every possible Generation of Entities even if they
/// have never been
pub struct Entities<'a> {
    guard: RwLockReadGuard<'a, Allocator>,
}

impl<'a> Open for &'a Entities<'a> {
    type Value = Self;
    type Mask = BitSetOr<&'a BitSet, &'a AtomicBitSet>;
    fn open(self) -> (Self::Mask, Self::Value) {
        (BitSetOr(&self.guard.alive, &self.guard.raised), self)
    }
}

impl<'a> Get for &'a Entities<'a> {
    type Value = Entity;
    unsafe fn get(&self, idx: Index) -> Self::Value {
        let gen = self.guard.generations.get(idx as usize)
            .map(|&gen| if gen.is_alive() { gen } else { gen.raised() })
            .unwrap_or(Generation(1));
        Entity(idx, gen)
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


struct Allocator {
    generations: Vec<Generation>,
    alive: BitSet,
    raised: AtomicBitSet,
    killed: AtomicBitSet,
    start_from: AtomicUsize
}

impl Allocator {
    fn new() -> Allocator {
        Allocator {
            generations: vec![],
            alive: BitSet::new(),
            raised: AtomicBitSet::with_capacity(MAX_EID as u32),
            killed: AtomicBitSet::with_capacity(MAX_EID as u32),
            start_from: AtomicUsize::new(0)
        }
    }

    fn kill(&self, idx: Index) {
        self.killed.add_atomic(idx);
    }

    /// Attempt to move the `start_from` value
    fn update_start_from(&self, start_from: usize) {
        loop {
            let current = self.start_from.load(Ordering::Relaxed);

            // if the current value is bigger then ours, we bail
            if current >= start_from {
                return;
            }

            if start_from == self.start_from.compare_and_swap(current, start_from, Ordering::Relaxed) {
                return;
            }
        }
    }

    /// Allocate a new entity
    fn allocate_atomic(&self) -> Entity {
        let idx = self.start_from.load(Ordering::Relaxed);
        for i in idx.. {
            if !self.alive.contains(i as Index) && !self.raised.add_atomic(i as Index) {
                self.update_start_from(i+1);

                let gen = self.generations.get(idx as usize)
                    .map(|&gen| if gen.is_alive() { gen } else { gen.raised() })
                    .unwrap_or(Generation(1));

                return Entity(i as Index, gen);
            }
        }
        panic!("No entities left to allocate")
    }

    /// Allocate a new entity
    fn allocate(&mut self) -> Entity {
        let idx = self.start_from.load(Ordering::Relaxed);
        for i in idx.. {
            if !self.raised.contains(i as Index) && !self.alive.add(i as Index) {
                self.update_start_from(i+1);

                while self.generations.len() <= i as usize {
                    self.generations.push(Generation(0));
                }
                self.generations[i as usize] = self.generations[i as usize].raised();

                return Entity(i as Index, self.generations[i as usize]);
            }
        }
        panic!("No entities left to allocate")
    }

    fn merge(&mut self) -> Vec<Entity> {
        let mut deleted = vec![];

        for i in (&self.raised).iter() {
            while self.generations.len() <= i as usize {
                self.generations.push(Generation(0));
            }
            self.generations[i as usize] = self.generations[i as usize].raised();
            self.alive.add(i);
        }
        self.raised.clear();

        if let Some(lowest) = (&self.killed).iter().next() {
            if lowest < self.start_from.load(Ordering::Relaxed) as Index {
                self.start_from.store(lowest as usize, Ordering::Relaxed);
            }
        }

        for i in (&self.killed).iter() {
            self.alive.remove(i);
            self.generations[i as usize].die();
            deleted.push(Entity(i, self.generations[i as usize]))
        }
        self.killed.clear();

        deleted
    }
}

/// Entity creation iterator. Will yield new empty entities infinitely.
/// Useful for bulk entity construction, since the locks are only happening once.
pub struct CreateEntities<'a> {
    allocate: RwLockWriteGuard<'a, Allocator>,
}

impl<'a> Iterator for CreateEntities<'a> {
    type Item = Entity;
    fn next(&mut self) -> Option<Entity> {
        Some(self.allocate.allocate())
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
    allocator: RwLock<Allocator>,
    components: HashMap<TypeId, Box<StorageLock>>,
}

impl World {
    /// Creates a new empty `World`.
    pub fn new() -> World {
        World {
            components: HashMap::new(),
            allocator: RwLock::new(Allocator::new())
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
    pub fn entities(&self) -> Entities {
        Entities {
            guard: self.allocator.read().unwrap(),
        }
    }
    /// Returns the entity creation iterator. Can be used to create many
    /// empty entities at once without paying the locking overhead.
    pub fn create_iter(&self) -> CreateEntities {
        CreateEntities {
            allocate: self.allocator.write().unwrap(),
        }
    }
    /// Creates a new entity instantly, locking the generations data.
    pub fn create_now(&self) -> EntityBuilder {
        let id = self.allocator.write().unwrap().allocate();
        EntityBuilder(id, self)
    }
    /// Deletes a new entity instantly, locking the generations data.
    pub fn delete_now(&self, entity: Entity) {
        for comp in self.components.values() {
            comp.del_slice(&[entity]);
        }
        let mut gens = self.allocator.write().unwrap();
        gens.generations[entity.get_id()].die();
        gens.alive.remove(entity.get_id() as Index);
        gens.raised.remove(entity.get_id() as Index);
        if entity.get_id() < gens.start_from.load(Ordering::Relaxed) {
            gens.start_from.store(entity.get_id(), Ordering::Relaxed);
        }
    }
    /// Creates a new entity dynamically.
    pub fn create_later(&self) -> Entity {
        let allocator = self.allocator.read().unwrap();
        allocator.allocate_atomic()
    }
    /// Deletes an entity dynamically.
    pub fn delete_later(&self, entity: Entity) {
        let allocator = self.allocator.read().unwrap();
        allocator.kill(entity.get_id() as Index);
    }
    /// Returns `true` if the given `Entity` is alive.
    pub fn is_alive(&self, entity: Entity) -> bool {
        debug_assert!(entity.get_gen().is_alive());
        let gens = self.allocator.read().unwrap();
        gens.generations.get(entity.get_id()).map(|&x| x == entity.get_gen()).unwrap_or(false)
    }
    /// Merges in the appendix, recording all the dynamically created
    /// and deleted entities into the persistent generations vector.
    /// Also removes all the abandoned components.
    pub fn merge(&self) {
        let mut allocator = self.allocator.write().unwrap();

        let temp_list = allocator.merge();
        for comp in self.components.values() {
            comp.del_slice(&temp_list);
        }
    }
}

/// System fetch-time argument. The fetch is executed at the start of the run.
/// It contains a subset of `World` methods that make sense during initialization.
#[derive(Copy, Clone)]
pub struct FetchArg<'a>(&'a World);

impl<'a> FetchArg<'a> {
    /// Constructs a new `FetchArg`, not supposed to be used.
    #[doc(hidden)]
    pub fn new(w: &'a World) -> FetchArg<'a> {
        FetchArg(w)
    }
    /// Locks a `Component` for reading.
    pub fn read<T: Component>(self) -> RwLockReadGuard<'a, T::Storage> {
        self.0.read::<T>()
    }
    /// Locks a `Component` for writing.
    pub fn write<T: Component>(self) -> RwLockWriteGuard<'a, T::Storage> {
        self.0.write::<T>()
    }
    /// Returns the entity iterator.
    pub fn entities(self) -> Entities<'a> {
        self.0.entities()
    }
}
