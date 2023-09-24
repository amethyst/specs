//! Different types of storages you can use for your components.

use core::{marker::PhantomData, mem::MaybeUninit, ptr, ptr::NonNull};
use std::collections::BTreeMap;

use ahash::AHashMap as HashMap;
use hibitset::BitSetLike;

use crate::{
    storage::{DistinctStorage, SharedGetMutStorage, SyncUnsafeCell, UnprotectedStorage},
    world::Index,
};

/// Some storages can provide slices to access the underlying data.
///
/// The underlying data may be of type `T`, or it may be of a type
/// which wraps `T`. The associated type `Element` identifies what
/// the slices will contain.
pub trait SliceAccess<T> {
    /// The type of the underlying data elements.
    type Element;

    /// Returns a slice of the underlying storage.
    fn as_slice(&self) -> &[Self::Element];
    /// Returns a mutable slice of the underlying storage.
    fn as_mut_slice(&mut self) -> &mut [Self::Element];
}

/// BTreeMap-based storage.
pub struct BTreeStorage<T>(BTreeMap<Index, SyncUnsafeCell<T>>);

impl<T> Default for BTreeStorage<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> UnprotectedStorage<T> for BTreeStorage<T> {
    type AccessMut<'a> = &'a mut T where T: 'a;

    unsafe fn clean<B>(&mut self, _has: B)
    where
        B: BitSetLike,
    {
        self.0.clear();
    }

    unsafe fn get(&self, id: Index) -> &T {
        let ptr = self.0[&id].get();
        // SAFETY: See `VecStorage` impl.
        unsafe { &*ptr }
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        self.0.get_mut(&id).unwrap().get_mut()
    }

    unsafe fn insert(&mut self, id: Index, v: T) {
        self.0.insert(id, SyncUnsafeCell::new(v));
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        self.0.remove(&id).unwrap().0.into_inner()
    }
}

impl<T> SharedGetMutStorage<T> for BTreeStorage<T> {
    unsafe fn shared_get_mut(&self, id: Index) -> &mut T {
        let ptr = self.0[&id].get();
        // SAFETY: See `VecStorage` impl.
        unsafe { &mut *ptr }
    }
}

// SAFETY: `shared_get_mut` doesn't perform any overlapping mutable
// accesses when provided distinct indices and is safe to call from multiple
// threads at once.
unsafe impl<T> DistinctStorage for BTreeStorage<T> {}

/// `HashMap`-based storage. Best suited for rare components.
///
/// This uses the [std::collections::HashMap] internally.
pub struct HashMapStorage<T>(HashMap<Index, SyncUnsafeCell<T>>);

impl<T> Default for HashMapStorage<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> UnprotectedStorage<T> for HashMapStorage<T> {
    type AccessMut<'a> = &'a mut T where T: 'a;

    unsafe fn clean<B>(&mut self, _has: B)
    where
        B: BitSetLike,
    {
        self.0.clear();
    }

    unsafe fn get(&self, id: Index) -> &T {
        let ptr = self.0[&id].get();
        // SAFETY: See `VecStorage` impl.
        unsafe { &*ptr }
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        self.0.get_mut(&id).unwrap().get_mut()
    }

    unsafe fn insert(&mut self, id: Index, v: T) {
        self.0.insert(id, SyncUnsafeCell::new(v));
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        self.0.remove(&id).unwrap().0.into_inner()
    }
}

impl<T> SharedGetMutStorage<T> for HashMapStorage<T> {
    unsafe fn shared_get_mut(&self, id: Index) -> &mut T {
        let ptr = self.0[&id].get();
        // SAFETY: See `VecStorage` impl.
        unsafe { &mut *ptr }
    }
}

// SAFETY: `shared_get_mut` doesn't perform any overlapping mutable
// accesses when provided distinct indices and is safe to call from multiple
// threads at once.
unsafe impl<T> DistinctStorage for HashMapStorage<T> {}

/// Dense vector storage. Has a redirection 2-way table
/// between entities and components, allowing to leave
/// no gaps within the data.
///
/// Note that this only stores the data (`T`) densely; indices
/// to the data are stored in a sparse `Vec`.
///
/// `as_slice()` and `as_mut_slice()` indices are local to this
/// `DenseVecStorage` at this particular moment. These indices
/// cannot be compared with indices from any other storage, and
/// a particular entity's position within this slice may change
/// over time.
pub struct DenseVecStorage<T> {
    data: Vec<SyncUnsafeCell<T>>,
    entity_id: Vec<Index>,
    data_id: Vec<MaybeUninit<Index>>,
}

impl<T> Default for DenseVecStorage<T> {
    fn default() -> Self {
        Self {
            data: Default::default(),
            entity_id: Default::default(),
            data_id: Default::default(),
        }
    }
}

impl<T> SliceAccess<T> for DenseVecStorage<T> {
    type Element = T;

    /// Returns a slice of all the components in this storage.
    ///
    /// Indices inside the slice do not correspond to anything in particular,
    /// and especially do not correspond with entity IDs.
    #[inline]
    fn as_slice(&self) -> &[Self::Element] {
        let unsafe_cell_slice_ptr = SyncUnsafeCell::as_cell_of_slice(self.data.as_slice()).get();
        // SAFETY: See `VecStorage` impl.
        unsafe { &*unsafe_cell_slice_ptr }
    }

    /// Returns a mutable slice of all the components in this storage.
    ///
    /// Indices inside the slice do not correspond to anything in particular,
    /// and especially do not correspond with entity IDs.
    #[inline]
    fn as_mut_slice(&mut self) -> &mut [Self::Element] {
        SyncUnsafeCell::as_slice_mut(self.data.as_mut_slice())
    }
}

impl<T> UnprotectedStorage<T> for DenseVecStorage<T> {
    type AccessMut<'a> = &'a mut T where T: 'a;

    unsafe fn clean<B>(&mut self, _has: B)
    where
        B: BitSetLike,
    {
        // NOTE: clearing `data` may panic due to drop impls. So to makes sure
        // everything is cleared and ensure `remove` is sound we clear `data`
        // last.
        self.data_id.clear();
        self.entity_id.clear();
        self.data.clear();
    }

    unsafe fn get(&self, id: Index) -> &T {
        // NOTE: `as` cast is not lossy since insert would have encountered an
        // allocation failure if this would overflow `usize.`
        // SAFETY (get_unchecked and assume_init): Caller required to call
        // `insert` with this `id` (with no following call to `remove` with that
        // id or to `clean`).
        let did = unsafe { self.data_id.get_unchecked(id as usize).assume_init() };
        // SAFETY: Indices retrieved from `data_id` with a valid `id` will
        // always correspond to an element in `data`.
        let ptr = unsafe { self.data.get_unchecked(did as usize) }.get();
        // SAFETY: See `VecStorage` impl.
        unsafe { &*ptr }
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        // NOTE: `as` cast is not lossy since insert would have encountered an
        // allocation failure if this would overflow `usize.`
        // SAFETY (get_unchecked and assume_init): Caller required to call
        // `insert` with this `id` (with no following call to `remove` with that
        // id or to `clean`).
        let did = unsafe { self.data_id.get_unchecked(id as usize).assume_init() };
        // SAFETY: Indices retrieved from `data_id` with a valid `id` will
        // always correspond to an element in `data`.
        unsafe { self.data.get_unchecked_mut(did as usize) }.get_mut()
    }

    unsafe fn insert(&mut self, id: Index, v: T) {
        let id = if Index::BITS > usize::BITS {
            // Saturate the cast to usize::MAX so if this overflows usize the
            // allocation below will fail.
            core::cmp::min(id, usize::MAX as Index) as usize
        } else {
            id as usize
        };

        if self.data_id.len() <= id {
            // NOTE: saturating add ensures that if this computation would
            // overflow it will instead fail the allocation when calling
            // reserve.
            let delta = if Index::BITS >= usize::BITS {
                id.saturating_add(1)
            } else {
                id + 1
            } - self.data_id.len();
            self.data_id.reserve(delta);
            // NOTE: Allocation would have failed if this addition would overflow
            // SAFETY: MaybeUninit elements don't require initialization and
            // the reserve call ensures the capacity will be sufficient for this
            // new length.
            unsafe { self.data_id.set_len(id + 1) };
        }
        // NOTE: `as` cast here is not lossy since the length will be at most
        // `Index::MAX` if there is still an entity without this component.
        unsafe { self.data_id.get_unchecked_mut(id) }.write(self.data.len() as Index);
        // NOTE: `id` originally of the type `Index` so the cast back won't
        // overflow.
        self.entity_id.push(id as Index);
        self.data.push(SyncUnsafeCell::new(v));
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        // NOTE: cast to usize won't overflow since `insert` would have failed
        // to allocate.
        // SAFETY (get_unchecked and assume_init): Caller required to have
        // called `insert` with this `id`.
        let did = unsafe { self.data_id.get_unchecked(id as usize).assume_init() };
        let last = *self.entity_id.last().unwrap();
        // NOTE: cast to usize won't overflow since `insert` would have failed
        // to allocate.
        // SAFETY: indices in `self.entity_id` correspond to components present
        // in this storage so this will be in-bounds.
        unsafe { self.data_id.get_unchecked_mut(last as usize) }.write(did);
        // NOTE: casting the index in the dense data array to usize won't
        // overflow since the maximum number of components is limited to
        // `Index::MAX + 1`.
        self.entity_id.swap_remove(did as usize);
        self.data.swap_remove(did as usize).0.into_inner()
    }
}

impl<T> SharedGetMutStorage<T> for DenseVecStorage<T> {
    unsafe fn shared_get_mut(&self, id: Index) -> &mut T {
        // NOTE: `as` cast is not lossy since insert would have encountered an
        // allocation failure if this would overflow `usize.`
        // SAFETY (get_unchecked and assume_init): Caller required to call
        // `insert` with this `id` (with no following call to `remove` with that
        // id or to `clean`).
        let did = unsafe { self.data_id.get_unchecked(id as usize).assume_init() };
        // SAFETY: Indices retrieved from `data_id` with a valid `id` will
        // always correspond to an element in `data`.
        let ptr = unsafe { self.data.get_unchecked(did as usize) }.get();
        // SAFETY: See `VecStorage` impl.
        unsafe { &mut *ptr }
    }
}

// SAFETY: `shared_get_mut` doesn't perform any overlapping mutable
// accesses when provided distinct indices and is safe to call from multiple
// threads at once.
unsafe impl<T> DistinctStorage for DenseVecStorage<T> {}

/// A null storage type, used for cases where the component
/// doesn't contain any data and instead works as a simple flag.
pub struct NullStorage<T>(PhantomData<T>);

impl<T> Default for NullStorage<T> {
    fn default() -> Self {
        use core::mem::size_of;

        assert_eq!(size_of::<T>(), 0, "NullStorage can only be used with ZST");

        NullStorage(PhantomData)
    }
}

impl<T> UnprotectedStorage<T> for NullStorage<T> {
    type AccessMut<'a> = &'a mut T where T: 'a;

    unsafe fn clean<B>(&mut self, has: B)
    where
        B: BitSetLike,
    {
        for id in has.iter() {
            // SAFETY: Caller required to provide mask that keeps track of the
            // existing elements, so every `id` is valid to use with `remove`.
            unsafe { self.remove(id) };
        }
    }

    unsafe fn get(&self, _: Index) -> &T {
        // SAFETY: Because the caller is required by the safety docs to first
        // insert a component with this index, this corresponds to an instance
        // of the ZST we conceptually own. The caller also must manage the
        // aliasing of accesses via get/get_mut.
        //
        // Self::default asserts that `T` is a ZST which makes generating a
        // reference from a dangling pointer not UB.
        unsafe { &*NonNull::dangling().as_ptr() }
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        // SAFETY: Exclusive reference to `self` guarantees that that are no
        // extant references to components and that we aren't calling this from
        // multiple threads at once. Remaining requirements passed to caller.
        unsafe { self.shared_get_mut(id) }
    }

    unsafe fn insert(&mut self, _: Index, v: T) {
        // We rely on the caller tracking the presence of the ZST via the mask.
        //
        // We need to forget this to avoid the drop impl from running so the
        // storage logically is taking ownership of this instance of the ZST.
        core::mem::forget(v)
    }

    unsafe fn remove(&mut self, _: Index) -> T {
        // SAFETY: Because the caller is required by the safety docs to first
        // insert a component with this index, this corresponds to an instance
        // of the ZST we conceptually own.
        //
        // Self::default asserts that `T` is a ZST which makes reading from a
        // dangling pointer not UB.
        unsafe { ptr::read(NonNull::dangling().as_ptr()) }
    }
}

impl<T> SharedGetMutStorage<T> for NullStorage<T> {
    unsafe fn shared_get_mut(&self, _: Index) -> &mut T {
        // SAFETY: Because the caller is required by the safety docs to first
        // insert a component with this index, this corresponds to an instance
        // of the ZST we conceptually own. The caller also must manage the
        // aliasing of accesses via get/get_mut.
        //
        // Self::default asserts that `T` is a ZST which makes generating a
        // reference from a dangling pointer not UB.
        unsafe { &mut *NonNull::dangling().as_ptr() }
    }
}

// SAFETY: `shared_get_mut` doesn't perform any overlapping mutable
// accesses when provided distinct indices and is safe to call from multiple
// threads at once.
unsafe impl<T> DistinctStorage for NullStorage<T> {}

/// Vector storage. Uses a simple `Vec`. Supposed to have maximum
/// performance for the components mostly present in entities.
///
/// `as_slice()` and `as_mut_slice()` indices correspond to
/// entity IDs. These can be compared to other `VecStorage`s, to
/// other `DefaultVecStorage`s, and to `Entity::id()`s for live
/// entities.
pub struct VecStorage<T>(Vec<SyncUnsafeCell<MaybeUninit<T>>>);

impl<T> Default for VecStorage<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> SliceAccess<T> for VecStorage<T> {
    type Element = MaybeUninit<T>;

    #[inline]
    fn as_slice(&self) -> &[Self::Element] {
        let unsafe_cell_slice_ptr = SyncUnsafeCell::as_cell_of_slice(self.0.as_slice()).get();
        // SAFETY: The only place that mutably accesses these elements via a
        // shared reference is the impl of `SharedGetMut::shared_get_mut` which
        // requires callers to avoid calling other methods with `&self` while
        // references returned there are still in use (and to ensure references
        // from methods like this no longer exist).
        unsafe { &*unsafe_cell_slice_ptr }
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [Self::Element] {
        SyncUnsafeCell::as_slice_mut(self.0.as_mut_slice())
    }
}

impl<T> UnprotectedStorage<T> for VecStorage<T> {
    type AccessMut<'a> = &'a mut T where T: 'a;

    unsafe fn clean<B>(&mut self, has: B)
    where
        B: BitSetLike,
    {
        for (i, v) in self.0.iter_mut().enumerate() {
            // NOTE: `as` cast is safe since the index used for insertion is a
            // `u32` so the indices will never be over `u32::MAX`.
            const _: Index = 0u32;
            if has.contains(i as u32) {
                // drop in place
                let v_inner = v.get_mut();
                // SAFETY: Present in the provided mask. All components are
                // considered removed after a call to `clean`.
                unsafe { v_inner.assume_init_drop() };
            }
        }
    }

    unsafe fn get(&self, id: Index) -> &T {
        // NOTE: `as` cast is not lossy since insert would have encountered an
        // allocation failure if this would overflow `usize.`
        // SAFETY: Caller required to call `insert` with this `id` (with no
        // following call to `remove` with that id or to `clean`).
        let ptr = unsafe { self.0.get_unchecked(id as usize) }.get();
        // SAFETY: Only method that obtains exclusive references from this
        // unsafe cell is `shared_get_mut` and callers of that method are
        // required to manually ensure that those references don't alias
        // references from this method.
        let maybe_uninit = unsafe { &*ptr };
        // SAFETY: Requirement to have `insert`ed this component ensures that it
        // will be initialized.
        unsafe { maybe_uninit.assume_init_ref() }
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        // NOTE: `as` cast is not lossy since `insert` would have encountered an
        // allocation failure if this would overflow `usize.`
        // SAFETY: Caller required to call `insert` with this `id` (with no
        // following call to `remove` with that id or to `clean`).
        let maybe_uninit = unsafe { self.0.get_unchecked_mut(id as usize) }.get_mut();
        // SAFETY: Requirement to have `insert`ed this component ensures that it
        // will be initialized.
        unsafe { maybe_uninit.assume_init_mut() }
    }

    // false positive https://github.com/rust-lang/rust-clippy/issues/10407
    #[allow(clippy::uninit_vec)]
    unsafe fn insert(&mut self, id: Index, v: T) {
        let id = if Index::BITS > usize::BITS {
            // Saturate the cast to usize::MAX so if this overflows usize the
            // allocation below will fail.
            core::cmp::min(id, usize::MAX as Index) as usize
        } else {
            id as usize
        };

        if self.0.len() <= id {
            // NOTE: saturating add ensures that if this computation would
            // overflow it will instead fail the allocation when calling
            // reserve.
            let delta = if Index::BITS >= usize::BITS {
                id.saturating_add(1)
            } else {
                id + 1
            } - self.0.len();
            self.0.reserve(delta);
            // NOTE: Allocation would have failed if this addition would overflow
            // SAFETY: MaybeUninit elements don't require initialization and
            // the reserve call ensures the capacity will be sufficient for this
            // new length.
            unsafe { self.0.set_len(id + 1) };
        }
        // Write the value without reading or dropping
        // the (currently uninitialized) memory.
        // SAFETY: The length of the vec was extended to contain this index
        // above.
        unsafe { self.0.get_unchecked_mut(id) }.get_mut().write(v);
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        // SAFETY: Caller required to have called `insert` with this `id`.
        // Exclusive `&mut self` ensures no aliasing is occuring.
        let component_ref = unsafe { self.get(id) };
        // SAFETY: Caller not allowed to call other methods that access this
        // `id` as an initialized value after this call to `remove` so it is
        // safe to move out of this.
        unsafe { ptr::read(component_ref) }
    }
}

impl<T> SharedGetMutStorage<T> for VecStorage<T> {
    unsafe fn shared_get_mut(&self, id: Index) -> &mut T {
        // NOTE: `as` cast is not lossy since insert would have encountered an
        // allocation failure if this would overflow `usize.`
        // SAFETY: Caller required to call `insert` with this `id` (with no
        // following call to `remove` with that id or to `clean`).
        let ptr = unsafe { self.0.get_unchecked(id as usize) }.get();
        // SAFETY: Caller required to manage aliasing (ensuring there are no
        // extant shared references into the storage, this is called with
        // distinct ids, and that other methods that take `&self` aren't called
        // while the exclusive references returned here are alive (except for
        // `UnprotectedStorage::get` which may be used with this provided the
        // caller avoids creating aliasing references from both that live at the
        // same time)).
        let maybe_uninit = unsafe { &mut *ptr };
        // SAFETY: Requirement to have `insert`ed this component ensures that it
        // will be initialized.
        unsafe { maybe_uninit.assume_init_mut() }
    }
}

// SAFETY: `shared_get_mut` doesn't perform any overlapping mutable
// accesses when provided distinct indices and is safe to call from multiple
// threads at once.
unsafe impl<T> DistinctStorage for VecStorage<T> {}

/// Vector storage, like `VecStorage`, but allows safe access to the
/// interior slices because unused slots are always initialized.
///
/// Requires the component to implement `Default`.
///
/// `as_slice()` and `as_mut_slice()` indices correspond to entity IDs.
/// These can be compared to other `DefaultVecStorage`s, to other
/// `VecStorage`s, and to `Entity::id()`s for live entities.
pub struct DefaultVecStorage<T>(Vec<SyncUnsafeCell<T>>);

impl<T> Default for DefaultVecStorage<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> SliceAccess<T> for DefaultVecStorage<T> {
    type Element = T;

    /// Returns a slice of all the components in this storage.
    #[inline]
    fn as_slice(&self) -> &[Self::Element] {
        let unsafe_cell_slice_ptr = SyncUnsafeCell::as_cell_of_slice(self.0.as_slice()).get();
        // SAFETY: See `VecStorage` impl.
        unsafe { &*unsafe_cell_slice_ptr }
    }

    /// Returns a mutable slice of all the components in this storage.
    #[inline]
    fn as_mut_slice(&mut self) -> &mut [Self::Element] {
        SyncUnsafeCell::as_slice_mut(self.0.as_mut_slice())
    }
}

impl<T> UnprotectedStorage<T> for DefaultVecStorage<T>
where
    T: Default,
{
    type AccessMut<'a> = &'a mut T where T: 'a;

    unsafe fn clean<B>(&mut self, _has: B)
    where
        B: BitSetLike,
    {
        self.0.clear();
    }

    unsafe fn get(&self, id: Index) -> &T {
        // NOTE: `as` cast is not lossy since insert would have encountered an
        // allocation failure if this would overflow `usize.`
        // SAFETY: See `VecStorage` impl.
        let ptr = unsafe { self.0.get_unchecked(id as usize) }.get();
        // SAFETY: See `VecStorage` impl.
        unsafe { &*ptr }
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        // NOTE: `as` cast is not lossy since insert would have encountered an
        // allocation failure if this would overflow `usize.`
        // SAFETY: See `VecStorage` impl.
        unsafe { self.0.get_unchecked_mut(id as usize) }.get_mut()
    }

    unsafe fn insert(&mut self, id: Index, v: T) {
        let id = if Index::BITS > usize::BITS {
            // Saturate the cast to usize::MAX so if this overflows usize the
            // allocation below will fail.
            core::cmp::min(id, usize::MAX as Index) as usize
        } else {
            id as usize
        };

        if self.0.len() <= id {
            // fill all the empty slots with default values
            self.0.resize_with(id, Default::default);
            // store the desired value
            self.0.push(SyncUnsafeCell::new(v))
        } else {
            // store the desired value directly
            *self.0[id].get_mut() = v;
        }
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        // Take value leaving a default instance behind
        // SAFETY: Caller required to have called `insert` with this `id`.
        core::mem::take(unsafe { self.0.get_unchecked_mut(id as usize) }.get_mut())
    }
}

impl<T> SharedGetMutStorage<T> for DefaultVecStorage<T>
where
    T: Default,
{
    unsafe fn shared_get_mut(&self, id: Index) -> &mut T {
        // NOTE: `as` cast is not lossy since insert would have encountered an
        // allocation failure if this would overflow `usize.`
        // SAFETY: See `VecStorage` impl.
        let ptr = unsafe { self.0.get_unchecked(id as usize) }.get();
        // SAFETY: See `VecStorage` impl.
        unsafe { &mut *ptr }
    }
}

// SAFETY: `shared_get_mut` doesn't perform any overlapping mutable
// accesses when provided distinct indices and is safe to call from multiple
// threads at once.
unsafe impl<T> DistinctStorage for DefaultVecStorage<T> {}
