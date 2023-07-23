//! Component storage types, implementations for component joins, etc.

pub use self::deref_flagged::{DerefFlaggedStorage, FlaggedAccessMut};
pub use self::{
    data::{ReadStorage, WriteStorage},
    entry::{Entries, OccupiedEntry, StorageEntry, VacantEntry},
    flagged::FlaggedStorage,
    generic::{GenericReadStorage, GenericWriteStorage},
    restrict::{
        PairedStorageRead, PairedStorageWriteExclusive, PairedStorageWriteShared,
        RestrictedStorage, SharedGetOnly,
    },
    storages::{
        BTreeStorage, DefaultVecStorage, DenseVecStorage, HashMapStorage, NullStorage, SliceAccess,
        VecStorage,
    },
    track::{ComponentEvent, Tracked},
};

use std::{
    self,
    marker::PhantomData,
    ops::{Deref, DerefMut, Not},
};

use hibitset::{BitSet, BitSetLike, BitSetNot};
use shred::{CastFrom, Fetch};

#[nougat::gat(Type)]
use crate::join::LendJoin;
#[cfg(feature = "parallel")]
use crate::join::ParJoin;
use crate::{
    error::{Error, WrongGeneration},
    join::{Join, RepeatableLendGet},
    world::{Component, EntitiesRes, Entity, Index},
};

use self::drain::Drain;
use self::sync_unsafe_cell::SyncUnsafeCell;

mod data;
mod deref_flagged;
mod drain;
mod entry;
mod flagged;
mod generic;
mod restrict;
mod storages;
mod sync_unsafe_cell;
#[cfg(test)]
mod tests;
mod track;

type AccessMutReturn<'a, T> = <<T as Component>::Storage as UnprotectedStorage<T>>::AccessMut<'a>;

/// An inverted storage type, only useful to iterate entities
/// that do not have a particular component type.
pub struct AntiStorage<'a>(pub &'a BitSet);

// SAFETY: Items are just `()` and it is always safe to retrieve them regardless
// of the mask and value returned by `open`.
#[nougat::gat]
unsafe impl<'a> LendJoin for AntiStorage<'a> {
    type Mask = BitSetNot<&'a BitSet>;
    type Type<'next> = ();
    type Value = ();

    unsafe fn open(self) -> (Self::Mask, ()) {
        (BitSetNot(self.0), ())
    }

    unsafe fn get<'next>(_: &'next mut (), _: Index)
    where
        Self: 'next,
    {
    }
}

// SAFETY: <AntiStorage as LendJoin>::get does nothing.
unsafe impl RepeatableLendGet for AntiStorage<'_> {}

// SAFETY: Items are just `()` and it is always safe to retrieve them regardless
// of the mask and value returned by `open`.
unsafe impl<'a> Join for AntiStorage<'a> {
    type Mask = BitSetNot<&'a BitSet>;
    type Type = ();
    type Value = ();

    unsafe fn open(self) -> (Self::Mask, ()) {
        (BitSetNot(self.0), ())
    }

    unsafe fn get(_: &mut (), _: Index) {}
}

// SAFETY: Since `get` does not do anything it is safe to concurrently call.
// Items are just `()` and it is always safe to retrieve them regardless of the
// mask and value returned by `open`.
#[cfg(feature = "parallel")]
unsafe impl<'a> ParJoin for AntiStorage<'a> {
    type Mask = BitSetNot<&'a BitSet>;
    type Type = ();
    type Value = ();

    unsafe fn open(self) -> (Self::Mask, ()) {
        (BitSetNot(self.0), ())
    }

    unsafe fn get(_: &(), _: Index) {}
}

/// A dynamic storage.
pub trait AnyStorage {
    /// Drop components of given entities.
    fn drop(&mut self, entities: &[Entity]);
}

// SAFETY: Returned pointer has a vtable valid for `T` and retains the same
// address/provenance.
unsafe impl<T> CastFrom<T> for dyn AnyStorage
where
    T: AnyStorage + 'static,
{
    fn cast(t: *mut T) -> *mut Self {
        t
    }
}

impl<T> AnyStorage for MaskedStorage<T>
where
    T: Component,
{
    fn drop(&mut self, entities: &[Entity]) {
        for entity in entities {
            MaskedStorage::drop(self, entity.id());
        }
    }
}

/// This is a marker trait which requires you to uphold the following guarantee:
///
/// # Safety
///
/// > Multiple threads may call `SharedGetMutStorage::shared_get_mut()`
/// with distinct indices without causing > undefined behavior.
///
/// This is for example valid for `Vec`:
///
/// ```rust
/// vec![1, 2, 3];
/// ```
///
/// We may modify both element 1 and 2 at the same time.
///
/// As a counter example, we may have some kind of cached storage; it caches
/// elements when they're retrieved, so pushes a new element to some
/// cache-vector. This storage is not allowed to implement `DistinctStorage`.
///
/// Implementing this trait marks the storage safe for concurrent mutation (of
/// distinct elements), thus allows `par_join()`.
pub unsafe trait DistinctStorage {}

/// The status of an `insert()`ion into a storage.
/// If the insertion was successful then the Ok value will
/// contain the component that was replaced (if any).
pub type InsertResult<T> = Result<Option<T>, Error>;

/// The `UnprotectedStorage` together with the `BitSet` that knows
/// about which elements are stored, and which are not.
pub struct MaskedStorage<T: Component> {
    mask: BitSet,
    inner: T::Storage,
}

impl<T: Component> Default for MaskedStorage<T>
where
    T::Storage: Default,
{
    fn default() -> Self {
        Self {
            mask: Default::default(),
            inner: Default::default(),
        }
    }
}

impl<T: Component> MaskedStorage<T> {
    /// Creates a new `MaskedStorage`. This is called when you register
    /// a new component type within the world.
    pub fn new(inner: T::Storage) -> MaskedStorage<T> {
        MaskedStorage {
            mask: BitSet::new(),
            inner,
        }
    }

    fn open_mut(&mut self) -> (&BitSet, &mut T::Storage) {
        (&self.mask, &mut self.inner)
    }

    /// Clear the contents of this storage.
    pub fn clear(&mut self) {
        // NOTE: We replace with default empty mask temporarily to protect against
        // unwinding from `Drop` of components.
        let mut mask_temp = core::mem::take(&mut self.mask);
        // SAFETY: `self.mask` is the correct mask as specified. We swap in a
        // temporary empty mask to ensure if this unwinds that the mask will be
        // cleared.
        unsafe { self.inner.clean(&mask_temp) };
        mask_temp.clear();
        self.mask = mask_temp;
    }

    /// Remove an element by a given index.
    pub fn remove(&mut self, id: Index) -> Option<T> {
        if self.mask.remove(id) {
            // SAFETY: We checked the mask (`remove` returned `true`)
            Some(unsafe { self.inner.remove(id) })
        } else {
            None
        }
    }

    /// Drop an element by a given index.
    pub fn drop(&mut self, id: Index) {
        if self.mask.remove(id) {
            // SAFETY: We checked the mask and removed the id before calling
            // drop (`remove` returned `true`).
            unsafe {
                self.inner.drop(id);
            }
        }
    }
}

impl<T: Component> Drop for MaskedStorage<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

/// A wrapper around the masked storage and the generations vector.
/// Can be used for safe lookup of components, insertions and removes.
/// This is what `World::read/write` fetches for the user.
pub struct Storage<'e, T, D> {
    data: D,
    entities: Fetch<'e, EntitiesRes>,
    phantom: PhantomData<T>,
}

impl<'e, T, D> Storage<'e, T, D> {
    /// Creates a new `Storage` from a fetched allocator and a immutable or
    /// mutable `MaskedStorage`, named `data`.
    pub fn new(entities: Fetch<'e, EntitiesRes>, data: D) -> Storage<'e, T, D> {
        Storage {
            data,
            entities,
            phantom: PhantomData,
        }
    }
}

impl<'e, T, D> Storage<'e, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
{
    /// Gets the wrapped storage.
    pub fn unprotected_storage(&self) -> &T::Storage {
        &self.data.inner
    }

    /// Returns the `EntitiesRes` resource fetched by this storage.
    /// **This does not have anything to do with the components inside.**
    /// You only want to use this when implementing additional methods
    /// for `Storage` via an extension trait.
    pub fn fetched_entities(&self) -> &EntitiesRes {
        &self.entities
    }

    /// Tries to read the data associated with an `Entity`.
    pub fn get(&self, e: Entity) -> Option<&T> {
        if self.data.mask.contains(e.id()) && self.entities.is_alive(e) {
            // SAFETY: We checked the mask, so all invariants are met.
            Some(unsafe { self.data.inner.get(e.id()) })
        } else {
            None
        }
    }

    /// Computes the number of elements this `Storage` contains by counting the
    /// bits in the bit set. This operation will never be performed in
    /// constant time.
    pub fn count(&self) -> usize {
        self.mask().iter().count()
    }

    /// Checks whether this `Storage` is empty. This operation is very cheap.
    pub fn is_empty(&self) -> bool {
        self.mask().is_empty()
    }

    /// Returns true if the storage has a component for this entity, and that
    /// entity is alive.
    pub fn contains(&self, e: Entity) -> bool {
        self.data.mask.contains(e.id()) && self.entities.is_alive(e)
    }

    /// Returns a reference to the bitset of this storage which allows filtering
    /// by the component type without actually getting the component.
    pub fn mask(&self) -> &BitSet {
        &self.data.mask
    }
}

impl<'e, T, D> Storage<'e, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
    T::Storage: SliceAccess<T>,
{
    /// Returns the component data as a slice.
    ///
    /// The indices of this slice may not correspond to anything in particular.
    /// Check the underlying storage documentation for details.
    pub fn as_slice(&self) -> &[<T::Storage as SliceAccess<T>>::Element] {
        self.data.inner.as_slice()
    }
}

impl<'e, T, D> Storage<'e, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
    T::Storage: SliceAccess<T>,
{
    /// Returns the component data as a slice.
    ///
    /// The indices of this slice may not correspond to anything in particular.
    /// Check the underlying storage documentation for details.
    pub fn as_mut_slice(&mut self) -> &mut [<T::Storage as SliceAccess<T>>::Element] {
        self.data.inner.as_mut_slice()
    }
}

impl<'e, T, D> Storage<'e, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Gets mutable access to the wrapped storage.
    ///
    /// # Safety
    ///
    /// This is unsafe because modifying the wrapped storage without also
    /// updating the mask bitset accordingly can result in illegal memory
    /// access.
    pub unsafe fn unprotected_storage_mut(&mut self) -> &mut T::Storage {
        &mut self.data.inner
    }

    /// Tries to mutate the data associated with an `Entity`.
    pub fn get_mut(&mut self, e: Entity) -> Option<AccessMutReturn<'_, T>> {
        if self.data.mask.contains(e.id()) && self.entities.is_alive(e) {
            // SAFETY: We have exclusive access (which ensures no aliasing or
            // concurrent calls from other threads) and we checked the mask,
            // thus it's safe to call.
            Some(unsafe { self.data.inner.get_mut(e.id()) })
        } else {
            None
        }
    }

    /// Inserts new data for a given `Entity`.
    /// Returns the result of the operation as a `InsertResult<T>`
    ///
    /// If a component already existed for the given `Entity`, then it will
    /// be overwritten with the new component. If it did overwrite, then the
    /// result will contain `Some(T)` where `T` is the previous component.
    pub fn insert(&mut self, e: Entity, mut v: T) -> InsertResult<T> {
        if self.entities.is_alive(e) {
            let id = e.id();
            if self.data.mask.contains(id) {
                // SAFETY: `id` is in the mask.
                std::mem::swap(&mut v, unsafe { self.data.inner.get_mut(id) }.access_mut());
                Ok(Some(v))
            } else {
                // SAFETY: The mask was previously empty, so this is safe to
                // call.
                unsafe { self.not_present_insert(id, v) }
                Ok(None)
            }
        } else {
            Err(Error::WrongGeneration(WrongGeneration {
                action: "insert component for entity",
                actual_gen: self.entities.entity(e.id()).gen(),
                entity: e,
            }))
        }
    }

    /// Insert the provided value at `id` and adds `id` to the mask.
    ///
    /// # Safety
    ///
    /// May only be called if `id` is not present in the mask.
    #[inline(always)]
    unsafe fn not_present_insert(&mut self, id: Index, value: T) {
        // SAFETY: The mask was previously empty, so it is safe to
        // insert. We immediately add the value to the mask below and
        // unwinding from the `insert` call means that we don't need to
        // include the value in the mask. If adding to the mask unwinds we
        // abort.
        unsafe { self.data.inner.insert(id, value) };
        if cfg!(panic = "abort") {
            self.data.mask.add(id);
        } else {
            struct AbortOnDrop;
            impl Drop for AbortOnDrop {
                fn drop(&mut self) {
                    std::process::abort()
                }
            }
            let guard = AbortOnDrop;
            self.data.mask.add(id);
            core::mem::forget(guard);
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

    /// Creates a draining storage wrapper which can be `.join`ed
    /// to get a draining iterator.
    pub fn drain(&mut self) -> Drain<T> {
        Drain {
            data: &mut self.data,
        }
    }
}

impl<'a, T, D: Clone> Clone for Storage<'a, T, D> {
    fn clone(&self) -> Self {
        Storage::new(self.entities.clone(), self.data.clone())
    }
}

impl<'a, 'e, T, D> Not for &'a Storage<'e, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
{
    type Output = AntiStorage<'a>;

    fn not(self) -> Self::Output {
        AntiStorage(&self.data.mask)
    }
}

// SAFETY: The mask and unprotected storage contained in `MaskedStorage`
// correspond and `open` returns references to them from the same
// `MaskedStorage` instance. Iterating the mask does not repeat indices.
#[nougat::gat]
unsafe impl<'a, 'e, T, D> LendJoin for &'a Storage<'e, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
{
    type Mask = &'a BitSet;
    type Type<'next> = &'a T;
    type Value = &'a T::Storage;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        (&self.data.mask, &self.data.inner)
    }

    unsafe fn get<'next>(v: &'next mut Self::Value, i: Index) -> &'a T
    where
        Self: 'next,
    {
        // SAFETY: Since we require that the mask was checked, an element for
        // `i` must have been inserted without being removed.
        unsafe { v.get(i) }
    }
}

// SAFETY: LendJoin::get impl for this type is safe to call multiple times with
// the same ID.
unsafe impl<'a, 'e, T, D> RepeatableLendGet for &'a Storage<'e, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
{
}

// SAFETY: The mask and unprotected storage contained in `MaskedStorage`
// correspond and `open` returns references to them from the same
// `MaskedStorage` instance. Iterating the mask does not repeat indices.
unsafe impl<'a, 'e, T, D> Join for &'a Storage<'e, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
{
    type Mask = &'a BitSet;
    type Type = &'a T;
    type Value = &'a T::Storage;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        (&self.data.mask, &self.data.inner)
    }

    unsafe fn get(v: &mut Self::Value, i: Index) -> &'a T {
        // SAFETY: Since we require that the mask was checked, an element for
        // `i` must have been inserted without being removed.
        unsafe { v.get(i) }
    }
}

// SAFETY: It is safe to call `<T::Storage as UnprotectedStorage>::get` from
// multiple threads at once since `T::Storage: Sync`.
//
// The mask and unprotected storage contained in `MaskedStorage` correspond and
// `open` returns references to them from the same `MaskedStorage` instance.
// Iterating the mask does not repeat indices.
#[cfg(feature = "parallel")]
unsafe impl<'a, 'e, T, D> ParJoin for &'a Storage<'e, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
    T::Storage: Sync,
{
    type Mask = &'a BitSet;
    type Type = &'a T;
    type Value = &'a T::Storage;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        (&self.data.mask, &self.data.inner)
    }

    unsafe fn get(v: &Self::Value, i: Index) -> &'a T {
        // SAFETY: Since we require that the mask was checked, an element for
        // `i` must have been inserted without being removed.
        unsafe { v.get(i) }
    }
}

// SAFETY: The mask and unprotected storage contained in `MaskedStorage`
// correspond and `open` returns references to them from the same
// `MaskedStorage` instance. Iterating the mask does not repeat indices.
#[nougat::gat]
unsafe impl<'a, 'e, T, D> LendJoin for &'a mut Storage<'e, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    type Mask = &'a BitSet;
    type Type<'next> = AccessMutReturn<'next, T>;
    type Value = &'a mut T::Storage;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        self.data.open_mut()
    }

    unsafe fn get<'next>(value: &'next mut Self::Value, id: Index) -> Self::Type<'next>
    where
        Self: 'next,
    {
        // SAFETY: Since we require that the mask was checked, an element for
        // `id` must have been inserted without being removed.
        unsafe { value.get_mut(id) }
    }
}

// SAFETY: LendJoin::get impl for this type is safe to call multiple times with
// the same ID.
unsafe impl<'a, 'e, T, D> RepeatableLendGet for &'a mut Storage<'e, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
}

mod shared_get_mut_only {
    use super::{Index, SharedGetMutStorage, UnprotectedStorage};
    use core::marker::PhantomData;

    /// This type provides a way to ensure only `shared_get_mut` can be called
    /// for the lifetime `'a` and that no references previously obtained from
    /// the storage exist when it is created. While internally this is a shared
    /// reference, constructing it requires an exclusive borrow for the lifetime
    /// `'a`.
    ///
    /// This is useful for implementations of [`Join`](super::Join) and
    /// [`ParJoin`](super::ParJoin).
    pub struct SharedGetMutOnly<'a, T, S>(&'a S, PhantomData<T>);

    impl<'a, T, S> SharedGetMutOnly<'a, T, S> {
        pub(crate) fn new(storage: &'a mut S) -> Self {
            Self(storage, PhantomData)
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
        /// obtained with the same `id`).
        ///
        /// Unless `S: DistinctStorage`, calling this from multiple threads at
        /// once is unsound.
        pub(crate) unsafe fn get_mut(
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
            // produced by calling `SharedGetMutStorage::shared_get_mut`.
            // Ensuring these don't alias and the remaining safety requirements
            // are passed on to the caller.
            unsafe { this.0.shared_get_mut(id) }
        }
    }
}
pub use shared_get_mut_only::SharedGetMutOnly;

// SAFETY: The mask and unprotected storage contained in `MaskedStorage`
// correspond and `open` returns references to them from the same
// `MaskedStorage` instance (the storage is wrapped in `SharedGetMutOnly`).
// Iterating the mask does not repeat indices.
unsafe impl<'a, 'e, T, D> Join for &'a mut Storage<'e, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
    T::Storage: SharedGetMutStorage<T>,
{
    type Mask = &'a BitSet;
    type Type = AccessMutReturn<'a, T>;
    type Value = SharedGetMutOnly<'a, T, T::Storage>;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        let (mask, value) = self.data.open_mut();
        let value = SharedGetMutOnly::new(value);
        (mask, value)
    }

    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        // SAFETY:
        // * Since we require that the mask was checked, an element for `id` must have
        //   been inserted without being removed.
        // * We also require that there are no subsequent calls with the same `id` for
        //   this instance of the values from `open`, so there are no extant references
        //   for the element corresponding to this `id`.
        // * Since we have an exclusive reference to `Self::Value`, we know this isn't
        //   being called from multiple threads at once.
        unsafe { SharedGetMutOnly::get_mut(value, id) }
    }
}

// SAFETY: It is safe to call `SharedGetMutOnly<'a, T>::get_mut` from multiple
// threads at once since `T::Storage: DistinctStorage`.
//
// The mask and unprotected storage contained in `MaskedStorage` correspond and
// `open` returns references to them from the same `MaskedStorage` instance (the
// storage is wrapped in `SharedGetMutOnly`). Iterating the mask does not repeat
// indices.
#[cfg(feature = "parallel")]
unsafe impl<'a, 'e, T, D> ParJoin for &'a mut Storage<'e, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
    T::Storage: Sync + SharedGetMutStorage<T> + DistinctStorage,
{
    type Mask = &'a BitSet;
    type Type = AccessMutReturn<'a, T>;
    type Value = SharedGetMutOnly<'a, T, T::Storage>;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        let (mask, value) = self.data.open_mut();
        let value = SharedGetMutOnly::new(value);
        (mask, value)
    }

    unsafe fn get(value: &Self::Value, id: Index) -> Self::Type {
        // SAFETY:
        // * Since we require that the mask was checked, an element for `id` must have
        //   been inserted without being removed.
        // * We also require that the returned value is no longer alive before
        //   subsequent calls with the same `id`, so there are no extant references that
        //   were obtained with the same `id`.
        // * `T::Storage` implements the unsafe trait `DistinctStorage` so it is safe to
        //   call this from multiple threads at once.
        unsafe { SharedGetMutOnly::get_mut(value, id) }
    }
}

/// Tries to create a default value, returns an `Err` with the name of the
/// storage and/or component if there's no default.
pub trait TryDefault: Sized {
    /// Tries to create the default.
    fn try_default() -> Result<Self, String>;

    /// Calls `try_default` and panics on an error case.
    fn unwrap_default() -> Self {
        match Self::try_default() {
            Ok(x) => x,
            Err(e) => panic!("Failed to create a default value for storage ({:?})", e),
        }
    }
}

impl<T> TryDefault for T
where
    T: Default,
{
    fn try_default() -> Result<Self, String> {
        Ok(T::default())
    }
}

/// DerefMut without autoderefing.
///
/// Allows forcing mutable access to be explicit. Useful to implement a flagged
/// storage where it is easier to discover sites where components are marked as
/// mutated. Of course, individual storages can use an associated `AccessMut`
/// type that also implements `DerefMut`, but this provides the common
/// denominator.
pub trait AccessMut: core::ops::Deref {
    /// This may generate a mutation event for certain flagged storages.
    fn access_mut(&mut self) -> &mut Self::Target;
}

impl<T: ?Sized> AccessMut for T
where
    T: core::ops::DerefMut,
{
    fn access_mut(&mut self) -> &mut Self::Target {
        &mut *self
    }
}

/// Used by the framework to quickly join components.
pub trait UnprotectedStorage<T>: TryDefault {
    /// The wrapper through with mutable access of a component is performed.
    type AccessMut<'a>: AccessMut<Target = T>
    where
        Self: 'a;

    /// Clean the storage given a bitset with bits set for valid indices
    /// dropping all existing components.
    ///
    /// Allows us to drop the storage without leaking components.
    ///
    /// # Safety
    ///
    /// May only be called with the mask which keeps track of the elements
    /// existing in this storage.
    ///
    /// If this unwinds (e.g. due to a drop impl panicing), the mask should
    /// still be cleared.
    unsafe fn clean<B>(&mut self, has: B)
    where
        B: BitSetLike;

    /// Gets a shared reference to the data associated with an `Index`.
    ///
    /// This is unsafe because the external set used to protect this storage is
    /// absent.
    ///
    /// # Safety
    ///
    /// May only be called after a call to `insert` with `id` and
    /// no following call to `remove` with `id` or to `clean`.
    ///
    /// A mask should keep track of those states, and an `id` being contained
    /// in the tracking mask is sufficient to call this method.
    unsafe fn get(&self, id: Index) -> &T;

    /// Gets mutable access to the the data associated with an `Index`.
    ///
    /// This doesn't necessarily directly return a `&mut` reference. This
    /// allows storages more flexibility. For example, some flagged storages
    /// utilize this to defer generation of mutation events until the user
    /// obtains an `&mut` reference out of the returned wrapper type.
    ///
    /// This is unsafe because the external set used to protect this storage is
    /// absent.
    ///
    /// # Safety
    ///
    /// May only be called after a call to `insert` with `id` and no following
    /// call to `remove` with `id` or to `clean`.
    ///
    /// A mask should keep track of those states, and an `id` being contained in
    /// the tracking mask is sufficient to call this method.
    unsafe fn get_mut(&mut self, id: Index) -> Self::AccessMut<'_>;

    /// Inserts new data for a given `Index`.
    ///
    /// # Safety
    ///
    /// May only be called if `insert` was not called with `id` before, or
    /// was reverted by a call to `remove` with `id` or a call to `clean`.
    ///
    /// A mask should keep track of those states, and an `id` missing from the
    /// mask is sufficient to call `insert`.
    ///
    /// If this call unwinds the insertion should be considered to have failed
    /// and not be included in the mask or count as having called `insert` for
    /// the safety requirements of other methods here.
    unsafe fn insert(&mut self, id: Index, value: T);

    /// Removes the data associated with an `Index`.
    ///
    /// # Safety
    ///
    /// May only be called if an element with `id` was `insert`ed and not yet
    /// removed / dropped.
    unsafe fn remove(&mut self, id: Index) -> T;

    /// Drops the data associated with an `Index`.
    /// This could be used when a more efficient implementation for it exists
    /// than `remove` when the data is no longer needed.
    /// Defaults to simply calling `remove`.
    ///
    /// # Safety
    ///
    /// May only be called if an element with `id` was `insert`ed and not yet
    /// removed / dropped.
    ///
    /// Caller must ensure this is cleared from the mask even if the drop impl
    /// of the component panics and this unwinds. Usually, this can be
    /// accomplished by removing the id from the mask just before calling this.
    unsafe fn drop(&mut self, id: Index) {
        // SAFETY: Requirements passed to the caller.
        unsafe { self.remove(id) };
    }
}

/// Used by the framework to mutably access components in contexts where
/// exclusive access to the storage is not possible.
pub trait SharedGetMutStorage<T>: UnprotectedStorage<T> {
    /// Gets mutable access to the the data associated with an `Index`.
    ///
    /// This is unsafe because the external set used to protect this storage is
    /// absent and because it doesn't protect against concurrent calls from
    /// multiple threads and aliasing must manually be managed.
    ///
    /// # Safety
    ///
    /// May only be called after a call to `insert` with `id` and no following
    /// call to `remove` with `id` or to `clean`.
    ///
    /// A mask should keep track of those states, and an `id` being contained in
    /// the tracking mask is sufficient to call this method.
    ///
    /// There must be no extant aliasing references to this component (i.e.
    /// obtained with the same `id`). Additionally, references obtained from
    /// methods on this type that take `&self` (e.g. [`SliceAccess::as_slice`],
    /// [`Tracked::channel`]) must no longer be alive when
    /// `shared_get_mut` is called and these methods must not be
    /// called while the references returned here are alive. An exception is
    /// made for [`UnprotectedStorage::get`] as long as the live references it
    /// has returned do not alias with live references returned here.
    ///
    /// Essentially, the `unsafe` code calling this must hold exclusive access
    /// of the storage at some level to ensure only known code is calling
    /// `&self` methods during the usage of this method and the references it
    /// produces.
    ///
    /// Unless this type implements `DistinctStorage`, calling this from
    /// multiple threads at once is unsound.
    unsafe fn shared_get_mut(&self, id: Index) -> <Self as UnprotectedStorage<T>>::AccessMut<'_>;
}

#[cfg(test)]
#[cfg(feature = "parallel")]
mod tests_inline {

    use crate::{
        Builder, Component, DenseVecStorage, Entities, ParJoin, ReadStorage, World, WorldExt,
    };
    use rayon::iter::ParallelIterator;

    struct Pos;

    impl Component for Pos {
        type Storage = DenseVecStorage<Self>;
    }

    #[test]
    fn test_anti_par_join() {
        let mut world = World::new();
        world.create_entity().build();
        world.exec(|(entities, pos): (Entities, ReadStorage<Pos>)| {
            (&entities, !&pos).par_join().for_each(|(ent, ())| {
                println!("Processing entity: {:?}", ent);
            });
        });
    }
}
