use std::{
    borrow::{Borrow, BorrowMut},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use hibitset::BitSet;
use shred::Fetch;

use crate::join::Join;

#[cfg(feature = "parallel")]
use crate::join::ParJoin;
use crate::{
    storage::{AccessMutReturn, DistinctStorage, MaskedStorage, Storage, UnprotectedStorage},
    world::{Component, EntitiesRes, Entity, Index},
};

/// Specifies that the `RestrictedStorage` cannot run in parallel.
///
/// A mutable `RestrictedStorage` can call `get`, `get_mut`, `get_unchecked` and
/// `get_mut_unchecked` for deferred/restricted access while an immutable
/// version can only call the immutable accessors.
pub enum SequentialRestriction {}
/// Specifies that the `RestrictedStorage` can run in parallel mutably.
///
/// This means the storage can only call `get_mut_unchecked` and
/// `get_unchecked`.
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
///             if comps.get_unchecked().0 < 5 {
///                 // Get a mutable reference now.
///                 let mut mutable = comps.get_mut_unchecked();
///                 mutable.0 += 1;
///             }
///         }
///     }
/// }
/// ```
pub struct RestrictedStorage<'rf, 'st: 'rf, C, S, B, Restrict>
where
    C: Component,
    S: Borrow<C::Storage> + 'rf,
    B: Borrow<BitSet> + 'rf,
{
    bitset: B,
    data: S,
    entities: &'rf Fetch<'st, EntitiesRes>,
    phantom: PhantomData<(C, Restrict)>,
}

#[cfg(feature = "parallel")]
unsafe impl<'rf, 'st: 'rf, C, S, B> ParJoin
    for &'rf mut RestrictedStorage<'rf, 'st, C, S, B, MutableParallelRestriction>
where
    C: Component,
    S: BorrowMut<C::Storage> + 'rf,
    B: Borrow<BitSet> + 'rf,
    C::Storage: Sync + DistinctStorage,
{
}

#[cfg(feature = "parallel")]
unsafe impl<'rf, 'st: 'rf, C, S, B, Restrict> ParJoin
    for &'rf RestrictedStorage<'rf, 'st, C, S, B, Restrict>
where
    C: Component,
    S: Borrow<C::Storage> + 'rf,
    B: Borrow<BitSet> + 'rf,
    Restrict: ImmutableAliasing,
{
}

impl<'rf, 'st: 'rf, C, S, B, Restrict> Join for &'rf RestrictedStorage<'rf, 'st, C, S, B, Restrict>
where
    C: Component,
    S: Borrow<C::Storage>,
    B: Borrow<BitSet>,
{
    type Mask = &'rf BitSet;
    type Type = PairedStorage<'rf, 'st, C, &'rf C::Storage, &'rf BitSet, Restrict>;
    type Value = (&'rf C::Storage, &'rf Fetch<'st, EntitiesRes>, &'rf BitSet);

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
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

impl<'rf, 'st: 'rf, C, S, B, Restrict> Join
    for &'rf mut RestrictedStorage<'rf, 'st, C, S, B, Restrict>
where
    C: Component,
    S: BorrowMut<C::Storage>,
    B: Borrow<BitSet>,
{
    type Mask = &'rf BitSet;
    type Type = PairedStorage<'rf, 'st, C, &'rf mut C::Storage, &'rf BitSet, Restrict>;
    type Value = (
        &'rf mut C::Storage,
        &'rf Fetch<'st, EntitiesRes>,
        &'rf BitSet,
    );

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        let bitset = self.bitset.borrow();
        (bitset, (self.data.borrow_mut(), self.entities, bitset))
    }

    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        // TODO: this is unsound with a non streaming JoinIter
        //
        // Calls with distinct `id`s will create aliased mutable
        // references that are returned all referencing the same Self::Value.
        //
        // Additionally, using `get_mut` with two separate `PairedStorages`s can
        // produce aliased mutable references:
        //
        // use specs::prelude::*;
        // use specs::storage::PairedStorage;
        //
        // struct Pos(f32);
        //
        // impl Component for Pos {
        //     type Storage = VecStorage<Self>;
        // }
        //
        // fn main() {
        //     let mut world = World::new();
        //
        //     world.register::<Pos>();
        //
        //     world.create_entity().with(Pos(0.0)).build();
        //     world.create_entity().with(Pos(1.6)).build();
        //     world.create_entity().with(Pos(5.4)).build();
        //
        //     let entities = world.entities();
        //     let mut pos = world.write_storage::<Pos>();
        //
        //     let mut restrict_pos = pos.restrict_mut();
        //     let mut vec: Vec<PairedStorage<'_, '_, _, &mut _, _, _>> = (&mut restrict_pos).join().collect();
        //     let (a, vec) = vec.split_first_mut().unwrap();
        //     let (b, _) = vec.split_first_mut().unwrap();
        //     let entity = entities.entity(0);
        //     let alias_a: &mut Pos = a.get_mut(entity).unwrap();
        //     let alias_b: &mut Pos = b.get_mut(entity).unwrap();
        //     assert!(core::ptr::eq(alias_a, alias_b));
        // }
        //
        // Also, since PairedStorage is Send, event if the underlying storage does not impl DistinctStorage
        // sending a PairedStorage to another thread can be used to call the underlying get_mut
        // simultaneously
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
    /// Builds an immutable `RestrictedStorage` out of a `Storage`. Allows
    /// deferred unchecked access to the entity's component.
    ///
    /// This is returned as a `ParallelRestriction` version since you can only
    /// get immutable components with this which is safe for parallel by
    /// default.
    pub fn restrict<'rf>(
        &'rf self,
    ) -> RestrictedStorage<'rf, 'st, T, &T::Storage, &BitSet, ImmutableParallelRestriction> {
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
    pub fn restrict_mut<'rf>(
        &'rf mut self,
    ) -> RestrictedStorage<'rf, 'st, T, &mut T::Storage, &BitSet, SequentialRestriction> {
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
    ) -> RestrictedStorage<'rf, 'st, T, &mut T::Storage, &BitSet, MutableParallelRestriction> {
        let (mask, data) = self.data.open_mut();
        RestrictedStorage {
            bitset: mask,
            data,
            entities: &self.entities,
            phantom: PhantomData,
        }
    }
}

/// Pairs a storage with an index, meaning that the index is guaranteed to exist
/// as long as the `PairedStorage<C, S>` exists.
pub struct PairedStorage<'rf, 'st: 'rf, C, S, B, Restrict> {
    index: Index,
    storage: S,
    bitset: B,
    entities: &'rf Fetch<'st, EntitiesRes>,
    phantom: PhantomData<(C, Restrict)>,
}

impl<'rf, 'st, C, S, B, Restrict> PairedStorage<'rf, 'st, C, S, B, Restrict>
where
    C: Component,
    S: Borrow<C::Storage>,
    B: Borrow<BitSet>,
{
    /// Gets the component related to the current entry without checking whether
    /// the storage has it or not.
    //
    // TODO: this is a misleading name since it implies something does need to be checked but we are
    // skipping it, when instead this is just getting the one entity's component that can be
    // retrieved without a check because we know it exists.
    pub fn get_unchecked(&self) -> &C {
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
    pub fn get_mut_unchecked(&mut self) -> AccessMutReturn<'_, C> {
        unsafe { self.storage.borrow_mut().get_access_mut(self.index) }
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
    /// This only works for non-parallel or immutably parallel
    /// `RestrictedStorage`.
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
    /// This only works if this is a non-parallel `RestrictedStorage`,
    /// otherwise you could access the same component mutably in two different
    /// threads.
    pub fn get_mut(&mut self, entity: Entity) -> Option<AccessMutReturn<'_, C>> {
        if self.bitset.borrow().contains(entity.id()) && self.entities.is_alive(entity) {
            Some(unsafe { self.storage.borrow_mut().get_access_mut(entity.id()) })
        } else {
            None
        }
    }
}
