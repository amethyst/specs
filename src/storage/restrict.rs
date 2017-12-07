
use std::borrow::{Borrow, BorrowMut};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use hibitset::{BitSet, BitSetLike};

use join::{Join, ParJoin};
use storage::{MaskedStorage, Storage, UnprotectedStorage};
use world::{Component, Entities, Entity, EntityIndex, Index};

/// Specifies that the `DeferredStorage` cannot run in parallel.
///
/// A mutable `DeferredStorage` can call `get`, `get_mut`, `get_unchecked` and
/// `get_mut_unchecked` for deferred access while an immutable version can only
/// call the immutable accessors.
pub enum SequentialRestriction {}
/// Specifies that the `DeferredStorage` can run in parallel mutably.
///
/// This means the storage can only call `get_mut_unchecked` and `get_unchecked`.
pub enum MutableParallelRestriction {}
/// Specifies that the `RestrictedStorage` can run in parallel immutably.
///
/// This means that the storage can call `get`, `get_unchecked`.
pub enum ImmutableParallelRestriction {}

/// Restrictions that are allowed to access `DeferredStorage::get`.
pub trait ImmutableAliasing: Sized {}
impl ImmutableAliasing for SequentialRestriction {}
impl ImmutableAliasing for ImmutableParallelRestriction {}

/// Similar to a `MaskedStorage` and a `Storage` combined, but restricts usage
/// to only getting and modifying the components. That means nothing that would
/// Example Usage:
///
/// ```rust
/// # use specs::prelude::*;
/// struct SomeComp(u32);
/// impl Component for SomeComp {
///     type Storage = VecStorage<Self>;
/// }
///
/// struct RestrictedSystem;
/// impl<'a> System<'a> for RestrictedSystem {
///     type SystemData = (
///         Entities<'a>,
///         WriteStorage<'a, SomeComp>,
///     );
///     fn run(&mut self, (entities, mut some_comps): Self::SystemData) {
///         for (entity, (mut entry, restricted)) in (
///             &*entities,
///             &mut some_comps.restrict_mut()
///         ).join() {
///             // Check if the reference is fine to mutate.
///             if restricted.get_unchecked(&entry).0 < 5 {
///                 // Get a mutable reference now.
///                 let mut mutable = restricted.get_mut_unchecked(&mut entry);
///                 mutable.0 += 1;
///             }
///         }
///     }
/// }
/// ```
pub struct DeferredStorage<'rf, 'st: 'rf, B, T, R, RT>
where
    T: Component,
    R: Borrow<T::Storage> + 'rf,
    B: Borrow<BitSet> + 'rf,
{
    bitset: B,
    data: R,
    entities: &'rf Entities<'st>,
    phantom: PhantomData<(T, RT)>,
}

/*
unsafe impl<'rf, 'st: 'rf, B, T, R> ParJoin
    for &'rf mut DeferredStorage<'rf, 'st, B, T, R, MutableParallelRestriction>
where
    T: Component,
    R: BorrowMut<T::Storage> + 'rf,
    B: Borrow<BitSet> + 'rf,
{
}

unsafe impl<'rf, 'st: 'rf, B, T, R, RT> ParJoin for &'rf RestrictedStorage<'rf, 'st, B, T, R, RT>
where
    T: Component,
    R: Borrow<T::Storage> + 'rf,
    B: Borrow<BitSet> + 'rf,
    RT: ImmutableAliasing,
{
}

impl<'rf, 'st, B, T, R, RT> DeferredStorage<'rf, 'st, B, T, R, RT>
where
    T: Component,
    R: Borrow<T::Storage>,
    B: Borrow<BitSet>,
{
    /// Gets the component related to the current entry without checking whether
    /// the storage has it or not.
    pub fn get_unchecked(&self) -> &T {
        unsafe { self.data.borrow().get(entry.index()) }
    }
}
*/

/*
impl<'rf, 'st, B, T, R, RT> DeferredStorage<'rf, 'st, B, T, R, RT>
where
    T: Component,
    R: BorrowMut<T::Storage>,
    B: Borrow<BitSet>,
{
    /// Gets the component related to the current entry without checking whether
    /// the storage has it or not.
    pub fn get_mut_unchecked(&mut self) -> &mut T {
        entry.assert_same_storage(self.data.borrow());
        unsafe { self.data.borrow_mut().get_mut(entry.index()) }
    }
}

impl<'rf, 'st, B, T, R, RT> DeferredStorage<'rf, 'st, B, T, R, RT>
where
    T: Component,
    R: Borrow<T::Storage>,
    B: Borrow<BitSet>,
    // Only non parallel and immutable parallel storages can access this.
    RT: ImmutableAliasing,
{
    /// Attempts to get the component related to the entity.
    ///
    /// Functions similar to the normal `Storage::get` implementation.
    ///
    /// This only works for non-parallel or immutably parallel `RestrictedStorage`.
    pub fn get(&self, entity: Entity) -> Option<&T> {
        if self.bitset.borrow().contains(entity.id()) && self.entities.is_alive(entity) {
            Some(unsafe { self.data.borrow().get(entity.id()) })
        } else {
            None
        }
    }
}

impl<'rf, 'st, B, T, R> DeferredStorage<'rf, 'st, B, T, R, SequentialRestriction>
where
    T: Component,
    R: BorrowMut<T::Storage>,
    B: Borrow<BitSet>,
{
    /// Attempts to get the component related to the entity mutably.
    ///
    /// Functions similar to the normal `Storage::get_mut` implementation.
    ///
    /// This only works if this is a non-parallel `DeferredStorage`,
    /// otherwise you could access the same component mutably in two different threads.
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        if self.bitset.borrow().contains(entity.id()) && self.entities.is_alive(entity) {
            Some(unsafe { self.data.borrow_mut().get_mut(entity.id()) })
        } else {
            None
        }
    }
}
*/

impl<'rf, 'st: 'rf, B, T, R, RT> Join for &'rf DeferredStorage<'rf, 'st, B, T, R, RT>
where
    T: Component,
    R: Borrow<T::Storage>,
    B: Borrow<BitSet>,
{
    type Type = PairedStorage<'rf, 'st, T, &'rf T::Storage, &'rf BitSet>;
    type Value = (&'rf T::Storage, &'rf Entities<'st>, &'rf BitSet);
    type Mask = &'rf BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        let bitset = self.bitset.borrow();
        (bitset, (self.data.borrow(), self.entities, bitset))
    }
    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        PairedStorage {
            index: id,
            storage: value.0,
            entities: value.1,
            bitset: value.2,
            phantom: PhantomData,
        }
    }
}

impl<'rf, 'st: 'rf, B, T, R, RT> Join for &'rf mut DeferredStorage<'rf, 'st, B, T, R, RT>
where
    T: Component,
    R: BorrowMut<T::Storage>,
    B: Borrow<BitSet>,
{
    type Type = PairedStorage<'rf, 'st, T, &'rf mut T::Storage, &'rf BitSet>;
    type Value = (&'rf mut T::Storage, &'rf Entities<'st>, &'rf BitSet);
    type Mask = &'rf BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        let bitset = self.bitset.borrow();
        (bitset, (self.data.borrow_mut(), self.entities, bitset))
    }
    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        let value: &'rf mut Self::Value = &mut *(value as *mut Self::Value);
        PairedStorage {
            index: id,
            storage: value.0,
            entities: value.1,
            bitset: value.2,
            phantom: PhantomData,
        }
    }
}

impl<'st, T, D> Storage<'st, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
{
    /// Builds an immutable `DeferredStorage` out of a `Storage`. Allows deferred
    /// unchecked access to the entity's component.
    ///
    /// This is returned as a `ParallelRestriction` version since you can only get
    /// immutable components with this which is safe for parallel by default.
    pub fn defer<'rf>(
        &'rf self,
    ) -> DeferredStorage<'rf, 'st, &BitSet, T, &T::Storage, ImmutableParallelRestriction> {
        DeferredStorage {
            bitset: &self.data.mask,
            data: &self.data.inner,
            entities: &self.entities,
            phantom: PhantomData,
        }
    }
}

impl<'st, T, D> Storage<'st, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Builds a mutable `DeferredStorage` out of a `Storage`. Allows restricted
    /// access to the inner components without allowing invalidating the
    /// bitset for iteration in `Join`.
    pub fn defer_mut<'rf>(
        &'rf mut self,
    ) -> DeferredStorage<'rf, 'st, &BitSet, T, &mut T::Storage, SequentialRestriction> {
        let (mask, data) = self.data.open_mut();
        DeferredStorage {
            bitset: mask,
            data,
            entities: &self.entities,
            phantom: PhantomData,
        }
    }

    /// Builds a mutable, parallel `DeferredStorage`,
    /// does not allow mutably getting other components
    /// aside from the current iteration.
    pub fn par_defer_mut<'rf>(
        &'rf mut self,
    ) -> DeferredStorage<'rf, 'st, &BitSet, T, &mut T::Storage, MutableParallelRestriction> {
        let (mask, data) = self.data.open_mut();
        DeferredStorage {
            bitset: mask,
            data,
            entities: &self.entities,
            phantom: PhantomData,
        }
    }
}

/// Pairs a storage with an index, meaning that the index is guaranteed to exist
/// as long as the `PairedStorage<C, S>` exists.
///
/// This implements `Deref` and `DerefMut` to get the component.
pub struct PairedStorage<'rf, 'st: 'rf, C, S, B, Restrict> {
    index: Index,
    storage: S,
    bitset: B,
    entities: &'rf Entities<'st>,
    phantom: PhantomData<C>,
}

impl<'rf, 'st, C, S, B, Restrict> Deref for PairedStorage<'rf, 'st, C, S, B, Restrict>
where
    C: Component,
    S: Borrow<C::Storage>,
    B: BitSetLike,
{
    type Target = C;
    fn deref(&self) -> &Self::Target {
        // This should be enforced through the construction of `PairedStorage`.
        unsafe { self.storage.borrow().get(self.index) }
    }
}

impl<'rf, 'st, C, S, B, Restrict> DerefMut for PairedStorage<'rf, 'st, C, S, B, Restrict>
where
    C: Component,
    S: BorrowMut<C::Storage>,
    B: BitSetLike,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.storage.borrow_mut().get_mut(self.index) }
    }
}
