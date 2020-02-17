use std::{
    fmt,
    num::NonZeroI32,
    sync::atomic::{AtomicUsize, Ordering},
};

use hibitset::{AtomicBitSet, BitSet, BitSetOr};
use shred::Read;

#[cfg(feature = "parallel")]
use crate::join::ParJoin;
use crate::{error::WrongGeneration, join::Join, storage::WriteStorage, world::Component};

/// An index is basically the id of an `Entity`.
pub type Index = u32;

/// A wrapper for a read `Entities` resource.
/// Note that this is just `Read<Entities>`, so
/// you can easily use it in your system:
///
/// ```
/// # use specs::prelude::*;
/// # struct Sys;
/// # impl<'a> System<'a> for Sys {
/// type SystemData = (Entities<'a> /* ... */,);
/// # fn run(&mut self, _: Self::SystemData) {}
/// # }
/// ```
///
/// Please note that you should call `World::maintain`
/// after creating / deleting entities with this resource.
///
/// When `.join`ing on `Entities`, you will need to do it like this:
///
/// ```
/// use specs::prelude::*;
///
/// # struct Pos; impl Component for Pos { type Storage = VecStorage<Self>; }
/// # let mut world = World::new(); world.register::<Pos>();
/// # let entities = world.entities(); let positions = world.write_storage::<Pos>();
/// for (e, pos) in (&entities, &positions).join() {
///     // Do something
/// #   let _ = e;
/// #   let _ = pos;
/// }
/// ```
pub type Entities<'a> = Read<'a, EntitiesRes>;

/// Internally used structure for `Entity` allocation.
#[derive(Default, Debug)]
pub(crate) struct Allocator {
    generations: Vec<ZeroableGeneration>,

    alive: BitSet,
    raised: AtomicBitSet,
    killed: AtomicBitSet,
    cache: EntityCache,
    max_id: AtomicUsize,
}

impl Allocator {
    /// Kills a list of entities immediately.
    pub fn kill(&mut self, delete: &[Entity]) -> Result<(), WrongGeneration> {
        for &entity in delete {
            let id = entity.id() as usize;

            if !self.is_alive(entity) {
                return self.del_err(entity);
            }

            self.alive.remove(entity.id());
            // If the `Entity` was killed by `kill_atomic`, remove the bit set by it.
            self.killed.remove(entity.id());

            self.update_generation_length(id);

            if self.raised.remove(entity.id()) {
                self.generations[id].raise();
            }
            self.generations[id].die();
        }

        self.cache.extend(delete.iter().map(|e| e.0));

        Ok(())
    }

    /// Kills and entity atomically (will be updated when the allocator is
    /// maintained).
    pub fn kill_atomic(&self, e: Entity) -> Result<(), WrongGeneration> {
        if !self.is_alive(e) {
            return self.del_err(e);
        }

        self.killed.add_atomic(e.id());

        Ok(())
    }

    pub(crate) fn del_err(&self, e: Entity) -> Result<(), WrongGeneration> {
        Err(WrongGeneration {
            action: "delete",
            actual_gen: self.generations[e.id() as usize]
                .0
                .unwrap_or_else(Generation::one),
            entity: e,
        })
    }

    /// Return `true` if the entity is alive.
    pub fn is_alive(&self, e: Entity) -> bool {
        e.gen()
            == match self.generations.get(e.id() as usize) {
                Some(g) if !g.is_alive() && self.raised.contains(e.id()) => g.raised(),
                Some(g) => g.0.unwrap_or_else(Generation::one),
                None => Generation::one(),
            }
    }

    /// Returns the `Generation` of the given `Index`, if any.
    pub fn generation(&self, id: Index) -> Option<Generation> {
        self.generations
            .get(id as usize)
            .cloned()
            .and_then(|gen| gen.0)
    }

    /// Returns the current alive entity with the given `Index`.
    pub fn entity(&self, id: Index) -> Entity {
        let gen = match self.generations.get(id as usize) {
            Some(g) if !g.is_alive() && self.raised.contains(id) => g.raised(),
            Some(g) => g.0.unwrap_or_else(Generation::one),
            None => Generation::one(),
        };

        Entity(id, gen)
    }

    /// Allocate a new entity
    pub fn allocate_atomic(&self) -> Entity {
        let id = self.cache.pop_atomic().unwrap_or_else(|| {
            atomic_increment(&self.max_id).expect("No entity left to allocate") as Index
        });

        self.raised.add_atomic(id);
        let gen = self
            .generation(id)
            .map(|gen| if gen.is_alive() { gen } else { gen.raised() })
            .unwrap_or_else(Generation::one);
        Entity(id, gen)
    }

    /// Allocate a new entity
    pub fn allocate(&mut self) -> Entity {
        let id = self.cache.pop().unwrap_or_else(|| {
            let id = *self.max_id.get_mut();
            *self.max_id.get_mut() = id.checked_add(1).expect("No entity left to allocate");
            id as Index
        });

        self.update_generation_length(id as usize);

        self.alive.add(id as Index);

        let gen = self.generations[id as usize].raise();

        Entity(id as Index, gen)
    }

    /// Maintains the allocated entities, mainly dealing with atomically
    /// allocated or killed entities.
    pub fn merge(&mut self) -> Vec<Entity> {
        use hibitset::BitSetLike;

        let mut deleted = vec![];

        let max_id = *self.max_id.get_mut();
        self.update_generation_length(max_id + 1);

        for i in (&self.raised).iter() {
            self.generations[i as usize].raise();
            self.alive.add(i);
        }
        self.raised.clear();

        for i in (&self.killed).iter() {
            self.alive.remove(i);
            deleted.push(Entity(i, self.generations[i as usize].0.unwrap()));
            self.generations[i as usize].die();
        }
        self.killed.clear();

        self.cache.extend(deleted.iter().map(|e| e.0));

        deleted
    }

    fn update_generation_length(&mut self, i: usize) {
        if self.generations.len() <= i as usize {
            self.generations
                .resize(i as usize + 1, ZeroableGeneration(None));
        }
    }
}

/// An iterator for entity creation.
/// Please note that you have to consume
/// it because iterators are lazy.
///
/// Returned from `Entities::create_iter`.
pub struct CreateIterAtomic<'a>(&'a Allocator);

impl<'a> Iterator for CreateIterAtomic<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Entity> {
        Some(self.0.allocate_atomic())
    }
}

/// `Entity` type, as seen by the user.
#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Entity(Index, Generation);

impl Entity {
    /// Creates a new entity (externally from ECS).
    #[cfg(test)]
    pub fn new(index: Index, gen: Generation) -> Self {
        Self(index, gen)
    }

    /// Returns the index of the `Entity`.
    #[inline]
    pub fn id(self) -> Index {
        self.0
    }

    /// Returns the `Generation` of the `Entity`.
    #[inline]
    pub fn gen(self) -> Generation {
        self.1
    }
}

/// The entities of this ECS. This is a resource, stored in the `World`.
/// If you just want to access it in your system, you can also use the
/// `Entities` type def.
///
/// **Please note that you should never get
/// this mutably in a system, because it would
/// block all the other systems.**
///
/// You need to call `World::maintain` after creating / deleting
/// entities with this struct.
#[derive(Debug, Default)]
pub struct EntitiesRes {
    pub(crate) alloc: Allocator,
}

impl EntitiesRes {
    /// Creates a new entity atomically.
    /// This will be persistent as soon
    /// as you call `World::maintain`.
    ///
    /// If you want a lazy entity builder, take a look
    /// at `LazyUpdate::create_entity`.
    ///
    /// In case you have access to the `World`,
    /// you can also use `World::create_entity` which
    /// creates the entity and the components immediately.
    pub fn create(&self) -> Entity {
        self.alloc.allocate_atomic()
    }

    /// Returns an iterator which creates
    /// new entities atomically.
    /// They will be persistent as soon
    /// as you call `World::maintain`.
    pub fn create_iter(&self) -> CreateIterAtomic {
        CreateIterAtomic(&self.alloc)
    }

    /// Similar to the `create` method above this
    /// creates an entity atomically, and then returns a
    /// builder which can be used to insert components into
    /// various storages if available.
    pub fn build_entity(&self) -> EntityResBuilder {
        let entity = self.create();
        EntityResBuilder {
            entity,
            entities: self,
            built: false,
        }
    }

    /// Deletes an entity atomically.
    /// The associated components will be
    /// deleted as soon as you call `World::maintain`.
    pub fn delete(&self, e: Entity) -> Result<(), WrongGeneration> {
        self.alloc.kill_atomic(e)
    }

    /// Returns an entity with a given `id`. There's no guarantee for validity,
    /// meaning the entity could be not alive.
    pub fn entity(&self, id: Index) -> Entity {
        self.alloc.entity(id)
    }

    /// Returns `true` if the specified entity is alive.
    #[inline]
    pub fn is_alive(&self, e: Entity) -> bool {
        self.alloc.is_alive(e)
    }
}

impl<'a> Join for &'a EntitiesRes {
    type Mask = BitSetOr<&'a BitSet, &'a AtomicBitSet>;
    type Type = Entity;
    type Value = Self;

    unsafe fn open(self) -> (Self::Mask, Self) {
        (BitSetOr(&self.alloc.alive, &self.alloc.raised), self)
    }

    unsafe fn get(v: &mut &'a EntitiesRes, idx: Index) -> Entity {
        let gen = v
            .alloc
            .generation(idx)
            .map(|gen| if gen.is_alive() { gen } else { gen.raised() })
            .unwrap_or_else(Generation::one);
        Entity(idx, gen)
    }
}

#[cfg(feature = "parallel")]
unsafe impl<'a> ParJoin for &'a EntitiesRes {}

/// An entity builder from `EntitiesRes`.  Allows building an entity with its
/// components if you have mutable access to the component storages.
#[must_use = "Please call .build() on this to finish building it."]
pub struct EntityResBuilder<'a> {
    /// The entity being built
    pub entity: Entity,
    /// The active borrow to `EntitiesRes`, used to delete the entity if the
    /// builder is dropped without called `build()`.
    pub entities: &'a EntitiesRes,
    built: bool,
}

impl<'a> EntityResBuilder<'a> {
    /// Appends a component and associates it with the entity.
    pub fn with<T: Component>(self, c: T, storage: &mut WriteStorage<T>) -> Self {
        storage.insert(self.entity, c).unwrap();
        self
    }

    /// Finishes the building and returns the entity.
    pub fn build(mut self) -> Entity {
        self.built = true;
        self.entity
    }
}

impl<'a> Drop for EntityResBuilder<'a> {
    fn drop(&mut self) {
        if !self.built {
            self.entities.delete(self.entity).unwrap();
        }
    }
}

/// Index generation. When a new entity is placed at an old index,
/// it bumps the `Generation` by 1. This allows to avoid using components
/// from the entities that were deleted.
#[derive(Clone, Copy, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Generation(NonZeroI32);

// Show the inner value as i32 instead of u32.
impl fmt::Debug for Generation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("Generation").field(&self.id()).finish()
    }
}

impl Generation {
    pub(crate) fn one() -> Self {
        Generation(unsafe { NonZeroI32::new_unchecked(1) })
    }

    #[cfg(test)]
    pub fn new(v: i32) -> Self {
        Generation(NonZeroI32::new(v).expect("generation id must be non-zero"))
    }

    /// Returns the id of the generation.
    #[inline]
    pub fn id(self) -> i32 {
        self.0.get()
    }

    /// Returns `true` if entities of this `Generation` are alive.
    #[inline]
    pub fn is_alive(self) -> bool {
        self.id() > 0
    }

    /// Revives and increments a dead `Generation`.
    ///
    /// # Panics
    ///
    /// Panics if it is alive.
    fn raised(self) -> Generation {
        assert!(!self.is_alive());
        unsafe { Generation(NonZeroI32::new_unchecked(1 - self.id())) }
    }
}

/// Convenience wrapper around Option<Generation>
#[derive(Clone, Copy, Debug, Hash, Eq, PartialEq)]
struct ZeroableGeneration(Option<Generation>);

impl ZeroableGeneration {
    /// Returns the id of the generation.
    #[inline]
    pub fn id(self) -> i32 {
        // should optimise to a noop.
        self.0.map(|gen| gen.id()).unwrap_or(0)
    }

    /// Returns `true` if entities of this `Generation` are alive.
    #[inline]
    fn is_alive(self) -> bool {
        self.id() > 0
    }

    /// Kills this `Generation`.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if it's not alive.
    fn die(&mut self) {
        debug_assert!(self.is_alive());
        self.0 = NonZeroI32::new(-self.id()).map(Generation);
    }

    /// Revives and increments a dead `Generation`.
    ///
    /// # Panics
    ///
    /// Panics if it is alive.
    fn raised(self) -> Generation {
        assert!(!self.is_alive());
        let gen = 1i32.checked_sub(self.id()).expect("generation overflow");
        Generation(unsafe { NonZeroI32::new_unchecked(gen) })
    }

    /// Revives and increments a dead `ZeroableGeneration`.
    ///
    /// # Panics
    ///
    /// Panics if it is alive.
    fn raise(&mut self) -> Generation {
        let gen = self.raised();
        self.0 = Some(gen);
        gen
    }
}

#[derive(Default, Debug)]
struct EntityCache {
    cache: Vec<Index>,
    len: AtomicUsize,
}

impl EntityCache {
    fn pop_atomic(&self) -> Option<Index> {
        atomic_decrement(&self.len).map(|x| self.cache[x - 1])
    }

    fn pop(&mut self) -> Option<Index> {
        self.maintain();
        let x = self.cache.pop();
        *self.len.get_mut() = self.cache.len();
        x
    }

    fn maintain(&mut self) {
        self.cache.truncate(*(self.len.get_mut()));
    }
}

impl Extend<Index> for EntityCache {
    fn extend<T: IntoIterator<Item = Index>>(&mut self, iter: T) {
        self.maintain();
        self.cache.extend(iter);
        *self.len.get_mut() = self.cache.len();
    }
}

/// Increments `i` atomically without wrapping on overflow.
/// Resembles a `fetch_add(1, Ordering::Relaxed)` with
/// checked overflow, returning `None` instead.
fn atomic_increment(i: &AtomicUsize) -> Option<usize> {
    use std::usize;
    let mut prev = i.load(Ordering::Relaxed);
    while prev != usize::MAX {
        match i.compare_exchange_weak(prev, prev + 1, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(x) => return Some(x),
            Err(next_prev) => prev = next_prev,
        }
    }
    None
}

/// Increments `i` atomically without wrapping on overflow.
/// Resembles a `fetch_sub(1, Ordering::Relaxed)` with
/// checked underflow, returning `None` instead.
fn atomic_decrement(i: &AtomicUsize) -> Option<usize> {
    let mut prev = i.load(Ordering::Relaxed);
    while prev != 0 {
        match i.compare_exchange_weak(prev, prev - 1, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(x) => return Some(x),
            Err(next_prev) => prev = next_prev,
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nonzero_optimization() {
        use std::mem::size_of;
        assert_eq!(size_of::<Option<Generation>>(), size_of::<Generation>());
        assert_eq!(size_of::<Option<Entity>>(), size_of::<Entity>());
    }

    #[test]
    fn kill_atomic_create_merge() {
        let mut allocator = Allocator::default();

        let entity = allocator.allocate();
        assert_eq!(entity.id(), 0);

        allocator.kill_atomic(entity).unwrap();

        assert_ne!(allocator.allocate(), entity);

        assert_eq!(allocator.killed.contains(entity.id()), true);
        assert_eq!(allocator.merge(), vec![entity]);
    }

    #[test]
    fn kill_atomic_kill_now_create_merge() {
        let mut allocator = Allocator::default();

        let entity = allocator.allocate();

        allocator.kill_atomic(entity).unwrap();

        assert_ne!(allocator.allocate(), entity);

        allocator.kill(&[entity]).unwrap();

        allocator.allocate();

        assert_eq!(allocator.killed.contains(entity.id()), false);
        assert_eq!(allocator.merge(), vec![]);
    }
}
