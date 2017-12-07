
use std::borrow::{Borrow, BorrowMut};
use std::ops::{Deref, DerefMut};
use std::marker::PhantomData;

use hibitset::BitSet;

use {Component, Entities, Index, Join, MaskedStorage, Storage, UnprotectedStorage};

/// Defers calls to the underlying component storage until you use them.
pub struct DeferredStorage<'rf, 'st: 'rf, C, S, B>
where
    C: Component,
    S: Borrow<C::Storage> + 'rf,
    B: Borrow<BitSet> + 'rf,
{
    bitset: B,
    storage: S,
    entities: &'rf Entities<'st>,
    phantom: PhantomData<C>, 
}

impl<'st, T, D> Storage<'st, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
{
    /// Defers calls to `get` on the underlying component storage.
    ///
    /// Joining the returned structure will allow you to `Deref` into
    /// the component.
    pub fn defer<'rf>(&'rf self) -> DeferredStorage<'rf, 'st, T, &T::Storage, &BitSet> {
        DeferredStorage {
            bitset: &self.data.mask,
            storage: &self.data.inner,
            phantom: PhantomData,
        }
    }
}

impl<'st, T, D> Storage<'st, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Defers calls to `get` and `get_mut` on the underlying component storage.
    ///
    /// Joining the returned structure will allow you to `Deref` into
    /// the component.
    pub fn defer_mut<'rf>(&'rf mut self) -> DeferredStorage<'rf, 'st, T, &mut T::Storage, &BitSet> {
        let (mask, storage) = self.data.open_mut();
        DeferredStorage {
            bitset: mask,
            storage,
            phantom: PhantomData,
        }
    }
}


impl<'rf, C, S, B> Join for &'rf DeferredStorage<'rf, 'st, C, S, B>
where
    C: Component,
    S: Borrow<C::Storage>,
    B: Borrow<BitSet>,
{
    type Type = PairedStorage<'rf, 'st, C, &'rf C::Storage>;
    type Value = &'rf C::Storage;
    type Mask = &'rf BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        (self.bitset.borrow(), self.storage.borrow())
    }
    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        PairedStorage {
            index: id,
            storage: *value,
            phantom: PhantomData,    
        }
    }
}

impl<'rf, C, S, B> Join for &'rf mut DeferredStorage<C, S, B>
where
    C: Component,
    S: BorrowMut<C::Storage>,
    B: Borrow<BitSet>,
{
    type Type = PairedStorage<C, &'rf mut C::Storage>;
    type Value = &'rf mut C::Storage;
    type Mask = &'rf BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        (self.bitset.borrow(), self.storage.borrow_mut())
    }
    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        let value: &'rf mut Self::Value = &mut *(value as *mut Self::Value);
        PairedStorage {
            index: id,
            storage: *value,
            phantom: PhantomData,    
        }
    }
}
