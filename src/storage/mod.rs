//! Component storage types, implementations for component joins, etc.

pub use self::data::{ReadStorage, WriteStorage};
pub use self::flagged::FlaggedStorage;
pub use self::generic::{GenericReadStorage, GenericWriteStorage};
pub use self::restrict::{
    ImmutableParallelRestriction, MutableParallelRestriction, RestrictedStorage,
    SequentialRestriction,
};
#[cfg(feature = "rudy")]
pub use self::storages::RudyStorage;
pub use self::storages::{BTreeStorage, DenseVecStorage, HashMapStorage, NullStorage, VecStorage};
pub use self::track::{ComponentEvent, Tracked};
pub use self::entry::{Entries, OccupiedEntry, VacantEntry, StorageEntry};

use std;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Not};

use hibitset::{BitSet, BitSetLike, BitSetNot};
use shred::{CastFrom, Fetch};

use self::drain::Drain;
use error::{Error, WrongGeneration};
use join::Join;
#[cfg(feature = "parallel")]
use join::ParJoin;
use world::{Component, EntitiesRes, Entity, Generation, Index};

mod data;
mod drain;
mod flagged;
mod generic;
mod restrict;
mod storages;
#[cfg(test)]
mod tests;
mod track;
mod entry;

/// An inverted storage type, only useful to iterate entities
/// that do not have a particular component type.
pub struct AntiStorage<'a>(&'a BitSet);

impl<'a> Join for AntiStorage<'a> {
    type Type = ();
    type Value = ();
    type Mask = BitSetNot<&'a BitSet>;

    unsafe fn open(self) -> (Self::Mask, ()) {
        (BitSetNot(self.0), ())
    }

    unsafe fn get(_: &mut (), _: Index) -> () {
        ()
    }
}

unsafe impl<'a> DistinctStorage for AntiStorage<'a> {}

#[cfg(feature = "parallel")]
unsafe impl<'a> ParJoin for AntiStorage<'a> {}

/// A dynamic storage.
pub trait AnyStorage {
    /// Drop components of given entities.
    fn drop(&mut self, entities: &[Entity]);
}

impl<T> CastFrom<T> for AnyStorage
where
    T: AnyStorage + 'static,
{
    fn cast(t: &T) -> &Self {
        t
    }

    fn cast_mut(t: &mut T) -> &mut Self {
        t
    }
}

impl<T> AnyStorage for MaskedStorage<T>
where
    T: Component,
{
    fn drop(&mut self, entities: &[Entity]) {
        for entity in entities {
            MaskedStorage::drop(self, entity.id());
        }
    }
}

/// This is a marker trait which requires you to uphold the following guarantee:
///
/// > Multiple threads may call `get_mut()` with distinct indices without causing
/// > undefined behavior.
///
/// This is for example valid for `Vec`:
///
/// ```rust
/// vec![1, 2, 3];
/// ```
///
/// We may modify both element 1 and 2 at the same time; indexing the vector mutably
/// does not modify anything else than the respective elements.
///
/// As a counter example, we may have some kind of cached storage; it caches
/// elements when they're retrieved, so pushes a new element to some cache-vector.
/// This storage is not allowed to implement `DistinctStorage`.
///
/// Implementing this trait marks the storage safe for concurrent mutation (of distinct
/// elements), thus allows `join_par()`.
pub unsafe trait DistinctStorage {}

/// The status of an `insert()`ion into a storage.
/// If the insertion was successful then the Ok value will
/// contain the component that was replaced (if any).
pub type InsertResult<T> = Result<Option<T>, Error>;

/// The `UnprotectedStorage` together with the `BitSet` that knows
/// about which elements are stored, and which are not.
#[derive(Derivative)]
#[derivative(Default(bound = "T::Storage: Default"))]
pub struct MaskedStorage<T: Component> {
    mask: BitSet,
    inner: T::Storage,
}

impl<T: Component> MaskedStorage<T> {
    /// Creates a new `MaskedStorage`. This is called when you register
    /// a new component type within the world.
    pub fn new(inner: T::Storage) -> MaskedStorage<T> {
        MaskedStorage {
            mask: BitSet::new(),
            inner,
        }
    }

    fn open_mut(&mut self) -> (&BitSet, &mut T::Storage) {
        (&self.mask, &mut self.inner)
    }

    /// Clear the contents of this storage.
    pub fn clear(&mut self) {
        unsafe {
            self.inner.clean(&self.mask);
        }
        self.mask.clear();
    }

    /// Remove an element by a given index.
    pub fn remove(&mut self, id: Index) -> Option<T> {
        if self.mask.remove(id) {
            Some(unsafe { self.inner.remove(id) })
        } else {
            None
        }
    }

    /// Drop an element by a given index.
    pub fn drop(&mut self, id: Index) {
        if self.mask.remove(id) {
            unsafe {
                self.inner.drop(id);
            }
        }
    }
}

impl<T: Component> Drop for MaskedStorage<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

/// A wrapper around the masked storage and the generations vector.
/// Can be used for safe lookup of components, insertions and removes.
/// This is what `World::read/write` fetches for the user.
pub struct Storage<'e, T, D> {
    data: D,
    entities: Fetch<'e, EntitiesRes>,
    phantom: PhantomData<T>,
}

impl<'e, T, D> Storage<'e, T, D> {
    /// Create a new `Storage`
    pub fn new(entities: Fetch<'e, EntitiesRes>, data: D) -> Storage<'e, T, D> {
        Storage {
            data,
            entities,
            phantom: PhantomData,
        }
    }
}

impl<'e, T, D> Storage<'e, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
{
    /// Gets the wrapped storage.
    pub fn unprotected_storage(&self) -> &T::Storage {
        &self.data.inner
    }

    /// Returns the `EntitesRes` resource fetched by this storage.
    /// **This does not have anything to do with the components inside.**
    /// You only want to use this when implementing additional methods
    /// for `Storage` via an extension trait.
    pub fn fetched_entities(&self) -> &EntitiesRes {
        &self.entities
    }

    /// Tries to read the data associated with an `Entity`.
    pub fn get(&self, e: Entity) -> Option<&T> {
        if self.data.mask.contains(e.id()) && self.entities.is_alive(e) {
            Some(unsafe { self.data.inner.get(e.id()) })
        } else {
            None
        }
    }

    /// Returns true if the storage has a component for this entity, and that entity is alive.
    pub fn contains(&self, e: Entity) -> bool {
        self.data.mask.contains(e.id()) && self.entities.is_alive(e)
    }

    /// Returns a reference to the bitset of this storage which allows filtering
    /// by the component type without actually getting the component.
    pub fn mask(&self) -> &BitSet {
        &self.data.mask
    }
}

impl<'e, T, D> Storage<'e, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Gets mutable access to the wrapped storage.
    ///
    /// This is unsafe because modifying the wrapped storage without also
    /// updating the mask bitset accordingly can result in illegal memory access.
    pub unsafe fn unprotected_storage_mut(&mut self) -> &mut T::Storage {
        &mut self.data.inner
    }

    /// Tries to mutate the data associated with an `Entity`.
    pub fn get_mut(&mut self, e: Entity) -> Option<&mut T> {
        if self.data.mask.contains(e.id()) && self.entities.is_alive(e) {
            Some(unsafe { self.data.inner.get_mut(e.id()) })
        } else {
            None
        }
    }

    /// Tries to retrieve `e` from the storage and passes a mutable reference to `and_then`.
    /// This is useful for accessing multiple components mutably.
    ///
    /// # Parameters
    ///
    /// * `entity`: The `Entity` you want to retrieve the component for. If it is not alive (was deleted)
    ///   or there is no component for `e` in this storage, `None` will be passed to `f`.
    /// * `f`: A closure executed independently of the existence of the component. It will
    ///   be passed `self` as a first argument, and an optional reference to the component (in
    ///   case it existed and is alive). The closure may return a value of type `R`, which will
    ///   be returned from this method.
    ///
    /// # How this works
    ///
    /// This implementation removes and re-inserts the component at `entity` to prove to the
    /// compiler the accessed components are distinct (or you will get a `None`).
    ///
    /// # Recommended usage
    ///
    /// Check entities for inequality prior to using this method; that way, if you get `None`, the
    /// error is limited to a missing component.
    ///
    /// # Examples
    ///
    /// ```
    /// use specs::prelude::*;
    ///
    /// pub enum FightResult {
    ///     Won,
    ///     Lost,
    ///     Undecided,
    /// }
    ///
    /// pub struct Robot;
    ///
    /// impl Robot {
    ///     pub fn fight(&mut self, _other: &mut Robot) -> FightResult {
    ///         // This one is actually pacifist
    ///         FightResult::Undecided
    ///     }
    /// }
    ///
    /// impl Component for Robot {
    ///     type Storage = HashMapStorage<Self>;
    /// }
    ///
    /// let mut world = World::new();
    ///
    /// world.register::<Robot>();
    ///
    /// let robot_entity1 = world
    ///     .create_entity()
    ///     .with(Robot)
    ///     .build();
    ///
    /// let robot_entity2 = world
    ///     .create_entity()
    ///     .with(Robot)
    ///     .build();
    ///
    /// let mut robots = world.write_storage::<Robot>(); // equivalent to `WriteStorage<Robot>`
    /// robots.with_mut(robot_entity1, |storage, robot| {
    ///     if let Some((robot1, robot2)) = robot
    ///         .and_then(|robot1| storage.get_mut(robot_entity2).map(|robot2| (robot1, robot2)))
    ///     {
    ///         let result = robot1.fight(robot2);
    ///     } else {
    ///         println!("Robots are identical or at least one is dead!");
    ///     }
    /// });
    /// ```
    ///
    /// Using this together with `.join()`:
    ///
    /// ```
    /// use specs::prelude::*;
    ///
    /// /// This is used to mark enemy robots.
    /// #[derive(Default)]
    /// pub struct Enemy;
    ///
    /// impl Component for Enemy {
    ///     type Storage = NullStorage<Self>;
    /// }
    ///
    /// pub enum FightResult {
    ///     Won,
    ///     Lost,
    ///     Undecided,
    /// }
    ///
    /// impl Component for FightResult {
    ///     type Storage = HashMapStorage<Self>;
    /// }
    ///
    /// pub struct Robot {
    ///     name: String,
    ///     level: i32,
    /// }
    ///
    /// impl Robot {
    ///     pub fn fight(&mut self, _other: &mut Robot) -> FightResult {
    ///         match self.name.as_ref() {
    ///             "Earl" => {
    ///                 // This one is actually pacifist
    ///                 FightResult::Undecided
    ///             }
    ///             _ => FightResult::Lost,
    ///         }
    ///     }
    /// }
    ///
    /// impl Component for Robot {
    ///     type Storage = HashMapStorage<Self>;
    /// }
    ///
    /// /// This system transforms a set of `Robot`s into a set of `FightResult`s.
    /// /// Each non-enemy robot will fight against each enemy robot.
    /// pub struct Fighting;
    ///
    /// impl<'a> System<'a> for Fighting {
    ///     type SystemData = (
    ///         Entities<'a>,
    ///         ReadStorage<'a, Enemy>,
    ///         WriteStorage<'a, FightResult>,
    ///         WriteStorage<'a, Robot>,
    ///     );
    ///
    ///     fn run(&mut self, (entities, enemies, mut results, mut robots): Self::SystemData) {
    ///         for (enemy_entity, _) in (&*entities, &enemies).join() {
    ///             // Start with a set of all robots...
    ///             let mut non_enemy_robots = robots.mask().clone();
    ///             // ...and remove all enemies by performing an OR
    ///             // with the negation of the enemies
    ///             non_enemy_robots |= &!enemies.mask();
    ///
    ///             for (robot_entity, _) in (&*entities, non_enemy_robots).join() {
    ///                 assert!(enemy_entity != robot_entity); // This should never be able to panic
    ///
    ///                 // Now, `enemy_entity` and `robot_entity` need to fight
    ///                 let fight_result = robots.with_mut(robot_entity, |robots, robot| {
    ///                     let enemy = robots.get_mut(enemy_entity);
    ///
    ///                     let robot = robot.expect("Joining ensured the component exists");
    ///                     let enemy = enemy.expect("Joining ensured the component exists");
    ///
    ///                     robot.fight(enemy)
    ///                 });
    ///
    ///                 // Let's store the results together with the fighting robot
    ///                 results.insert(robot_entity, fight_result).expect("Entity must be valid");
    ///             }
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// Panics if the component of `entity` gets inserted inside `f`.
    pub fn with_mut<F, R>(&mut self, entity: Entity, f: F) -> R
    where
        F: FnOnce(&mut Self, Option<&mut T>) -> R,
    {
        let mut component = self.remove(entity);

        let r = f(self, component.as_mut());

        if let Some(component) = component {
            assert!(
                self.insert(entity, component)
                    .expect("Unreachable: Entity has to be alive")
                    .is_none(),
                "Closure inserted a component for `entity`; please modify the component instead.",
            );
        }

        r
    }

    /// Inserts new data for a given `Entity`.
    /// Returns the result of the operation as a `InsertResult<T>`
    ///
    /// If a component already existed for the given `Entity`, then it will
    /// be overwritten with the new component. If it did overwrite, then the
    /// result will contain `Some(T)` where `T` is the previous component.
    pub fn insert(&mut self, e: Entity, mut v: T) -> InsertResult<T> {
        if self.entities.is_alive(e) {
            let id = e.id();
            if self.data.mask.contains(id) {
                std::mem::swap(&mut v, unsafe { self.data.inner.get_mut(id) });
                Ok(Some(v))
            } else {
                self.data.mask.add(id);
                unsafe { self.data.inner.insert(id, v) };
                Ok(None)
            }
        } else {
            Err(Error::WrongGeneration(WrongGeneration {
                action: "insert component for entity",
                actual_gen: self.entities.entity(e.id()).gen(),
                entity: e,
            }))
        }
    }

    /// Removes the data associated with an `Entity`.
    pub fn remove(&mut self, e: Entity) -> Option<T> {
        if self.entities.is_alive(e) {
            self.data.remove(e.id())
        } else {
            None
        }
    }

    /// Clears the contents of the storage.
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Creates a draining storage wrapper which can be `.join`ed
    /// to get a draining iterator.
    pub fn drain(&mut self) -> Drain<T> {
        Drain {
            data: &mut self.data,
        }
    }
}

unsafe impl<'a, T: Component, D> DistinctStorage for Storage<'a, T, D> where
    T::Storage: DistinctStorage
{}

impl<'a, 'e, T, D> Join for &'a Storage<'e, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
{
    type Type = &'a T;
    type Value = &'a T::Storage;
    type Mask = &'a BitSet;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        (&self.data.mask, &self.data.inner)
    }

    unsafe fn get(v: &mut Self::Value, i: Index) -> &'a T {
        v.get(i)
    }
}

impl<'a, 'e, T, D> Not for &'a Storage<'e, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
{
    type Output = AntiStorage<'a>;

    fn not(self) -> Self::Output {
        AntiStorage(&self.data.mask)
    }
}

#[cfg(feature = "parallel")]
unsafe impl<'a, 'e, T, D> ParJoin for &'a Storage<'e, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
    T::Storage: Sync,
{}

impl<'a, 'e, T, D> Join for &'a mut Storage<'e, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    type Type = &'a mut T;
    type Value = &'a mut T::Storage;
    type Mask = &'a BitSet;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        self.data.open_mut()
    }

    unsafe fn get(v: &mut Self::Value, i: Index) -> &'a mut T {
        // This is horribly unsafe. Unfortunately, Rust doesn't provide a way
        // to abstract mutable/immutable state at the moment, so we have to hack
        // our way through it.
        let value: *mut Self::Value = v as *mut Self::Value;
        (*value).get_mut(i)
    }
}

#[cfg(feature = "parallel")]
unsafe impl<'a, 'e, T, D> ParJoin for &'a mut Storage<'e, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
    T::Storage: Sync + DistinctStorage,
{}

/// Tries to create a default value, returns an `Err` with the name of the storage and/or component
/// if there's no default.
pub trait TryDefault: Sized {
    /// Tries to create the default.
    fn try_default() -> Result<Self, String>;

    /// Calls `try_default` and panics on an error case.
    fn unwrap_default() -> Self {
        match Self::try_default() {
            Ok(x) => x,
            Err(e) => panic!("Failed to create a default value for storage ({:?})", e),
        }
    }
}

impl<T> TryDefault for T
where
    T: Default,
{
    fn try_default() -> Result<Self, String> {
        Ok(T::default())
    }
}

/// Used by the framework to quickly join components.
pub trait UnprotectedStorage<T>: TryDefault {
    /// Clean the storage given a bitset with bits set for valid indices.
    /// Allows us to safely drop the storage.
    unsafe fn clean<B>(&mut self, has: B)
    where
        B: BitSetLike;

    /// Tries reading the data associated with an `Index`.
    /// This is unsafe because the external set used
    /// to protect this storage is absent.
    unsafe fn get(&self, id: Index) -> &T;

    /// Tries mutating the data associated with an `Index`.
    /// This is unsafe because the external set used
    /// to protect this storage is absent.
    unsafe fn get_mut(&mut self, id: Index) -> &mut T;

    /// Inserts new data for a given `Index`.
    unsafe fn insert(&mut self, id: Index, value: T);

    /// Removes the data associated with an `Index`.
    unsafe fn remove(&mut self, id: Index) -> T;

    /// Drops the data associated with an `Index`.
    unsafe fn drop(&mut self, id: Index) {
        self.remove(id);
    }
}

#[cfg(test)]
mod tests_inline {

    use rayon::iter::ParallelIterator;
    use {Builder, Component, DenseVecStorage, Entities, ParJoin, ReadStorage, World};

    struct Pos;

    impl Component for Pos {
        type Storage = DenseVecStorage<Self>;
    }

    #[test]
    fn test_anti_par_join() {
        let mut world = World::new();
        world.create_entity().build();
        world.exec(|(entities, pos): (Entities, ReadStorage<Pos>)| {
            (&entities, !&pos).par_join().for_each(|(ent, ())| {
                println!("Processing entity: {:?}", ent);
            });
        });
    }
}
