//! Storage types

pub use self::data::{ReadStorage, WriteStorage};
pub use self::flagged::FlaggedStorage;
pub use self::restrict::{Entry, SequentialRestriction, MutableParallelRestriction, ImmutableParallelRestriction, RestrictedStorage};
#[cfg(feature = "serde")]
pub use self::ser::{MergeError, PackedData};
pub use self::storages::{BTreeStorage, DenseVecStorage, HashMapStorage, NullStorage, VecStorage};
#[cfg(feature = "rudy")]
pub use self::storages::RudyStorage;
pub use self::tracked::{Change, ChangeEvents, TrackedStorage};

use std;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Not};

use hibitset::{BitSet, BitSetNot, BitSetLike};
use shred::Fetch;

use self::drain::Drain;
use {Component, EntitiesRes, Entity, Index, Join, ParJoin};
use error::WrongGeneration;

mod data;
mod drain;
mod restrict;
mod flagged;
#[cfg(feature = "serde")]
mod ser;
mod storages;
#[cfg(test)]
mod tests;
mod tracked;

/// An inverted storage type, only useful to iterate entities
/// that do not have a particular component type.
pub struct AntiStorage<'a>(&'a BitSet);

impl<'a> Join for AntiStorage<'a> {
    type Type = ();
    type Value = ();
    type Mask = BitSetNot<&'a BitSet>;

    fn open(self) -> (Self::Mask, ()) {
        (BitSetNot(self.0), ())
    }

    unsafe fn get(_: &mut (), _: Index) -> () {
        ()
    }
}

unsafe impl<'a> DistinctStorage for AntiStorage<'a> {}

/// A dynamic storage.
pub trait AnyStorage {
    /// Remove the component of an entity with a given index.
    fn drop(&mut self, id: Index);
}

impl<T> AnyStorage for MaskedStorage<T>
where
    T: Component,
{
    fn drop(&mut self, id: Index) {
        MaskedStorage::drop(self, id)
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
#[derive(Debug, PartialEq)]
pub enum InsertResult<T> {
    /// The value was inserted and there was no value before
    Inserted,
    /// The value was updated an already inserted value
    /// the value returned is the old value
    Updated(T),
    /// The value failed to insert because the entity
    /// was invalid
    EntityIsDead(T),
}

/// The `UnprotectedStorage` together with the `BitSet` that knows
/// about which elements are stored, and which are not.
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct MaskedStorage<T: Component> {
    mask: BitSet,
    inner: T::Storage,
}

impl<T: Component> MaskedStorage<T> {
    /// Creates a new `MaskedStorage`. This is called when you register
    /// a new component type within the world.
    pub fn new() -> MaskedStorage<T> {
        Default::default()
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
            unsafe { self.inner.drop(id); }
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
    /// Tries to read the data associated with an `Entity`.
    pub fn get(&self, e: Entity) -> Option<&T> {
        if self.data.mask.contains(e.id()) && self.entities.is_alive(e) {
            Some(unsafe { self.data.inner.get(e.id()) })
        } else {
            None
        }
    }

    /// Returns a copy of the `BitSet` of the storage. This allows you to
    /// do some methods on the actual storage without worrying about borrowing
    /// semantics.
    ///
    /// This bitset *can* be invalidated here if insertion or removal methods
    /// are used after the call to get the `BitSet`, so there is no guarantee
    /// that the storage will have a component for a specific entity.
    #[deprecated(since = "0.11", note = "Use `Storage::mask` and then clone the bitset it returns instead. This method hides a rather expensive operation which could be handled better in other ways.")]
    pub fn check(&self) -> BitSet {
        self.data.mask.clone()
    }

    /// Returns a reference to the bitset of this storage which allows filtering
    /// by the component type without actually getting the component.
    pub fn mask(&self) -> &BitSet {
        &self.data.mask
    }
}

/// An entry to a storage which has a component associated to the entity.
pub struct OccupiedEntry<'a, 'b: 'a, T: 'a, D: 'a> {
    entity: Entity,
    storage: &'a mut Storage<'b, T, D>,
}

impl<'a, 'b, T, D> OccupiedEntry<'a, 'b, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
{
    /// Get a reference to the component associated with the entity.
    pub fn get(&self) -> &T {
        unsafe { self.storage.data.inner.get(self.entity.id()) }
    }
}

impl<'a, 'b, T, D> OccupiedEntry<'a, 'b, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Get a mutable reference to the component associated with the entity.
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { self.storage.data.inner.get_mut(self.entity.id()) }
    }

    /// Converts the `OccupiedEntry` into a mutable reference bounded by
    /// the storage's lifetime.
    pub fn into_mut(self) -> &'a mut T {
        unsafe { self.storage.data.inner.get_mut(self.entity.id()) }
    }

    /// Inserts a value into the storage and returns the old one.
    pub fn insert(&mut self, mut component: T) -> T {
        std::mem::swap(&mut component, self.get_mut());
        component
    }

    /// Removes the component from the storage and returns it.
    pub fn remove(self) -> T {
        self.storage.data.remove(self.entity.id()).unwrap()
    }
}

/// An entry to a storage which does not have a component associated to the entity.
pub struct VacantEntry<'a, 'b: 'a, T: 'a, D: 'a> {
    entity: Entity,
    storage: &'a mut Storage<'b, T, D>,
}

impl<'a, 'b, T, D> VacantEntry<'a, 'b, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Inserts a value into the storage.
    pub fn insert(self, component: T) -> &'a mut T {
        let id = self.entity.id();
        self.storage.data.mask.add(id);
        unsafe {
            self.storage.data.inner.insert(id, component);
            self.storage.data.inner.get_mut(id)
        }
    }
}

/// Entry to a storage for convenient filling of components or removal based on whether
/// the entity has a component.
pub enum StorageEntry<'a, 'b: 'a, T: 'a, D: 'a> {
    /// Entry variant that is returned if the entity does has a component.
    Occupied(OccupiedEntry<'a, 'b, T, D>),
    /// Entry variant that is returned if the entity does not have a component.
    Vacant(VacantEntry<'a, 'b, T, D>),
}

impl<'a, 'b, T, D> StorageEntry<'a, 'b, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Inserts a component if the entity does not contain a component.
    pub fn or_insert(self, component: T) -> &'a mut T {
        self.or_insert_with(|| component)
    }

    /// Inserts a component using a lazily called function that is only called
    /// when inserting the component.
    pub fn or_insert_with<F>(self, default: F) -> &'a mut T
    where
        F: FnOnce() -> T,
    {
        match self {
            StorageEntry::Occupied(occupied) => occupied.into_mut(),
            StorageEntry::Vacant(vacant) => vacant.insert(default()),
        }
    }
}

impl<'e, T, D> Storage<'e, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Tries to mutate the data associated with an `Entity`.
    pub fn get_mut(&mut self, e: Entity) -> Option<&mut T> {
        if self.data.mask.contains(e.id()) && self.entities.is_alive(e) {
            Some(unsafe { self.data.inner.get_mut(e.id()) })
        } else {
            None
        }
    }

    /// Returns an entry to the component associated to the entity.
    ///
    /// Behaves somewhat similarly to `std::collections::HashMap`'s entry api.
    ///
    /// ## Example
    ///
    /// ```rust
    /// # extern crate specs;
    /// # struct Comp {
    /// #    field: u32
    /// # }
    /// # impl specs::Component for Comp {
    /// #    type Storage = specs::DenseVecStorage<Self>;
    /// # }
    /// # fn main() {
    /// # let mut world = specs::World::new();
    /// # world.register::<Comp>();
    /// # let entity = world.create_entity().build();
    /// # let mut storage = world.write::<Comp>();
    ///  if let Ok(entry) = storage.entry(entity) {
    ///      entry.or_insert(Comp { field: 55 });
    ///  }
    /// # }
    /// ```
    pub fn entry<'a>(&'a mut self, e: Entity) -> Result<StorageEntry<'a, 'e, T, D>, WrongGeneration>
    where
        'e: 'a,
    {
        if self.entities.is_alive(e) {
            if self.data.mask.contains(e.id()) {
                Ok(StorageEntry::Occupied(OccupiedEntry {
                    entity: e,
                    storage: self,
                }))
            } else {
                Ok(StorageEntry::Vacant(VacantEntry {
                    entity: e,
                    storage: self,
                }))
            }
        } else {
            let gen = self.entities
                .alloc
                .generations
                .get(e.id() as usize)
                .cloned()
                .unwrap_or(::Generation(1));
            Err(WrongGeneration {
                action: "attempting to get an entry to a storage",
                actual_gen: gen,
                entity: e,
            })
        }
    }

    /// Inserts new data for a given `Entity`.
    /// Returns the result of the operation as a `InsertResult<T>`
    pub fn insert(&mut self, e: Entity, mut v: T) -> InsertResult<T> {
        if self.entities.is_alive(e) {
            let id = e.id();
            if self.data.mask.contains(id) {
                std::mem::swap(&mut v, unsafe { self.data.inner.get_mut(id) });
                InsertResult::Updated(v)
            } else {
                self.data.mask.add(id);
                unsafe { self.data.inner.insert(id, v) };
                InsertResult::Inserted
            }
        } else {
            InsertResult::EntityIsDead(v)
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

unsafe impl<'a, T: Component, D> DistinctStorage for Storage<'a, T, D>
where
    T::Storage: DistinctStorage,
{
}

impl<'a, 'e, T, D> Join for &'a Storage<'e, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
{
    type Type = &'a T;
    type Value = &'a T::Storage;
    type Mask = &'a BitSet;

    fn open(self) -> (Self::Mask, Self::Value) {
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

unsafe impl<'a, 'e, T, D> ParJoin for &'a Storage<'e, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
    T::Storage: Sync,
{
}

impl<'a, 'e, T, D> Join for &'a mut Storage<'e, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    type Type = &'a mut T;
    type Value = &'a mut T::Storage;
    type Mask = &'a BitSet;

    fn open(self) -> (Self::Mask, Self::Value) {
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

unsafe impl<'a, 'e, T, D> ParJoin for &'a mut Storage<'e, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
    T::Storage: Sync + DistinctStorage,
{
}

/// Used by the framework to quickly join components.
pub trait UnprotectedStorage<T>: Default + Sized {
    /// Clean the storage given a check to figure out if an index
    /// is valid or not. Allows us to safely drop the storage.
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
