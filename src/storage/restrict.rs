use std::{
    borrow::{Borrow, BorrowMut},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use hibitset::BitSet;
use shred::Fetch;

#[nougat::gat(Type)]
use crate::join::LendJoin;
use crate::join::{Join, RepeatableLendGet};

#[cfg(feature = "parallel")]
use crate::join::ParJoin;
use crate::{
    storage::{
        AccessMutReturn, DistinctStorage, MaskedStorage, SharedGetMutStorage, Storage,
        UnprotectedStorage,
    },
    world::{Component, EntitiesRes, Entity, Index},
};

/// Similar to a `MaskedStorage` and a `Storage` combined, but restricts usage
/// to only getting and modifying the components. That means it's not possible
/// to modify the inner bitset so the iteration cannot be invalidated. In other
/// words, no insertion or removal is allowed.
///
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
///     type SystemData = (Entities<'a>, WriteStorage<'a, SomeComp>);
///
///     fn run(&mut self, (entities, mut some_comps): Self::SystemData) {
///         for (entity, mut comps) in (&entities, &mut some_comps.restrict_mut()).join() {
///             // Check if the reference is fine to mutate.
///             if comps.get().0 < 5 {
///                 // Get a mutable reference now.
///                 let mut mutable = comps.get_mut();
///                 mutable.0 += 1;
///             }
///         }
///     }
/// }
/// ```
pub struct RestrictedStorage<'rf, 'st: 'rf, C, S> {
    bitset: &'rf BitSet,
    data: S,
    entities: &'rf Fetch<'st, EntitiesRes>,
    phantom: PhantomData<C>,
}

impl<'st, T, D> Storage<'st, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
{
    /// Builds an immutable `RestrictedStorage` out of a `Storage`. Allows
    /// deferred unchecked access to the entity's component.
    ///
    /// This is returned as a `ParallelRestriction` version since you can only
    /// get immutable components with this which is safe for parallel by
    /// default.
    pub fn restrict<'rf>(&'rf self) -> RestrictedStorage<'rf, 'st, T, &T::Storage> {
        RestrictedStorage {
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
    /// Builds a mutable `RestrictedStorage` out of a `Storage`. Allows
    /// restricted access to the inner components without allowing
    /// invalidating the bitset for iteration in `Join`.
    pub fn restrict_mut<'rf>(&'rf mut self) -> RestrictedStorage<'rf, 'st, T, &mut T::Storage> {
        let (mask, data) = self.data.open_mut();
        RestrictedStorage {
            bitset: mask,
            data,
            entities: &self.entities,
            phantom: PhantomData,
        }
    }
}

// SAFETY: `open` returns references to corresponding mask and storage values
// contained in the wrapped `Storage`. Iterating the mask does not repeat
// indices.
#[nougat::gat]
unsafe impl<'rf, 'st: 'rf, C, S> LendJoin for &'rf RestrictedStorage<'rf, 'st, C, S>
where
    C: Component,
    S: Borrow<C::Storage>,
{
    type Mask = &'rf BitSet;
    type Type<'next> = PairedStorageRead<'rf, 'st, C>;
    type Value = (&'rf C::Storage, &'rf Fetch<'st, EntitiesRes>, &'rf BitSet);

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        let bitset = self.bitset.borrow();
        (bitset, (self.data.borrow(), self.entities, bitset))
    }

    unsafe fn get<'next>(value: &'next mut Self::Value, id: Index) -> Self::Type<'next>
    where
        Self: 'next,
    {
        // NOTE: Methods on this type rely on safety requiments of this method.
        PairedStorageRead {
            index: id,
            storage: value.0,
            entities: value.1,
            bitset: value.2,
        }
    }
}

// SAFETY: LendJoin::get impl for this type can safely be called multiple times
// with the same ID.
unsafe impl<'rf, 'st: 'rf, C, S> RepeatableLendGet for &'rf RestrictedStorage<'rf, 'st, C, S>
where
    C: Component,
    S: Borrow<C::Storage>,
{
}

// SAFETY: `open` returns references to corresponding mask and storage values
// contained in the wrapped `Storage`. Iterating the mask does not repeat
// indices.
#[nougat::gat]
unsafe impl<'rf, 'st: 'rf, C, S> LendJoin for &'rf mut RestrictedStorage<'rf, 'st, C, S>
where
    C: Component,
    S: BorrowMut<C::Storage>,
{
    type Mask = &'rf BitSet;
    type Type<'next> = PairedStorageWriteExclusive<'next, 'st, C>;
    type Value = (
        &'rf mut C::Storage,
        &'rf Fetch<'st, EntitiesRes>,
        &'rf BitSet,
    );

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        let bitset = self.bitset.borrow();
        (bitset, (self.data.borrow_mut(), self.entities, bitset))
    }

    unsafe fn get<'next>(value: &'next mut Self::Value, id: Index) -> Self::Type<'next>
    where
        Self: 'next,
    {
        // NOTE: Methods on this type rely on safety requiments of this method.
        PairedStorageWriteExclusive {
            index: id,
            storage: value.0,
            entities: value.1,
            bitset: value.2,
        }
    }
}

// SAFETY: LendJoin::get impl for this type can safely be called multiple times
// with the same ID.
unsafe impl<'rf, 'st: 'rf, C, S> RepeatableLendGet for &'rf mut RestrictedStorage<'rf, 'st, C, S>
where
    C: Component,
    S: BorrowMut<C::Storage>,
{
}

// SAFETY: `open` returns references to corresponding mask and storage values
// contained in the wrapped `Storage`. Iterating the mask does not repeat
// indices.
unsafe impl<'rf, 'st: 'rf, C, S> Join for &'rf RestrictedStorage<'rf, 'st, C, S>
where
    C: Component,
    S: Borrow<C::Storage>,
{
    type Mask = &'rf BitSet;
    type Type = PairedStorageRead<'rf, 'st, C>;
    type Value = (&'rf C::Storage, &'rf Fetch<'st, EntitiesRes>, &'rf BitSet);

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        let bitset = self.bitset.borrow();
        (bitset, (self.data.borrow(), self.entities, bitset))
    }

    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        // NOTE: Methods on this type rely on safety requiments of this method.
        PairedStorageRead {
            index: id,
            storage: value.0,
            entities: value.1,
            bitset: value.2,
        }
    }
}

mod shared_get_only {
    use super::{DistinctStorage, Index, SharedGetMutStorage, UnprotectedStorage};
    use core::marker::PhantomData;

    /// This type provides a way to ensure only `shared_get_mut` and `get` can
    /// be called for the lifetime `'a` and that no references previously
    /// obtained from the storage exist when it is created. While internally
    /// this is a shared reference, constructing it requires an exclusive borrow
    /// for the lifetime `'a`.
    ///
    /// This is useful for implementation of [`Join`](super::Join) and
    /// [`ParJoin`](super::ParJoin) for `&mut RestrictedStorage`.
    pub struct SharedGetOnly<'a, T, S>(&'a S, PhantomData<T>);

    // SAFETY: All fields are required to be `Send` in the where clause. This
    // also requires `S: DistinctStorage` so that we can freely duplicate
    // `ShareGetOnly` while preventing `get_mut` from being called from multiple
    // threads at once.
    unsafe impl<'a, T, S> Send for SharedGetOnly<'a, T, S>
    where
        for<'b> &'b S: Send,
        PhantomData<T>: Send,
        S: DistinctStorage,
    {
    }
    // SAFETY: See above.
    // NOTE: A limitation of this is that `PairedStorageWrite` is not `Sync` in
    // some cases where it would be fine (we can address this if it is an issue).
    unsafe impl<'a, T, S> Sync for SharedGetOnly<'a, T, S>
    where
        for<'b> &'b S: Sync,
        PhantomData<T>: Sync,
        S: DistinctStorage,
    {
    }

    impl<'a, T, S> SharedGetOnly<'a, T, S> {
        pub(super) fn new(storage: &'a mut S) -> Self {
            Self(storage, PhantomData)
        }

        pub(crate) fn duplicate(this: &Self) -> Self {
            Self(this.0, this.1)
        }

        /// # Safety
        ///
        /// May only be called after a call to `insert` with `id` and no
        /// following call to `remove` with `id` or to `clean`.
        ///
        /// A mask should keep track of those states, and an `id` being
        /// contained in the tracking mask is sufficient to call this method.
        ///
        /// There must be no extant aliasing references to this component (i.e.
        /// obtained with the same `id` via this method or [`Self::get`]).
        pub(super) unsafe fn get_mut(
            this: &Self,
            id: Index,
        ) -> <S as UnprotectedStorage<T>>::AccessMut<'a>
        where
            S: SharedGetMutStorage<T>,
        {
            // SAFETY: `Self::new` takes an exclusive reference to this storage,
            // ensuring there are no extant references to its content at the
            // time `self` is created and ensuring that only `self` has access
            // to the storage for its lifetime and the lifetime of the produced
            // `AccessMutReturn`s (the reference we hold to the storage is not
            // exposed outside of this module).
            //
            // This means we only have to worry about aliasing references being
            // produced by calling `SharedGetMutStorage::shared_get_mut` (via
            // this method) or `UnprotectedStorage::get` (via `Self::get`).
            // Ensuring these don't alias is enforced by the requirements on
            // this method and `Self::get`.
            //
            // `Self` is only `Send`/`Sync` when `S: DistinctStorage`. Note,
            // that multiple instances of `Self` can be created via `duplicate`
            // but they can't be sent between threads (nor can shared references
            // be sent) unless `S: DistinctStorage`. These factors, along with
            // `Self::new` taking an exclusive reference to the storage, prevent
            // calling `shared_get_mut` from multiple threads at once unless `S:
            // DistinctStorage`.
            //
            // The remaining safety requirements are passed on to the caller.
            unsafe { this.0.shared_get_mut(id) }
        }

        /// # Safety
        ///
        /// May only be called after a call to `insert` with `id` and no
        /// following call to `remove` with `id` or to `clean`.
        ///
        /// A mask should keep track of those states, and an `id` being
        /// contained in the tracking mask is sufficient to call this method.
        ///
        /// There must be no extant references obtained from [`Self::get_mut`]
        /// using the same `id`.
        pub(super) unsafe fn get(this: &Self, id: Index) -> &'a T
        where
            S: UnprotectedStorage<T>,
        {
            // SAFETY: Safety requirements passed to the caller.
            unsafe { this.0.get(id) }
        }
    }
}
pub use shared_get_only::SharedGetOnly;

// SAFETY: `open` returns references to corresponding mask and storage values
// contained in the wrapped `Storage`. Iterating the mask does not repeat
// indices.
unsafe impl<'rf, 'st: 'rf, C, S> Join for &'rf mut RestrictedStorage<'rf, 'st, C, S>
where
    C: Component,
    S: BorrowMut<C::Storage>,
    C::Storage: SharedGetMutStorage<C>,
{
    type Mask = &'rf BitSet;
    type Type = PairedStorageWriteShared<'rf, C>;
    type Value = SharedGetOnly<'rf, C, C::Storage>;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        let bitset = &self.bitset;
        let storage = SharedGetOnly::new(self.data.borrow_mut());
        (bitset, storage)
    }

    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        // NOTE: Methods on this type rely on safety requiments of this method.
        PairedStorageWriteShared {
            index: id,
            storage: SharedGetOnly::duplicate(value),
        }
    }
}

// SAFETY: It is safe to call `get` from multiple threads at once since
// `T::Storage: Sync`. We construct a `PairedStorageRead` which can be used to
// call `UnprotectedStorage::get` which is safe to call concurrently.
//
// `open` returns references to corresponding mask and storage values contained
// in the wrapped `Storage`.
//
// Iterating the mask does not repeat indices.
unsafe impl<'rf, 'st: 'rf, C, S> ParJoin for &'rf RestrictedStorage<'rf, 'st, C, S>
where
    C: Component,
    S: Borrow<C::Storage>,
    C::Storage: Sync,
{
    type Mask = &'rf BitSet;
    type Type = PairedStorageRead<'rf, 'st, C>;
    type Value = (&'rf C::Storage, &'rf Fetch<'st, EntitiesRes>, &'rf BitSet);

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        let bitset = self.bitset.borrow();
        (bitset, (self.data.borrow(), self.entities, bitset))
    }

    unsafe fn get(value: &Self::Value, id: Index) -> Self::Type {
        // NOTE: Methods on this type rely on safety requiments of this method.
        PairedStorageRead {
            index: id,
            storage: value.0,
            entities: value.1,
            bitset: value.2,
        }
    }
}

// SAFETY: It is safe to call `get` from multiple threads at once since
// `T::Storage: Sync`. We construct a `PairedStorageSharedWrite` which can be
// used to call `UnprotectedStorage::get` which is safe to call concurrently and
// `SharedGetOnly::get_mut` which is safe to call concurrently since we require
// `C::Storage: DistinctStorage` here.
//
// `open` returns references to corresponding mask and storage values contained
// in the wrapped `Storage`.
//
// Iterating the mask does not repeat indices.
#[cfg(feature = "parallel")]
unsafe impl<'rf, 'st: 'rf, C, S> ParJoin for &'rf mut RestrictedStorage<'rf, 'st, C, S>
where
    C: Component,
    S: BorrowMut<C::Storage>,
    C::Storage: Sync + SharedGetMutStorage<C> + DistinctStorage,
{
    type Mask = &'rf BitSet;
    type Type = PairedStorageWriteShared<'rf, C>;
    type Value = SharedGetOnly<'rf, C, C::Storage>;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        let bitset = &self.bitset;
        let storage = SharedGetOnly::new(self.data.borrow_mut());
        (bitset, storage)
    }

    unsafe fn get(value: &Self::Value, id: Index) -> Self::Type {
        // NOTE: Methods on this type rely on safety requiments of this method.
        PairedStorageWriteShared {
            index: id,
            storage: SharedGetOnly::duplicate(value),
        }
    }
}

/// Pairs a storage with an index, meaning that the index is guaranteed to exist
/// as long as the `PairedStorage<C>` exists.
///
/// Yielded by `lend_join`/`join`/`par_join` on `&storage.restrict()`.
pub struct PairedStorageRead<'rf, 'st: 'rf, C: Component> {
    index: Index,
    storage: &'rf C::Storage,
    bitset: &'rf BitSet,
    entities: &'rf Fetch<'st, EntitiesRes>,
}

/// Pairs a storage with an index, meaning that the index is guaranteed to
/// exist.
///
/// Yielded by `join`/`par_join` on `&mut storage.restrict_mut()`.
pub struct PairedStorageWriteShared<'rf, C: Component> {
    index: Index,
    storage: SharedGetOnly<'rf, C, C::Storage>,
}

// SAFETY: All fields are required to implement `Send` in the where clauses. We
// also require `C::Storage: DistinctStorage` so that this cannot be sent
// between threads and then used to call `get_mut` from multiple threads at
// once.
unsafe impl<C> Send for PairedStorageWriteShared<'_, C>
where
    C: Component,
    Index: Send,
    for<'a> SharedGetOnly<'a, C, C::Storage>: Send,
    C::Storage: DistinctStorage,
{
}

/// Pairs a storage with an index, meaning that the index is guaranteed to
/// exist.
///
/// Yielded by `lend_join` on `&mut storage.restrict_mut()`.
pub struct PairedStorageWriteExclusive<'rf, 'st: 'rf, C: Component> {
    index: Index,
    storage: &'rf mut C::Storage,
    bitset: &'rf BitSet,
    entities: &'rf Fetch<'st, EntitiesRes>,
}

impl<'rf, 'st, C> PairedStorageRead<'rf, 'st, C>
where
    C: Component,
{
    /// Gets the component related to the current entity.
    ///
    /// Note, unlike `get_other` this doesn't need to check whether the
    /// component is present.
    pub fn get(&self) -> &C {
        // SAFETY: This is constructed in the `get` methods of
        // `LendJoin`/`Join`/`ParJoin` above. These all require that the mask
        // has been checked.
        unsafe { self.storage.get(self.index) }
    }

    /// Attempts to get the component related to an arbitrary entity.
    ///
    /// Functions similar to the normal `Storage::get` implementation.
    ///
    /// This only works for non-parallel or immutably parallel
    /// `RestrictedStorage`.
    pub fn get_other(&self, entity: Entity) -> Option<&C> {
        if self.bitset.contains(entity.id()) && self.entities.is_alive(entity) {
            // SAFETY:We just checked the mask.
            Some(unsafe { self.storage.get(entity.id()) })
        } else {
            None
        }
    }
}

impl<'rf, C> PairedStorageWriteShared<'rf, C>
where
    C: Component,
    C::Storage: SharedGetMutStorage<C>,
{
    /// Gets the component related to the current entity.
    pub fn get(&self) -> &C {
        // SAFETY: See note in `Self::get_mut` below. The only difference is
        // that here we take a shared reference which prevents `get_mut` from
        // being called while the return value is alive, but also allows this
        // method to still be called again (which is fine).
        unsafe { SharedGetOnly::get(&self.storage, self.index) }
    }

    /// Gets the component related to the current entity.
    pub fn get_mut(&mut self) -> AccessMutReturn<'_, C> {
        // SAFETY:
        // * This is constructed in the `get` methods of `Join`/`ParJoin` above.
        //   These all require that the mask has been checked.
        // * We also require that either there are no subsequent calls with the
        //   same `id` (`Join`) or that there are not extant references from a
        //   call with the same `id` (`ParJoin`). Thus, `id` is unique among
        //   the instances of `Self` created by the join `get` methods. We then
        //   tie the lifetime of the returned value to the exclusive borrow of
        //   self which prevents this or `Self::get` from being called while the
        //   returned reference is still alive.
        unsafe { SharedGetOnly::get_mut(&self.storage, self.index) }
    }
}

impl<'rf, 'st, C> PairedStorageWriteExclusive<'rf, 'st, C>
where
    C: Component,
{
    /// Gets the component related to the current entity.
    ///
    /// Note, unlike `get_other` this doesn't need to check whether the
    /// component is present.
    pub fn get(&self) -> &C {
        // SAFETY: This is constructed in `LendJoin::get` which requires that
        // the mask has been checked.
        unsafe { self.storage.get(self.index) }
    }

    /// Gets the component related to the current entity.
    ///
    /// Note, unlike `get_other_mut` this doesn't need to check whether the
    /// component is present.
    pub fn get_mut(&mut self) -> AccessMutReturn<'_, C> {
        // SAFETY: This is constructed in `LendJoin::get` which requires that
        // the mask has been checked.
        unsafe { self.storage.get_mut(self.index) }
    }

    /// Attempts to get the component related to an arbitrary entity.
    ///
    /// Functions similar to the normal `Storage::get` implementation.
    pub fn get_other(&self, entity: Entity) -> Option<&C> {
        if self.bitset.borrow().contains(entity.id()) && self.entities.is_alive(entity) {
            // SAFETY:We just checked the mask.
            Some(unsafe { self.storage.get(entity.id()) })
        } else {
            None
        }
    }

    /// Attempts to mutably get the component related to an arbitrary entity.
    ///
    /// Functions similar to the normal `Storage::get_mut` implementation.
    ///
    /// This only works if this is a lending `RestrictedStorage`, otherwise you
    /// could access the same component mutably via two different
    /// `PairedStorage`s at the same time.
    pub fn get_other_mut(&mut self, entity: Entity) -> Option<AccessMutReturn<'_, C>> {
        if self.bitset.contains(entity.id()) && self.entities.is_alive(entity) {
            // SAFETY:We just checked the mask.
            Some(unsafe { self.storage.get_mut(entity.id()) })
        } else {
            None
        }
    }
}
