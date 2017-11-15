
use std::ops::{Deref, DerefMut};

use hibitset::{BitSet, BitSetAnd, BitSetLike};

use storage::{MaskedStorage, Storage, WrappedStorage, UnprotectedStorage};

use {Component, Index, Join};

pub trait Metadata<T>: Default {
    fn clean<F>(&mut self, _: &F)
    where
        F: Fn(Index) -> bool { }
    fn get(&self, _: Index, _: &T) { }
    fn get_mut(&mut self, _: Index, _: &mut T) { }
    fn insert(&mut self, _: Index, _: &T) { }
    fn remove(&mut self, _: Index, _: &T) { }
}

pub trait Associate<T> {
    type Mask: BitSetLike;
    fn mask(self) -> Self::Mask;
}

pub trait AssociateMut<T> {
    type Mask: BitSetLike;
    fn mut_mask(self) -> Self::Mask;
}

pub struct Associated<'a, 'e: 'a, T, D, M, F>
where F: for<'f> Fn(&'f T::Metadata) -> &'f M,
      T: Component,
      D: 'a,
{
    pub(crate) storage: &'a Storage<'e, T, D>,
    pub(crate) pick: F,
}

pub struct AssociatedMut<'a, 'e: 'a, T, D, M, F>
where F: for<'f> Fn(&'f T::Metadata) -> &'f M,
      T: Component,
      D: 'a,
{
    pub(crate) storage: &'a mut Storage<'e, T, D>,
    pub(crate) pick: F,
}

/*
impl<'a, 'e, T, D, M, F> Join for Associated<'a, 'e, T, D, M, F>
    where T: Component,
          D: Deref<Target = MaskedStorage<T>>,
          F: Fn() -> &M{

}
*/

impl<T> Metadata<T> for () { }

impl<'a, 'b, 'e, T, D, M, F> Join for &'a Associated<'b, 'e, T, D, M, F>
    where T: Component,
          M: Metadata<T>,
          &'a M: Associate<T>,
          D: Deref<Target = MaskedStorage<T>>,
          F: for<'f> Fn(&'f T::Metadata) -> &'f M,
{
    type Type = &'a T;
    type Value = &'a WrappedStorage<T>;
    type Mask = BitSetAnd<&'a BitSet, <&'a M as Associate<T>>::Mask>;
    fn open(self) -> (Self::Mask, Self::Value) {
        let specific = (self.pick)(&self.storage.data.wrapped.meta);
        let storage_mask = &self.storage.data.mask;
        (BitSetAnd(storage_mask, specific.mask()), &self.storage.data.wrapped)
    }
    unsafe fn get(v: &mut Self::Value, id: Index) -> Self::Type {
        v.get(id)
    }
}

impl<'a, 'b, 'e, T, D, M, F> Join for &'a mut AssociatedMut<'b, 'e, T, D, M, F>
    where T: Component,
          M: Metadata<T>,
          &'a M: AssociateMut<T>,
          D: DerefMut<Target = MaskedStorage<T>>,
          F: for<'f> Fn(&'f T::Metadata) -> &'f M,
{
    type Type = &'a T;
    type Value = &'a WrappedStorage<T>;
    type Mask = BitSetAnd<&'a BitSet, <&'a M as AssociateMut<T>>::Mask>;
    fn open(self) -> (Self::Mask, Self::Value) {
        let specific = (self.pick)(&self.storage.data.wrapped.meta);
        let storage_mask = &self.storage.data.mask;
        (BitSetAnd(storage_mask, specific.mut_mask()), &self.storage.data.wrapped)
    }
    unsafe fn get(v: &mut Self::Value, id: Index) -> Self::Type {
        v.get(id)
    }
}

/*
impl Join for Associated {
    type Type = &'a T;
    type Value = (&'a Storage<'e, T, D>, &'a M);
    type Mask = BitSetAnd<&'a BitSet, M::Mask>;
    fn open(self) -> (Self::Mask, Self::Value) {
        let picked = self.pick(self.storage.meta());
        let mask = BitSetAnd(self.storage.data.mask, picked.mask());
        (mask, (self.storage, picked))
    }
    unsafe fn get((storage, metadata): &mut Self::Value, id: Index) -> Self::Type {
        
    }
}
*/
