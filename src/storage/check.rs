
use std::ops::{Deref, Not};

use hibitset::BitSet;

use storage::{AntiStorage, MaskedStorage};
use {Component, DistinctStorage, Index, Join, Storage};

/// Allows iterating over a storage without borrowing the storage itself.
pub struct CheckStorage {
    bitset: BitSet,
}

impl<'a> Join for &'a CheckStorage {
    type Type = ();
    type Value = ();
    type Mask = &'a BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        (&self.bitset, ())
    }
    unsafe fn get(_: &mut (), _: Index) -> Self::Type {
        ()
    } 
}

impl<'a> Not for &'a CheckStorage {
    type Output = AntiStorage<'a>;
    fn not(self) -> Self::Output {
        AntiStorage(&self.bitset)
    }
}

unsafe impl DistinctStorage for CheckStorage {}

impl<'st, T, D> Storage<'st, T, D>
    where T: Component,
          D: Deref<Target = MaskedStorage<T>>,
{
    /// Builds a `CheckStorage` without borrowing the original storage.
    /// The bitset *can* be invalidated here if insertion or removal
    /// methods are used after the `CheckStorage` is created so there is
    /// no guarantee that the storage does have the component for a specific
    /// entity.
    pub fn check(&self) -> CheckStorage {
        CheckStorage {
            bitset: self.data.mask.clone(),
        }
    }
}
