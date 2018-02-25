use std::borrow::{Borrow, BorrowMut};
use std::fmt;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use hibitset::BitSet;

use join::Join;
#[cfg(feature = "parallel")]
use join::ParJoin;
use storage::{MaskedStorage, Storage, UnprotectedStorage};
use world::{Component, Entities, Entity, EntityIndex, Index};

/// Specifies that the `RestrictedStorage` cannot run in parallel.
///
/// A mutable `RestrictedStorage` can call `get`, `get_mut`, `get_unchecked` and
/// `get_mut_unchecked` for deferred access while an immutable version can only
/// call the immutable accessors.
pub enum SequentialRestriction {}
/// Specifies that the `RestrictedStorage` can run in parallel mutably.
///
/// This means the storage can only call `get_mut_unchecked` and `get_unchecked`.
pub enum MutableParallelRestriction {}
/// Specifies that the `RestrictedStorage` can run in parallel immutably.
///
/// This means that the storage can call `get`, `get_unchecked`.
pub enum ImmutableParallelRestriction {}

/// Restrictions that are allowed to access `RestrictedStorage::get`.
pub trait ImmutableAliasing: Sized {}
impl ImmutableAliasing for SequentialRestriction {}
impl ImmutableAliasing for ImmutableParallelRestriction {}

/// Similar to a `MaskedStorage` and a `Storage` combined, but restricts usage
/// to only getting and modifying the components. That means nothing that would
/// modify the inner bitset so the iteration cannot be invalidated. For example,
/// no insertion or removal is allowed.
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
pub struct RestrictedStorage<'rf, 'st: 'rf, B, T, R, RT>
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

#[cfg(feature = "parallel")]
unsafe impl<'rf, 'st: 'rf, B, T, R> ParJoin
    for &'rf mut RestrictedStorage<'rf, 'st, B, T, R, MutableParallelRestriction>
where
    T: Component,
    R: BorrowMut<T::Storage> + 'rf,
    B: Borrow<BitSet> + 'rf,
{
}

#[cfg(feature = "parallel")]
unsafe impl<'rf, 'st: 'rf, B, T, R, RT> ParJoin for &'rf RestrictedStorage<'rf, 'st, B, T, R, RT>
where
    T: Component,
    R: Borrow<T::Storage> + 'rf,
    B: Borrow<BitSet> + 'rf,
    RT: ImmutableAliasing,
{
}

impl<'rf, 'st, B, T, R, RT> RestrictedStorage<'rf, 'st, B, T, R, RT>
where
    T: Component,
    R: Borrow<T::Storage>,
    B: Borrow<BitSet>,
{
    /// Gets the component related to the current entry without checking whether
    /// the storage has it or not.
    pub fn get_unchecked(&self, entry: &Entry<'rf, T>) -> &T {
        entry.assert_same_storage(self.data.borrow());
        unsafe { self.data.borrow().get(entry.index()) }
    }
}

impl<'rf, 'st, B, T, R, RT> RestrictedStorage<'rf, 'st, B, T, R, RT>
where
    T: Component,
    R: BorrowMut<T::Storage>,
    B: Borrow<BitSet>,
{
    /// Gets the component related to the current entry without checking whether
    /// the storage has it or not.
    pub fn get_mut_unchecked(&mut self, entry: &Entry<'rf, T>) -> &mut T {
        entry.assert_same_storage(self.data.borrow());
        unsafe { self.data.borrow_mut().get_mut(entry.index()) }
    }
}

impl<'rf, 'st, B, T, R, RT> RestrictedStorage<'rf, 'st, B, T, R, RT>
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

impl<'rf, 'st, B, T, R> RestrictedStorage<'rf, 'st, B, T, R, SequentialRestriction>
where
    T: Component,
    R: BorrowMut<T::Storage>,
    B: Borrow<BitSet>,
{
    /// Attempts to get the component related to the entity mutably.
    ///
    /// Functions similar to the normal `Storage::get_mut` implementation.
    ///
    /// This only works if this is a non-parallel `RestrictedStorage`,
    /// otherwise you could access the same component mutably in two different threads.
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        if self.bitset.borrow().contains(entity.id()) && self.entities.is_alive(entity) {
            Some(unsafe { self.data.borrow_mut().get_mut(entity.id()) })
        } else {
            None
        }
    }
}

impl<'rf, 'st: 'rf, B, T, R, RT> Join for &'rf RestrictedStorage<'rf, 'st, B, T, R, RT>
where
    T: Component,
    R: Borrow<T::Storage>,
    B: Borrow<BitSet>,
{
    type Type = (Entry<'rf, T>, Self);
    type Value = Self;
    type Mask = &'rf BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        (self.bitset.borrow(), self)
    }
    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        let entry = Entry {
            id,
            pointer: value.data.borrow() as *const T::Storage,
            phantom: PhantomData,
        };

        (entry, value)
    }
}

impl<'rf, 'st: 'rf, B, T, R, RT> Join for &'rf mut RestrictedStorage<'rf, 'st, B, T, R, RT>
where
    T: Component,
    R: BorrowMut<T::Storage>,
    B: Borrow<BitSet>,
{
    type Type = (Entry<'rf, T>, Self);
    type Value = Self;
    type Mask = BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        (self.bitset.borrow().clone(), self)
    }
    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        use std::mem;
        let entry = Entry {
            id,
            pointer: value.data.borrow() as *const T::Storage,
            phantom: PhantomData,
        };

        let value: &'rf mut Self::Value = mem::transmute(value);
        (entry, value)
    }
}

impl<'st, T, D> Storage<'st, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
{
    /// Builds an immutable `RestrictedStorage` out of a `Storage`. Allows deferred
    /// unchecked access to the entity's component.
    ///
    /// This is returned as a `ParallelRestriction` version since you can only get
    /// immutable components with this which is safe for parallel by default.
    pub fn restrict<'rf>(
        &'rf self,
    ) -> RestrictedStorage<'rf, 'st, &BitSet, T, &T::Storage, ImmutableParallelRestriction> {
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
    /// Builds a mutable `RestrictedStorage` out of a `Storage`. Allows restricted
    /// access to the inner components without allowing invalidating the
    /// bitset for iteration in `Join`.
    pub fn restrict_mut<'rf>(
        &'rf mut self,
    ) -> RestrictedStorage<'rf, 'st, &BitSet, T, &mut T::Storage, SequentialRestriction> {
        let (mask, data) = self.data.open_mut();
        RestrictedStorage {
            bitset: mask,
            data,
            entities: &self.entities,
            phantom: PhantomData,
        }
    }

    /// Builds a mutable, parallel `RestrictedStorage`,
    /// does not allow mutably getting other components
    /// aside from the current iteration.
    pub fn par_restrict_mut<'rf>(
        &'rf mut self,
    ) -> RestrictedStorage<'rf, 'st, &BitSet, T, &mut T::Storage, MutableParallelRestriction> {
        let (mask, data) = self.data.open_mut();
        RestrictedStorage {
            bitset: mask,
            data,
            entities: &self.entities,
            phantom: PhantomData,
        }
    }
}

/// An entry to a storage.
pub struct Entry<'rf, T>
where
    T: Component,
{
    id: Index,
    // Pointer for comparison when attempting to check against a storage.
    pointer: *const T::Storage,
    phantom: PhantomData<&'rf ()>,
}

unsafe impl<'rf, T: Component> Send for Entry<'rf, T> {}
unsafe impl<'rf, T: Component> Sync for Entry<'rf, T> {}

impl<'rf, T> fmt::Debug for Entry<'rf, T>
where
    T: Component,
{
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(
            formatter,
            "Entry {{ id: {}, pointer: {:?} }}",
            self.id, self.pointer
        )
    }
}

impl<'rf, T> Entry<'rf, T>
where
    T: Component,
{
    #[inline]
    fn assert_same_storage(&self, storage: &T::Storage) {
        assert_eq!(
            self.pointer, storage as *const T::Storage,
            "Attempt to get an unchecked entry from a storage: {:?} {:?}",
            self.pointer, storage as *const T::Storage
        );
    }
}

impl<'rf, T> EntityIndex for Entry<'rf, T>
where
    T: Component,
{
    fn index(&self) -> Index {
        self.id
    }
}

impl<'a, 'rf, T> EntityIndex for &'a Entry<'rf, T>
where
    T: Component,
{
    fn index(&self) -> Index {
        (*self).index()
    }
}
