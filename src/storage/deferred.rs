
use std::borrow::{Borrow, BorrowMut};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use hibitset::{BitSet, BitSetLike};

use {Component, Entities, Entity, Index, Join, ParJoin, Storage, UnprotectedStorage};
use storage::MaskedStorage;

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
/// Specifies that the `DeferredStorage` can run in parallel immutably.
/// 
/// This means that the storage can call `get`, `get_unchecked`.
pub enum ImmutableParallelRestriction {}

/// Restrictions that are allowed to access `DeferredStorage::get`.
pub trait ImmutableAliasing: Sized {}
impl ImmutableAliasing for SequentialRestriction { }
impl ImmutableAliasing for ImmutableParallelRestriction { }

/// Similar to a `MaskedStorage` and a `Storage` combined, but restricts usage
/// to only getting and modifying the components. That means nothing that would
/// Example Usage:
///
/// ```rust
/// # use specs::{Join, System, Component, DeferredStorage, WriteStorage, VecStorage, Entities};
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
///         for (entity, mut comps) in (
///             &*entities,
///             &mut some_comps.defer_mut()
///         ).join() {
///             // Check if the reference is fine to mutate.
///             if comps.get_deferred().0 < 5 {
///                 // Get a mutable reference now.
///                 let mut mutable = comps.get_mut_deferred();
///                 mutable.0 += 1;
///             }
///         }
///     }
/// }
/// ```
pub struct DeferredStorage<'rf, 'st: 'rf, C, S, B, Restrict>
where
    C: Component,
    S: Borrow<C::Storage> + 'rf,
    B: Borrow<BitSet> + 'rf,
{
    bitset: B,
    data: S,
    entities: &'rf Entities<'st>,
    phantom: PhantomData<(C, Restrict)>,
}

unsafe impl<'rf, 'st: 'rf, C, S, B> ParJoin
    for &'rf mut DeferredStorage<'rf, 'st, C, S, B, MutableParallelRestriction>
where
    C: Component,
    S: BorrowMut<C::Storage> + 'rf,
    B: Borrow<BitSet> + 'rf,
{
}

unsafe impl<'rf, 'st: 'rf, C, S, B, Restrict> ParJoin
    for &'rf DeferredStorage<'rf, 'st, C, S, B, Restrict>
where
    C: Component,
    S: Borrow<C::Storage> + 'rf,
    B: Borrow<BitSet> + 'rf,
    Restrict: ImmutableAliasing,
{
}

impl<'rf, 'st, C, S, B, Restrict> PairedStorage<'rf, 'st, C, S, B, Restrict>
where
    C: Component,
    S: Borrow<C::Storage>,
    B: Borrow<BitSet>,
{
    /// Gets the component related to the current entry without checking whether
    /// the storage has it or not.
    pub fn get_deferred(&self) -> &C {
        unsafe { self.storage.borrow().get(self.index) }
    }
}

impl<'rf, 'st, C, S, B, Restrict> PairedStorage<'rf, 'st, C, S, B, Restrict>
where
    C: Component,
    S: BorrowMut<C::Storage>,
    B: Borrow<BitSet>,
{
    /// Gets the component related to the current entry without checking whether
    /// the storage has it or not.
    pub fn get_mut_deferred(&mut self) -> &mut C {
        unsafe { self.storage.borrow_mut().get_mut(self.index) }
    }
}

impl<'rf, 'st, C, S, B, Restrict> PairedStorage<'rf, 'st, C, S, B, Restrict>
where
    C: Component,
    S: Borrow<C::Storage>,
    B: Borrow<BitSet>,
    // Only non parallel and immutable parallel storages can access this.
    Restrict: ImmutableAliasing,
{
    /// Attempts to get the component related to the entity.
    ///
    /// Functions similar to the normal `Storage::get` implementation.
    /// 
    /// This only works for non-parallel or immutably parallel `DeferredStorage`.
    pub fn get(&self, entity: Entity) -> Option<&C> {
        if self.bitset.borrow().contains(entity.id()) && self.entities.is_alive(entity) {
            Some(unsafe { self.storage.borrow().get(entity.id()) })
        } else {
            None
        }
    }
}

impl<'rf, 'st, C, S, B> PairedStorage<'rf, 'st, C, S, B, SequentialRestriction>
where
    C: Component,
    S: BorrowMut<C::Storage>,
    B: Borrow<BitSet>,
{
    /// Attempts to get the component related to the entity mutably.
    ///
    /// Functions similar to the normal `Storage::get_mut` implementation.
    ///
    /// This only works if this is a non-parallel `DeferredStorage`,
    /// otherwise you could access the same component mutably in two different threads.
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut C> {
        if self.bitset.borrow().contains(entity.id()) && self.entities.is_alive(entity) {
            Some(unsafe { self.storage.borrow_mut().get_mut(entity.id()) })
        } else {
            None
        }
    }
}

impl<'rf, 'st: 'rf, C, S, B, Restrict> Join for &'rf DeferredStorage<'rf, 'st, C, S, B, Restrict>
where
    C: Component,
    S: Borrow<C::Storage>,
    B: Borrow<BitSet>,
{
    type Type = PairedStorage<'rf, 'st, C, &'rf C::Storage, &'rf BitSet, Restrict>;
    type Value = (&'rf C::Storage, &'rf Entities<'st>, &'rf BitSet);
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

impl<'rf, 'st: 'rf, C, S, B, Restrict> Join for &'rf mut DeferredStorage<'rf, 'st, C, S, B, Restrict>
where
    C: Component,
    S: BorrowMut<C::Storage>,
    B: Borrow<BitSet>,
{
    type Type = PairedStorage<'rf, 'st, C, &'rf mut C::Storage, &'rf BitSet, Restrict>;
    type Value = (&'rf mut C::Storage, &'rf Entities<'st>, &'rf BitSet);
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
    ) -> DeferredStorage<'rf, 'st, T, &T::Storage, &BitSet, ImmutableParallelRestriction> {
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
    ) -> DeferredStorage<'rf, 'st, T, &mut T::Storage, &BitSet, SequentialRestriction> {
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
    ) -> DeferredStorage<'rf, 'st, T, &mut T::Storage, &BitSet, MutableParallelRestriction> {
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
    phantom: PhantomData<(C, Restrict)>,
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
