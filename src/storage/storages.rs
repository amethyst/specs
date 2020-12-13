//! Different types of storages you can use for your components.

use std::{collections::BTreeMap, mem::MaybeUninit};

use hashbrown::HashMap;
use hibitset::BitSetLike;

use crate::{
    storage::{DistinctStorage, UnprotectedStorage},
    world::Index,
};

/// Some storages can provide slices to access the underlying data.
///
/// The underlying data may be of type `T`, or it may be of a type
/// which wraps `T`. The associated type `Element` identifies what
/// the slices will contain.
pub trait SliceAccess<T> {
    type Element;

    fn as_slice(&self) -> &[Self::Element];
    fn as_mut_slice(&mut self) -> &mut [Self::Element];
}

/// BTreeMap-based storage.
pub struct BTreeStorage<T>(BTreeMap<Index, T>);

impl<T> Default for BTreeStorage<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> UnprotectedStorage<T> for BTreeStorage<T> {
    #[cfg(feature = "nightly")]
    type AccessMut<'a> where T: 'a = &'a mut T;

    unsafe fn clean<B>(&mut self, _has: B)
    where
        B: BitSetLike,
    {
        // nothing to do
    }

    unsafe fn get(&self, id: Index) -> &T {
        &self.0[&id]
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        self.0.get_mut(&id).unwrap()
    }

    unsafe fn insert(&mut self, id: Index, v: T) {
        self.0.insert(id, v);
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        self.0.remove(&id).unwrap()
    }
}

unsafe impl<T> DistinctStorage for BTreeStorage<T> {}

/// `HashMap`-based storage. Best suited for rare components.
///
/// This uses the [hashbrown::HashMap] internally.
pub struct HashMapStorage<T>(HashMap<Index, T>);

impl<T> Default for HashMapStorage<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> UnprotectedStorage<T> for HashMapStorage<T> {
    #[cfg(feature = "nightly")]
    type AccessMut<'a> where T: 'a = &'a mut T;

    unsafe fn clean<B>(&mut self, _has: B)
    where
        B: BitSetLike,
    {
        //nothing to do
    }

    unsafe fn get(&self, id: Index) -> &T {
        &self.0[&id]
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        self.0.get_mut(&id).unwrap()
    }

    unsafe fn insert(&mut self, id: Index, v: T) {
        self.0.insert(id, v);
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        self.0.remove(&id).unwrap()
    }
}

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
    data: Vec<T>,
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
        self.data.as_slice()
    }

    /// Returns a mutable slice of all the components in this storage.
    ///
    /// Indices inside the slice do not correspond to anything in particular,
    /// and especially do not correspond with entity IDs.
    #[inline]
    fn as_mut_slice(&mut self) -> &mut [Self::Element] {
        self.data.as_mut_slice()
    }
}

impl<T> UnprotectedStorage<T> for DenseVecStorage<T> {
    #[cfg(feature = "nightly")]
    type AccessMut<'a> where T: 'a = &'a mut T;

    unsafe fn clean<B>(&mut self, _has: B)
    where
        B: BitSetLike,
    {
        // nothing to do
    }

    unsafe fn get(&self, id: Index) -> &T {
        let did = self.data_id.get_unchecked(id as usize).assume_init();
        self.data.get_unchecked(did as usize)
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        let did = self.data_id.get_unchecked(id as usize).assume_init();
        self.data.get_unchecked_mut(did as usize)
    }

    unsafe fn insert(&mut self, id: Index, v: T) {
        let id = id as usize;
        if self.data_id.len() <= id {
            let delta = id + 1 - self.data_id.len();
            self.data_id.reserve(delta);
            self.data_id.set_len(id + 1);
        }
        self.data_id
            .get_unchecked_mut(id)
            .as_mut_ptr()
            .write(self.data.len() as Index);
        self.entity_id.push(id as Index);
        self.data.push(v);
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        let did = self.data_id.get_unchecked(id as usize).assume_init();
        let last = *self.entity_id.last().unwrap();
        self.data_id
            .get_unchecked_mut(last as usize)
            .as_mut_ptr()
            .write(did);
        self.entity_id.swap_remove(did as usize);
        self.data.swap_remove(did as usize)
    }
}

unsafe impl<T> DistinctStorage for DenseVecStorage<T> {}

/// A null storage type, used for cases where the component
/// doesn't contain any data and instead works as a simple flag.
pub struct NullStorage<T>(T);

impl<T> UnprotectedStorage<T> for NullStorage<T>
where
    T: Default,
{
    #[cfg(feature = "nightly")]
    type AccessMut<'a> where T: 'a = &'a mut T;

    unsafe fn clean<B>(&mut self, _has: B)
    where
        B: BitSetLike,
    {
    }

    unsafe fn get(&self, _: Index) -> &T {
        &self.0
    }

    unsafe fn get_mut(&mut self, _: Index) -> &mut T {
        &mut self.0
    }

    unsafe fn insert(&mut self, _: Index, _: T) {}

    unsafe fn remove(&mut self, _: Index) -> T {
        Default::default()
    }
}

impl<T> Default for NullStorage<T>
where
    T: Default,
{
    fn default() -> Self {
        use std::mem::size_of;

        assert_eq!(size_of::<T>(), 0, "NullStorage can only be used with ZST");

        NullStorage(Default::default())
    }
}

/// This is safe because you cannot mutate ZSTs.
unsafe impl<T> DistinctStorage for NullStorage<T> {}

/// Vector storage. Uses a simple `Vec`. Supposed to have maximum
/// performance for the components mostly present in entities.
///
/// `as_slice()` and `as_mut_slice()` indices correspond to
/// entity IDs. These can be compared to other `VecStorage`s, to
/// other `DefaultVecStorage`s, and to `Entity::id()`s for live
/// entities.
pub struct VecStorage<T>(Vec<MaybeUninit<T>>);

impl<T> Default for VecStorage<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> SliceAccess<T> for VecStorage<T> {
    type Element = MaybeUninit<T>;

    #[inline]
    fn as_slice(&self) -> &[Self::Element] {
        self.0.as_slice()
    }

    #[inline]
    fn as_mut_slice(&mut self) -> &mut [Self::Element] {
        self.0.as_mut_slice()
    }
}

impl<T> UnprotectedStorage<T> for VecStorage<T> {
    #[cfg(feature = "nightly")]
    type AccessMut<'a> where T: 'a = &'a mut T;

    unsafe fn clean<B>(&mut self, has: B)
    where
        B: BitSetLike,
    {
        use std::ptr;
        for (i, v) in self.0.iter_mut().enumerate() {
            if has.contains(i as u32) {
                // drop in place
                ptr::drop_in_place(&mut *v.as_mut_ptr());
            }
        }
        self.0.set_len(0);
    }

    unsafe fn get(&self, id: Index) -> &T {
        &*self.0.get_unchecked(id as usize).as_ptr()
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        &mut *self.0.get_unchecked_mut(id as usize).as_mut_ptr()
    }

    unsafe fn insert(&mut self, id: Index, v: T) {
        let id = id as usize;
        if self.0.len() <= id {
            let delta = id + 1 - self.0.len();
            self.0.reserve(delta);
            self.0.set_len(id + 1);
        }
        // Write the value without reading or dropping
        // the (currently uninitialized) memory.
        *self.0.get_unchecked_mut(id as usize) = MaybeUninit::new(v);
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        use std::ptr;
        ptr::read(self.get(id))
    }
}

unsafe impl<T> DistinctStorage for VecStorage<T> {}

/// Vector storage, like `VecStorage`, but allows safe access to the
/// interior slices because unused slots are always initialized.
///
/// Requires the component to implement `Default`.
///
/// `as_slice()` and `as_mut_slice()` indices correspond to entity IDs.
/// These can be compared to other `DefaultVecStorage`s, to other
/// `VecStorage`s, and to `Entity::id()`s for live entities.
pub struct DefaultVecStorage<T>(Vec<T>);

impl<T> Default for DefaultVecStorage<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> UnprotectedStorage<T> for DefaultVecStorage<T>
where
    T: Default,
{
    #[cfg(feature = "nightly")]
    type AccessMut<'a> where T: 'a = &'a mut T;

    unsafe fn clean<B>(&mut self, _has: B)
    where
        B: BitSetLike,
    {
        self.0.clear();
    }

    unsafe fn get(&self, id: Index) -> &T {
        self.0.get_unchecked(id as usize)
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        self.0.get_unchecked_mut(id as usize)
    }

    unsafe fn insert(&mut self, id: Index, v: T) {
        let id = id as usize;

        if self.0.len() <= id {
            // fill all the empty slots with default values
            self.0.resize_with(id, Default::default);
            // store the desired value
            self.0.push(v)
        } else {
            // store the desired value directly
            self.0[id] = v;
        }
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        // make a new default value
        let mut v = T::default();
        // swap it into the vec
        std::ptr::swap(self.0.get_unchecked_mut(id as usize), &mut v);
        // return the old value
        v
    }
}

unsafe impl<T> DistinctStorage for DefaultVecStorage<T> {}

impl<T> SliceAccess<T> for DefaultVecStorage<T> {
    type Element = T;

    /// Returns a slice of all the components in this storage.
    #[inline]
    fn as_slice(&self) -> &[Self::Element] {
        self.0.as_slice()
    }

    /// Returns a mutable slice of all the components in this storage.
    #[inline]
    fn as_mut_slice(&mut self) -> &mut [Self::Element] {
        self.0.as_mut_slice()
    }
}
