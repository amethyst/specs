
use std::ops::{Deref, DerefMut};

use hibitset::{BitSet, BitSetAnd, BitSetLike};

use storage::{MaskedStorage, Storage, WrappedStorage, UnprotectedStorage};

use Index;

/// Observes interactions with the component's storage and stores state alongside it.
///
/// Useful for things like tracking modifications to components, storing sorted lists related
/// to the storage, etc.
pub trait Metadata<T>: Default {
    fn clean<F>(&mut self, _: &F)
    where
        F: Fn(Index) -> bool { }
    fn get(&self, _: Index, _: &T) { }
    fn get_mut(&mut self, _: Index, _: &mut T) { }
    fn insert(&mut self, _: Index, _: &T) { }
    fn remove(&mut self, _: Index, _: &T) { }
}

/// Exposes "sub"-metadata by allowing metadata structs to forward the types
/// via this trait.
///
/// Useful for if you are using a tuple metadata and don't want to index each
/// sub-metadata using `metadata.0`, `metadata.1`, etc. as well as helping with
/// modelling generic APIs where something needs a specific metadata structure from
/// a component.
pub trait HasMeta<M> {
    /// Immutably gets the sub-metadata.
    fn find(&self) -> &M;
    /// Mutably gets the sub-metadata.
    fn find_mut(&mut self) -> &mut M;
}

impl<T> Metadata<T> for () { }

