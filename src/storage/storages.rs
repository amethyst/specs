//! Different types of storages you can use for your components.

use std::collections::BTreeMap;
use std::marker::PhantomData;

use fnv::FnvHashMap;
use hibitset::{BitSet, BitSetAnd, BitSetNot};

use world::EntityIndex;
use {DistinctStorage, Index, Join, UnprotectedStorage};

/// BTreeMap-based storage.
pub struct BTreeStorage<T>(BTreeMap<Index, T>);

impl<T> UnprotectedStorage<T> for BTreeStorage<T> {
    fn new() -> Self {
        BTreeStorage(Default::default())
    }

    unsafe fn clean<F>(&mut self, _: F)
        where F: Fn(Index) -> bool
    {
        // nothing to do
    }

    unsafe fn get(&self, id: Index) -> &T {
        self.0.get(&id).unwrap()
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

/// Wrapper storage that stores modifications to components in a bitset.
///
/// **Note:** Joining over all components of a `FlaggedStorage` mutably will flag all components.**
/// What you want to instead is to use `check()` to first get the entities which contain
/// the component, and then conditionally set the component after a call to `get_mut_unchecked()`.
///
/// # Examples
///
/// ```rust
/// extern crate specs;
///
/// use specs::{Component, FlaggedStorage, Join, System, VecStorage, WriteStorage};
///
/// pub struct Comp(u32);
/// impl Component for Comp {
///     // `FlaggedStorage` acts as a wrapper around another storage.
///     // You can put any store inside of here (e.g. HashMapStorage, VecStorage, etc.)
///     type Storage = FlaggedStorage<Self, VecStorage<Self>>;
/// }
///
/// pub struct CompSystem;
/// impl<'a> System<'a> for CompSystem {
///     type SystemData = WriteStorage<'a, Comp>;
///     fn run(&mut self, mut comps: WriteStorage<'a, Comp>) {
///         // Iterates over all components like normal.
///         for comp in (&comps).join() {
///             // ...
///         }
///
///         // **Never do this**
///         // This will flag all components as modified regardless of whether the inner loop
///         // did modify their data.
///         for comp in (&mut comps).join() {
///             // ...
///         }
///
///         // Instead do something like:
///         for mut entry in (&comps.check()).join() {
///             if true { // check whether this component should be modified.
///                 let mut comp = comps.get_mut_unchecked(&mut entry);
///                 // ...
///             }
///         }
///
///         // To iterate over the flagged/modified components:
///         for flagged_comp in ((&comps).open().1).join() {
///             // ...
///         }
///
///         // Clears the flagged storage every frame with this system.
///         (&mut comps).open().1.clear_flags();
///     }
/// }
///# fn main() { }
/// ```
pub struct FlaggedStorage<C, T> {
    mask: BitSet,
    storage: T,
    phantom: PhantomData<C>,
}

impl<C, T: UnprotectedStorage<C>> UnprotectedStorage<C> for FlaggedStorage<C, T> {
    fn new() -> Self {
        FlaggedStorage {
            mask: BitSet::new(),
            storage: T::new(),
            phantom: PhantomData,
        }
    }
    unsafe fn clean<F>(&mut self, has: F) where F: Fn(Index) -> bool {
        self.clear_flags();
        self.storage.clean(has);
    }
    unsafe fn get(&self, id: Index) -> &C {
        self.storage.get(id)
    }
    unsafe fn get_mut(&mut self, id: Index) -> &mut C {
        // calling `.iter()` on an unconstrained mutable storage will flag everything
        self.flag(id);
        self.storage.get_mut(id)
    }
    unsafe fn insert(&mut self, id: Index, comp: C) {
        self.flag(id);
        self.storage.insert(id, comp);
    }
    unsafe fn remove(&mut self, id: Index) -> C {
        self.unflag(id);
        self.storage.remove(id)
    }
}

impl<C, T: UnprotectedStorage<C>> FlaggedStorage<C, T> {
    /// Whether the component that belongs to the given entity was flagged or not.
    pub fn flagged<E: EntityIndex>(&self, entity: E) -> bool {
        self.mask.contains(entity.index())
    }
    /// Clears the bitset for flagged components.
    ///
    /// Should be called at least once depending on where you want to reset the flags.
    pub fn clear_flags(&mut self) {
        self.mask.clear();
    }
    /// Removes the flag for the component of the given entity.
    pub fn unflag<E: EntityIndex>(&mut self, entity: E) {
        self.mask.remove(entity.index());
    }
    /// Flags a single component.
    pub fn flag<E: EntityIndex>(&mut self, entity: E) {
        self.mask.add(entity.index());
    }
}

impl<'a, C, T: UnprotectedStorage<C>> Join for &'a FlaggedStorage<C, T> {
    type Type = &'a C;
    type Value = &'a T;
    type Mask = &'a BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        (&self.mask, &self.storage)
    }
    unsafe fn get(v: &mut Self::Value, id: Index) -> &'a C {
        v.get(id)
    }
}

impl<'a, C, T: UnprotectedStorage<C>> Join for &'a mut FlaggedStorage<C, T> {
    type Type = &'a mut C;
    type Value = &'a mut T;
    type Mask = &'a BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        (&self.mask, &mut self.storage)
    }
    unsafe fn get(v: &mut Self::Value, id: Index) -> &'a mut C {
        // similar issue here as the `Storage<T, A, D>` implementation
        use std::mem;
        let value: &'a mut Self::Value = mem::transmute(v);
        value.get_mut(id)
    }
}

/// HashMap-based storage. Best suited for rare components.
pub struct HashMapStorage<T>(FnvHashMap<Index, T>);

impl<T> UnprotectedStorage<T> for HashMapStorage<T> {
    fn new() -> Self {
        HashMapStorage(Default::default())
    }

    unsafe fn clean<F>(&mut self, _: F)
        where F: Fn(Index) -> bool
    {
        //nothing to do
    }

    unsafe fn get(&self, id: Index) -> &T {
        self.0.get(&id).unwrap()
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
pub struct DenseVecStorage<T> {
    data: Vec<T>,
    entity_id: Vec<Index>,
    data_id: Vec<Index>,
}

impl<T> UnprotectedStorage<T> for DenseVecStorage<T> {
    fn new() -> Self {
        DenseVecStorage {
            data: Vec::new(),
            entity_id: Vec::new(),
            data_id: Vec::new(),
        }
    }

    unsafe fn clean<F>(&mut self, _: F)
        where F: Fn(Index) -> bool
    {
        // nothing to do
    }

    unsafe fn get(&self, id: Index) -> &T {
        let did = *self.data_id.get_unchecked(id as usize);
        self.data.get_unchecked(did as usize)
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        let did = *self.data_id.get_unchecked(id as usize);
        self.data.get_unchecked_mut(did as usize)
    }

    unsafe fn insert(&mut self, id: Index, v: T) {
        let id = id as usize;
        if self.data_id.len() <= id {
            let delta = id + 1 - self.data_id.len();
            self.data_id.reserve(delta);
            self.data_id.set_len(id + 1);
        }
        *self.data_id.get_unchecked_mut(id) = self.data.len() as Index;
        self.entity_id.push(id as Index);
        self.data.push(v);
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        let did = *self.data_id.get_unchecked(id as usize);
        let last = *self.entity_id.last().unwrap();
        *self.data_id.get_unchecked_mut(last as usize) = did;
        self.entity_id.swap_remove(did as usize);
        self.data.swap_remove(did as usize)
    }
}

unsafe impl<T> DistinctStorage for DenseVecStorage<T> {}

/// A null storage type, used for cases where the component
/// doesn't contain any data and instead works as a simple flag.
pub struct NullStorage<T>(T);

impl<T: Default> UnprotectedStorage<T> for NullStorage<T> {
    fn new() -> Self {
        NullStorage(Default::default())
    }
    unsafe fn clean<F>(&mut self, _: F) where F: Fn(Index) -> bool {}
    unsafe fn get(&self, _: Index) -> &T {
        &self.0
    }
    unsafe fn get_mut(&mut self, _: Index) -> &mut T {
        panic!("One does not simply modify a NullStorage")
    }
    unsafe fn insert(&mut self, _: Index, _: T) {}
    unsafe fn remove(&mut self, _: Index) -> T {
        Default::default()
    }
}

/// This is safe because mutating doesn't work and panics instead
unsafe impl<T> DistinctStorage for NullStorage<T> {}

/// Vector storage. Uses a simple `Vec`. Supposed to have maximum
/// performance for the components mostly present in entities.
pub struct VecStorage<T>(Vec<T>);

impl<T> UnprotectedStorage<T> for VecStorage<T> {
    fn new() -> Self {
        VecStorage(Vec::new())
    }

    unsafe fn clean<F>(&mut self, has: F)
        where F: Fn(Index) -> bool
    {
        use std::ptr;
        for (i, v) in self.0.iter_mut().enumerate() {
            if has(i as Index) {
                ptr::drop_in_place(v);
            }
        }
        self.0.set_len(0);
    }

    unsafe fn get(&self, id: Index) -> &T {
        self.0.get_unchecked(id as usize)
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        self.0.get_unchecked_mut(id as usize)
    }

    unsafe fn insert(&mut self, id: Index, v: T) {
        use std::ptr;

        let id = id as usize;
        if self.0.len() <= id {
            let delta = id + 1 - self.0.len();
            self.0.reserve(delta);
            self.0.set_len(id + 1);
        }
        // Write the value without reading or dropping
        // the (currently uninitialized) memory.
        ptr::write(self.0.get_unchecked_mut(id), v);
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        use std::ptr;

        ptr::read(self.get(id))
    }
}

unsafe impl<T> DistinctStorage for VecStorage<T> {}

/// Wrapper storage that stores modifications to components with a bitset along with
/// comparing equality to a cached version.
///
///# Example Usage:
/// 
/// ```rust
/// extern crate specs;
/// use specs::{Component, ChangedStorage, Join, System, VecStorage, WriteStorage};
/// 
/// #[derive(PartialEq, Clone)]
/// pub struct Comp(u32);
/// impl Component for Comp {
///     // `ChangedStorage` acts as a wrapper around another storage.
///     // You can put any store inside of here (e.g. HashMapStorage, VecStorage, etc.)
///     type Storage = ChangedStorage<Comp, VecStorage<Comp>>;
/// }
/// 
/// pub struct CompSystem;
/// impl<'a> System<'a> for CompSystem {
///     type SystemData = WriteStorage<'a, Comp>;
///     fn run(&mut self, mut comps: WriteStorage<'a, Comp>) {
///         // Iterates over all components like normal.
///         for comp in (&comps).join() {
///             // ...
///         }
/// 
///         // To iterate over the flagged/modified components:
///         for flagged_comp in (&comps).open().1.join() {
///             // ...
///         }
/// 
///         // Modify the components in some way.
///         for comp in (&mut comps).join() {
///             // ...
///         }
///
///         // Clears the changed storage every frame with this system.
///         (&mut comps).open().1.clear_flags();
///     }
/// }
///# fn main() { }
/// ```
pub struct ChangedStorage<C, U> {
    inner: FlaggedStorage<C, U>,
    cache: U,
    next: Option<Index>,
}

impl<C, U> ChangedStorage<C, U>
    where C: PartialEq + Clone,
          U: UnprotectedStorage<C>,
{
    /// Whether the component related to the entity was flagged or not.
    pub fn flagged<E: EntityIndex>(&self, entity: E) -> bool {
        self.inner.flagged(entity)
    }
    /// Clears the bitset for flagged components.
    ///
    /// Should be called at least once depending on where you want to reset the flags.
    pub fn clear_flags(&mut self) {
        if let Some(index) = self.next {
            unsafe { self.cache.insert(index, self.inner.get(index).clone()); }
            self.next = None;
        }
        self.inner.clear_flags();
    }
    /// Flags a single component as not flagged.
    pub fn unflag<E: EntityIndex>(&mut self, entity: E) {
        self.inner.unflag(entity);
    }
    /// Flags a single component as flagged.
    pub fn flag<E: EntityIndex>(&mut self, entity: E) {
        self.inner.flag(entity);
    }
}

impl<C, U> UnprotectedStorage<C> for ChangedStorage<C, U>
    where C: PartialEq + Clone,
          U: UnprotectedStorage<C>,
{
    fn new() -> Self {
        ChangedStorage {
            inner: FlaggedStorage::<C, U>::new(),
            cache: U::new(),
            next: None,
        }
    }
    unsafe fn clean<F>(&mut self, has: F) where F: Fn(Index) -> bool {
        self.inner.clean(&has);
        self.cache.clean(&has);
    }
    unsafe fn get(&self, id: Index) -> &C {
        self.inner.get(id)
    }
    unsafe fn get_mut(&mut self, id: Index) -> &mut C {
        if let Some(index) = self.next {
            if self.cache.get(index) == self.inner.get(index) {
                self.unflag(index);
            }
            else {
                self.cache.insert(index, self.inner.get(index).clone());
            }
        }
        self.next = Some(id);
        self.inner.get_mut(id)
    }
    unsafe fn insert(&mut self, id: Index, comp: C) {
        self.cache.insert(id, comp.clone());
        self.inner.insert(id, comp);
    }
    unsafe fn remove(&mut self, id: Index) -> C {
        if let Some(next) = self.next {
            if next == id {
                self.next = None;
            }
        }
        self.inner.remove(id)
    }
}

impl<'a, C, U: UnprotectedStorage<C>> Join for &'a ChangedStorage<C, U>
    where C: PartialEq,
          U: UnprotectedStorage<C>,
{
    type Type = &'a C;
    type Value = &'a U;
    type Mask = BitSetAnd<&'a BitSet, BitSetNot<BitSet>>;
    fn open(self) -> (Self::Mask, Self::Value) {
        // filter out singular bit in case that the cached
        // next index to check is invalid
        let mut single = BitSet::new();
        if let Some(index) = self.next {
            if unsafe { self.cache.get(index) } == unsafe { self.inner.get(index) } {
                single.add(index);
            }
        }

        let (bitset, storage) = (&self.inner).open();
        (BitSetAnd(bitset, BitSetNot(single)), storage)
    }
    unsafe fn get(v: &mut Self::Value, id: Index) -> &'a C {
        v.get(id)
    }
}

impl<'a, C, U: UnprotectedStorage<C>> Join for &'a mut ChangedStorage<C, U>
    where C: PartialEq + Clone,
          U: UnprotectedStorage<C>,
{
    type Type = &'a mut C;
    type Value = &'a mut U;
    type Mask = &'a BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        if let Some(index) = self.next {
            unsafe { 
                if self.cache.get(index) == self.inner.get(index) {
                    self.unflag(index);
                }
                else {
                    self.cache.insert(index, self.inner.get(index).clone());
                }
            }

            self.next = None;
        }
        (&mut self.inner).open()
    }
    unsafe fn get(v: &mut Self::Value, id: Index) -> &'a mut C {
        // similar issue here as the `Storage<T, A, D>` implementation
        use std::mem;
        let value: &'a mut Self::Value = mem::transmute(v);
        value.get_mut(id)
    }
}
