
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use hibitset::BitSet;

use join::Join;
use storage::{DistinctStorage, MaskedStorage, Storage, UnprotectedStorage};
use world::{Component, EntityIndex};
use Index;

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

unsafe impl<'a, T, D> DistinctStorage for CheckStorage<'a, T, D> {}

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

impl<'e, T, D> Storage<'e, T, D>
    where T: Component,
          D: Deref<Target = MaskedStorage<T>>
{
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


impl<'e, T, D> Storage<'e, T, D>
    where T: Component,
          D: DerefMut<Target = MaskedStorage<T>>
{
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
}
