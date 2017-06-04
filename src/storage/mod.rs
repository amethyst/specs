//! Storage types

use std;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Not};

use hibitset::{BitSet, BitSetNot};
use mopa::Any;
use shred::{Fetch, FetchMut, ResourceId, Resources, SystemData};

use join::Join;
#[cfg(feature="parallel")]
use join::ParJoin;
use world::{Component, Entity, EntityIndex, Entities};
use Index;

#[cfg(feature="serialize")]
use serde;

pub mod storages;

#[cfg(test)]
mod tests;

/// A storage with read access.
pub type ReadStorage<'a, T> = Storage<'a, T, Fetch<'a, MaskedStorage<T>>>;

impl<'a, T> SystemData<'a> for ReadStorage<'a, T>
    where T: Component
{
    fn fetch(res: &'a Resources, id: usize) -> Self {
        Storage::new(res.fetch(0), res.fetch(id))
    }

    fn reads(id: usize) -> Vec<ResourceId> {
        vec![ResourceId::new::<Entities>(),
             ResourceId::new_with_id::<MaskedStorage<T>>(id)]
    }

    fn writes(_: usize) -> Vec<ResourceId> {
        vec![]
    }
}

/// A storage with read and write access.
pub type WriteStorage<'a, T> = Storage<'a, T, FetchMut<'a, MaskedStorage<T>>>;

impl<'a, T> SystemData<'a> for WriteStorage<'a, T>
    where T: Component
{
    fn fetch(res: &'a Resources, id: usize) -> Self {
        Storage::new(res.fetch(0), res.fetch_mut(id))
    }

    fn reads(_: usize) -> Vec<ResourceId> {
        vec![ResourceId::new::<Entities>()]
    }

    fn writes(id: usize) -> Vec<ResourceId> {
        vec![ResourceId::new_with_id::<MaskedStorage<T>>(id)]
    }
}

/// A dynamic storage.
pub trait AnyStorage {
    /// Remove the component of an entity with a given index.
    fn remove(&mut self, id: Index) -> Option<Box<Any>>;
}

impl<T> AnyStorage for MaskedStorage<T>
    where T: Component
{
    fn remove(&mut self, id: Index) -> Option<Box<Any>> {
        MaskedStorage::remove(self, id).map(|x| Box::new(x) as Box<Any>)
    }
}

/// The `UnprotectedStorage` together with the `BitSet` that knows
/// about which elements are stored, and which are not.
pub struct MaskedStorage<T: Component> {
    mask: BitSet,
    inner: T::Storage,
}

impl<T: Component> MaskedStorage<T> {
    /// Creates a new `MaskedStorage`. This is called when you register
    /// a new component type within the world.
    pub fn new() -> MaskedStorage<T> {
        MaskedStorage {
            mask: BitSet::new(),
            inner: UnprotectedStorage::new(),
        }
    }

    fn open_mut(&mut self) -> (&BitSet, &mut T::Storage) {
        (&self.mask, &mut self.inner)
    }

    /// Clear the contents of this storage.
    pub fn clear(&mut self) {
        let mask = &mut self.mask;
        unsafe {
            self.inner.clean(|i| mask.contains(i));
        }
        mask.clear();
    }

    /// Remove an element by a given index.
    pub fn remove(&mut self, id: Index) -> Option<T> {
        if self.mask.remove(id) {
            Some(unsafe { self.inner.remove(id) })
        } else {
            None
        }
    }
}

impl<T: Component> Drop for MaskedStorage<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

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

/// An entry to a storage.
pub struct Entry<'a, 'e, T, D> {
    id: Index,
    // Pointer for comparison when attempting to check against a storage.
    original: *const Storage<'e, T, D>,
    phantom: PhantomData<&'a ()>,
}

impl<'a, 'e, T, D> EntityIndex for Entry<'a, 'e, T, D> {
    fn index(&self) -> Index {
        self.id
    }
}

impl<'a, 'b, 'e, T, D> EntityIndex for &'b Entry<'a, 'e, T, D> {
    fn index(&self) -> Index {
        (*self).index()
    }
}

/// A storage type that iterates entities that have
/// a particular component type, but does not return the
/// component.
pub struct CheckStorage<'a, T, D> {
    bitset: BitSet,
    // Pointer back to the storage the CheckStorage was created from.
    original: *const Storage<'a, T, D>,
}

impl<'a, 'e, T, D> Join for &'a CheckStorage<'e, T, D> {
    type Type = Entry<'a, 'e, T, D>;
    type Value = *const Storage<'e, T, D>;
    type Mask = &'a BitSet;

    fn open(self) -> (Self::Mask, Self::Value) {
        (&self.bitset, self.original)
    }

    unsafe fn get(storage: &mut *const Storage<'e, T, D>, id: Index) -> Entry<'a, 'e, T, D> {
        Entry {
            id: id,
            original: *storage,
            phantom: PhantomData,
        }
    }
}

/// A wrapper around the masked storage and the generations vector.
/// Can be used for safe lookup of components, insertions and removes.
/// This is what `World::read/write` fetches for the user.
pub struct Storage<'e, T, D> {
    data: D,
    entities: Fetch<'e, Entities>,
    phantom: PhantomData<T>,
}

impl<'a, 'e, T, D> Not for &'a Storage<'e, T, D>
    where T: Component,
          D: Deref<Target = MaskedStorage<T>>
{
    type Output = AntiStorage<'a>;

    fn not(self) -> Self::Output {
        AntiStorage(&self.data.mask)
    }
}

impl<'e, T, D> Storage<'e, T, D> {
    /// Create a new `Storage`
    pub fn new(entities: Fetch<'e, Entities>, data: D) -> Storage<'e, T, D> {
        Storage {
            data: data,
            entities: entities,
            phantom: PhantomData,
        }
    }
}

impl<'e, T, D> Storage<'e, T, D>
    where T: Component,
          D: Deref<Target = MaskedStorage<T>>
{
    /// Tries to read the data associated with an `Entity`.
    pub fn get(&self, e: Entity) -> Option<&T> {
        if self.data.mask.contains(e.id()) && self.entities.is_alive(e) {
            Some(unsafe { self.data.inner.get(e.id()) })
        } else {
            None
        }
    }

    /// Returns a struct that can iterate over the entities that have it
    /// but does not return the contents of the storage.
    ///
    /// Useful if you want to check if an entity has a component
    /// and then possibly get the component later on in the loop.
    pub fn check(&self) -> CheckStorage<'e, T, D> {
        CheckStorage {
            bitset: self.data.mask.clone(),
            original: self as *const Storage<'e, T, D>,
        }
    }

    /// Reads the data associated with the entry.
    ///
    /// `Entry`s are returned from a `CheckStorage` to remove unnecessary checks.
    ///
    /// # Panics
    ///
    /// Panics if the entry was retrieved from another storage.
    pub fn get_unchecked<'a>(&'a self, entry: &'a Entry<'a, 'e, T, D>) -> &'a T {
        assert_eq!(entry.original,
                   self as *const Storage<'e, T, D>,
                   "Attempt to get an unchecked entry from a storage: {:?} {:?}",
                   entry.original,
                   self as *const Storage<'e, T, D>);
        unsafe { self.data.inner.get(entry.id) }
    }
}

/// The status of an insert operation
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

impl<'e, T, D> Storage<'e, T, D>
    where T: Component,
          D: DerefMut<Target = MaskedStorage<T>>
{
    /// Tries to mutate the data associated with an `Entity`.
    pub fn get_mut(&mut self, e: Entity) -> Option<&mut T> {
        if self.data.mask.contains(e.id()) && self.entities.is_alive(e) {
            Some(unsafe { self.data.inner.get_mut(e.id()) })
        } else {
            None
        }
    }

    /// Tries to mutate the data associated with an entry.
    ///
    /// `Entry`s are returned from a `CheckStorage` to remove unnecessary checks.
    pub fn get_mut_unchecked<'a>(&'a mut self, entry: &'a mut Entry<'a, 'e, T, D>) -> &'a mut T {
        assert_eq!(entry.original,
                   self as *const Storage<'e, T, D>,
                   "Attempt to get an unchecked entry from a storage: {:?} {:?}",
                   entry.original,
                   self as *const Storage<'e, T, D>);

        unsafe { self.data.inner.get_mut(entry.id) }
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
}

/// The error type returned
/// by [`Storage::merge`].
///
/// [`Storage::merge`]: struct.Storage.html#method.merge
#[cfg(feature="serialize")]
#[derive(Debug)]
pub enum MergeError {
    /// Returned if there is no
    /// entity matching the specified offset.
    NoEntity(Index),
}

#[cfg(feature="serialize")]
impl<'e, T, D> Storage<'e, T, D>
    where T: Component + serde::Deserialize<'e>,
          D: DerefMut<Target = MaskedStorage<T>>
{
    /// Merges a list of components into the storage.
    ///
    /// The list of entities will be used as the base for the offsets of the packed data.
    ///
    /// e.g.
    /// ```rust,ignore
    ///let list = vec![Entity(0, 1), Entity(1, 1), Entity(2, 1)];
    ///let packed = PackedData { offsets: [0, 2], components: [ ... ] };
    ///storage.merge(&list, packed);
    /// ```
    /// Would merge the components at offset 0 and 2, which would be `Entity(0, 1)` and
    /// `Entity(2, 1)` while ignoring
    /// `Entity(1, 1)`.
    ///
    /// Note:
    /// The entity list should be at least the same size as the packed data. To make sure,
    /// you can call `packed.pair_truncate(&entities)`.
    /// If the entity list is larger than the packed data then those entities are ignored.
    ///
    /// Packed data should also be sorted in ascending order of offsets.
    /// If this is deserialized from data received from serializing a storage it will be
    /// in ascending order.
    pub fn merge<'a>(&'a mut self,
                     entities: &'a [Entity],
                     mut packed: PackedData<T>)
                     -> Result<(), MergeError> {
        for (component, offset) in packed.components.drain(..).zip(packed.offsets.iter()) {
            match entities.get(*offset as usize) {
                Some(entity) => {
                    self.insert(*entity, component);
                }
                None => {
                    return Err(MergeError::NoEntity(*offset));
                }
            }
        }
        Ok(())
    }
}

impl<'a, 'e, T, D> Join for &'a Storage<'e, T, D>
    where T: Component,
          D: Deref<Target = MaskedStorage<T>>
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

// TODO: This implements it for all storages and doesn't make sure that distinct indices can be accessed in parallel.
#[cfg(feature="parallel")]
impl<'a, 'e, T, D> ParJoin for &'a Storage<'e, T, D>
    where T: Component,
          D: Deref<Target = MaskedStorage<T>>,
          T::Storage: Sync,
{}

impl<'a, 'e, T, D> Join for &'a mut Storage<'e, T, D>
    where T: Component,
          D: DerefMut<Target = MaskedStorage<T>>
{
    type Type = &'a mut T;
    type Value = &'a mut T::Storage;
    type Mask = &'a BitSet;

    fn open(self) -> (Self::Mask, Self::Value) {
        self.data.open_mut()
    }

    unsafe fn get(v: &mut Self::Value, i: Index) -> &'a mut T {
        use std::mem;

        // This is horribly unsafe. Unfortunately, Rust doesn't provide a way
        // to abstract mutable/immutable state at the moment, so we have to hack
        // our way through it.
        let value: &'a mut Self::Value = mem::transmute(v);
        value.get_mut(i)
    }
}

// TODO: This implements it for all storages and doesn't make sure that distinct indices can be accessed in parallel.
#[cfg(feature="parallel")]
impl<'a, 'e, T, D> ParJoin for &'a mut Storage<'e, T, D>
    where T: Component,
          D: DerefMut<Target = MaskedStorage<T>>,
          T::Storage: Sync,
{}

#[cfg(feature="serialize")]
impl<'e, T, D> serde::Serialize for Storage<'e, T, D>
    where T: Component + serde::Serialize,
          D: Deref<Target = MaskedStorage<T>>
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use hibitset::BitSetLike;
        use serde::ser::SerializeStruct;

        // Serializes the storage in a format of PackedData<T>
        let (bitset, storage) = self.open();
        // Serialize a struct that has 2 fields
        let mut structure = serializer.serialize_struct("PackedData", 2)?;
        let mut components: Vec<&T> = Vec::new();
        let mut offsets: Vec<u32> = Vec::new();
        for index in bitset.iter() {
            offsets.push(index);
            let component = unsafe { storage.get(index) };
            components.push(component);
        }

        structure.serialize_field("offsets", &offsets)?;
        structure.serialize_field("components", &components)?;
        structure.end()
    }
}

#[cfg(feature="serialize")]
#[derive(Debug, Serialize, Deserialize)]
/// Structure of packed components with offsets of which entities they belong to.
/// Offsets define which entities the components correspond to, based on a list of entities
/// the packed data is sent in with.
///
/// If the list of entities is all entities in the world, then the offsets in the
/// packed data are the indices of the entities.
pub struct PackedData<T> {
    /// List of components.
    pub components: Vec<T>,
    /// Offsets used to get entities which correspond to the components.
    pub offsets: Vec<Index>,
}

#[cfg(feature="serialize")]
impl<T> PackedData<T> {
    /// Modifies the data to match an entity list's length for merging.
    pub fn pair_truncate<'a>(&mut self, entities: &'a [Entity]) {
        self.truncate(entities.len());
    }
    /// Truncates the length of components and offsets.
    pub fn truncate(&mut self, length: usize) {
        self.components.truncate(length);
        self.offsets.truncate(length);
    }
}

/// Used by the framework to quickly join components.
pub trait UnprotectedStorage<T>: Sized {
    /// Creates a new `Storage<T>`. This is called when you register a new
    /// component type within the world.
    fn new() -> Self;

    /// Clean the storage given a check to figure out if an index
    /// is valid or not. Allows us to safely drop the storage.
    unsafe fn clean<F>(&mut self, f: F) where F: Fn(Index) -> bool;

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
}
