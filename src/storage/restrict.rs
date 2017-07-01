
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::borrow::{Borrow, BorrowMut};
use std::fmt;

use shred::Fetch;
use hibitset::BitSet;

use storage::MaskedStorage;
use world::{EntityIndex, EntitiesRes};
use {Component, Entity, Index, Join, Storage, UnprotectedStorage};

/// Generic flags for the restricted storage to determine
/// the type of restriction.
pub mod restrict_type {
    pub struct Normal;
    pub struct Parallel;
}

/// Similar to a `MaskedStorage` and a `Storage` combined, but restricts usage
/// to only getting and modifying the components. That means nothing that would
/// modify the inner bitset so the iteration cannot be invalidated. For example,
/// no insertion or removal is allowed.
pub struct RestrictedStorage<'rf, 'st: 'rf, B, T, R, RT>
    where T: Component,
          R: Borrow<T::Storage> + 'rf,
          B: Borrow<BitSet> + 'rf,
{
    bitset: B,
    data: R,
    entities: &'rf Fetch<'st, EntitiesRes>,
    phantom: PhantomData<(T, RT)>,
}

impl<'rf, 'st, B, T, R, RT> RestrictedStorage<'rf, 'st, B, T, R, RT>
    where T: Component,
          R: Borrow<T::Storage>,
          B: Borrow<BitSet>,
{
    /// Attempts to get the component related to the entity.
    ///
    /// Functions similar to the normal `Storage::get` implementation.
    pub fn get(&self, entity: Entity) -> Option<&T> {
        if self.bitset.borrow().contains(entity.id()) && self.entities.is_alive(entity) {
            Some(unsafe { self.data.borrow().get(entity.id()) })
        } else {
            None
        }
    }

    /// Gets the component related to the current entry without checking whether
    /// the storage has it or not.
    pub fn get_unchecked(&self, entry: &Entry<'rf, T>) -> &T {
        entry.assert_same_storage(self.data.borrow());
        unsafe { self.data.borrow().get(entry.index()) }
    }
}

impl<'rf, 'st, B, T, R, RT> RestrictedStorage<'rf, 'st, B, T, R, RT>
    where T: Component,
          R: BorrowMut<T::Storage>,
          B: Borrow<BitSet>,
{
    /// Gets the component related to the current entry without checking whether
    /// the storage has it or not.
    pub fn get_mut_unchecked(&mut self, entry: &Entry<'rf, T>) -> &mut T {
        entry.assert_same_storage(self.data.borrow());
        unsafe { self.data.borrow_mut().get_mut(entry.index()) }
    }
}

impl<'rf, 'st, B, T, R> RestrictedStorage<'rf, 'st, B, T, R, restrict_type::Normal>
    where T: Component,
          R: BorrowMut<T::Storage>,
          B: Borrow<BitSet>,
{
    /// Attempts to get the component related to the entity mutably.
    ///
    /// Functions similar to the normal `Storage::get_mut` implementation.
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        if self.bitset.borrow().contains(entity.id()) && self.entities.is_alive(entity) {
            Some(unsafe { self.data.borrow_mut().get_mut(entity.id()) })
        } else {
            None
        }
    }

}

impl<'rf, 'st: 'rf, B, T, R, RT> Join for &'rf RestrictedStorage<'rf, 'st, B, T, R, RT>
    where T: Component,
          R: Borrow<T::Storage>,
          B: Borrow<BitSet>,
{
    type Type = (Entry<'rf, T>, Self); 
    type Value = Self;
    type Mask = &'rf BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        (self.bitset.borrow(), self)
    }
    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        let entry = Entry {
            id: id,
            pointer: value.data.borrow() as *const T::Storage,
            phantom: PhantomData,
        };
        
        (entry, value) // reference?
    }
}

impl<'rf, 'st: 'rf, B, T, R, RT> Join for &'rf mut RestrictedStorage<'rf, 'st, B, T, R, RT>
    where T: Component,
          R: BorrowMut<T::Storage>,
          B: Borrow<BitSet>,
{
    type Type = (Entry<'rf, T>, Self); 
    type Value = Self;
    type Mask = BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        (self.bitset.borrow().clone(), self)
    }
    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        use std::mem;
        let entry = Entry {
            id: id,
            pointer: value.data.borrow() as *const T::Storage,
            phantom: PhantomData,
        };

        let value: &'rf mut Self::Value = mem::transmute(value);
        (entry, value)
    }
}

impl<'st, T, D> Storage<'st, T, D>
    where T: Component,
          D: Deref<Target = MaskedStorage<T>>,
{
    /// Builds an immutable `RestrictedStorage` out of a `Storage`. Allows restricted
    /// access to the inner components without allowing invalidating the
    /// bitset for iteration in `Join`.
    pub fn restrict<'rf>(&'rf self)
        -> RestrictedStorage<'rf, 'st, &'rf BitSet, T, &'rf T::Storage, restrict_type::Normal>
    {
        RestrictedStorage {
            bitset: &self.data.mask,
            data: &self.data.inner,
            entities: &self.entities,
            phantom: PhantomData,
        }
    }

    /// Builds a parallel `RestrictedStorage`, does not allow mutably getting other components
    /// aside from the current iteration.
    pub fn par_restrict<'rf>(&'rf self)
        -> RestrictedStorage<'rf, 'st, &'rf BitSet, T, &'rf T::Storage, restrict_type::Parallel>
    {
        RestrictedStorage {
            bitset: &self.data.mask,
            data: &self.data.inner,
            entities: &self.entities,
            phantom: PhantomData,
        }
    }
}

impl<'st, T, D> Storage<'st, T, D>
    where T: Component,
          D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Builds a mutable `RestrictedStorage` out of a `Storage`. Allows restricted
    /// access to the inner components without allowing invalidating the
    /// bitset for iteration in `Join`.
    pub fn restrict_mut<'rf>(&'rf mut self)
        -> RestrictedStorage<'rf, 'st, &'rf BitSet, T, &'rf mut T::Storage, restrict_type::Normal>
    {
        let (mask, data) = self.data.open_mut();
        RestrictedStorage {
            bitset: mask,
            data: data,
            entities: &self.entities,
            phantom: PhantomData,
        }
    }

    /// Builds a mutable, parallel `RestrictedStorage`, does not allow mutably getting other components
    /// aside from the current iteration.
    pub fn par_restrict_mut<'rf>(&'rf mut self)
        -> RestrictedStorage<'rf, 'st, &'rf BitSet, T, &'rf mut T::Storage, restrict_type::Parallel>
    {
        let (mask, data) = self.data.open_mut();
        RestrictedStorage {
            bitset: mask,
            data: data,
            entities: &self.entities,
            phantom: PhantomData,
        }
    }
}

/// An entry to a storage.
pub struct Entry<'rf, T>
    where T: Component,
{
    id: Index,
    // Pointer for comparison when attempting to check against a storage.
    pointer: *const T::Storage,
    phantom: PhantomData<&'rf ()>,
}

impl<'rf, T> fmt::Debug for Entry<'rf, T>
    where T: Component
{
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "Entry {{ id: {}, pointer: {:?} }}", self.id, self.pointer)
    }
}

impl<'rf, T> Entry<'rf, T>
    where T: Component,
{
    #[inline]
    fn assert_same_storage(&self, storage: &T::Storage) {
        assert_eq!(self.pointer,
                   storage as *const T::Storage,
                   "Attempt to get an unchecked entry from a storage: {:?} {:?}",
                   self.pointer,
                   storage as *const T::Storage);
    }
}

impl<'rf, T> EntityIndex for Entry<'rf, T>
    where T: Component,
{
    fn index(&self) -> Index {
        self.id
    }
}

impl<'a, 'rf, T> EntityIndex for &'a Entry<'rf, T>
    where T: Component,
{
    fn index(&self) -> Index {
        (*self).index()
    }
}

