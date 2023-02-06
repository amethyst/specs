//! Joining of components for iteration over entities with specific components.

use hibitset::{BitIter, BitSetLike};
use shred::{Fetch, FetchMut, Read, ReadExpect, Resource, Write, WriteExpect};
use std::ops::{Deref, DerefMut};

use crate::world::Index;

mod bit_and;
mod lend_join;
mod maybe;
#[cfg(feature = "parallel")]
mod par_join;

pub use bit_and::BitAnd;
#[nougat::gat(Type)]
pub use lend_join::LendJoin;
pub use lend_join::{JoinLendIter, LendJoinType, RepeatableLendGet};
pub use maybe::MaybeJoin;
#[cfg(feature = "parallel")]
pub use par_join::{JoinParIter, ParJoin};

/// The purpose of the `Join` trait is to provide a way
/// to access multiple storages at the same time with
/// the merged bit set.
///
/// Joining component storages means that you'll only get values where
/// for a given entity every storage has an associated component.
///
/// ## Example
///
/// ```
/// # use specs::prelude::*;
/// # use specs::world::EntitiesRes;
/// # #[derive(Debug, PartialEq)]
/// # struct Pos; impl Component for Pos { type Storage = VecStorage<Self>; }
/// # #[derive(Debug, PartialEq)]
/// # struct Vel; impl Component for Vel { type Storage = VecStorage<Self>; }
/// let mut world = World::new();
///
/// world.register::<Pos>();
/// world.register::<Vel>();
///
/// {
///     let pos = world.read_storage::<Pos>();
///     let vel = world.read_storage::<Vel>();
///
///     // There are no entities yet, so no pair will be returned.
///     let joined: Vec<_> = (&pos, &vel).join().collect();
///     assert_eq!(joined, vec![]);
/// }
///
/// world.create_entity().with(Pos).build();
///
/// {
///     let pos = world.read_storage::<Pos>();
///     let vel = world.read_storage::<Vel>();
///
///     // Although there is an entity, it only has `Pos`.
///     let joined: Vec<_> = (&pos, &vel).join().collect();
///     assert_eq!(joined, vec![]);
/// }
///
/// let ent = world.create_entity().with(Pos).with(Vel).build();
///
/// {
///     let pos = world.read_storage::<Pos>();
///     let vel = world.read_storage::<Vel>();
///
///     // Now there is one entity that has both a `Vel` and a `Pos`.
///     let joined: Vec<_> = (&pos, &vel).join().collect();
///     assert_eq!(joined, vec![(&Pos, &Vel)]);
///
///     // If we want to get the entity the components are associated to,
///     // we need to join over `Entities`:
///
///     let entities = world.read_resource::<EntitiesRes>();
///     // note: `EntitiesRes` is the fetched resource; we get back
///     // `Read<EntitiesRes>`.
///     // `Read<EntitiesRes>` can also be referred to by `Entities` which
///     // is a shorthand type definition to the former type.
///
///     let joined: Vec<_> = (&entities, &pos, &vel).join().collect();
///     assert_eq!(joined, vec![(ent, &Pos, &Vel)]);
/// }
/// ```
///
/// ## Iterating over a single storage
///
/// `Join` can also be used to iterate over a single
/// storage, just by writing `(&storage).join()`.
///
/// # Safety
///
/// The `Self::Mask` value returned with the `Self::Value` must correspond such
/// that it is safe to retrieve items from `Self::Value` whose presence is
/// indicated in the mask. As part of this, `BitSetLike::iter` must not produce
/// an iterator that repeats an `Index` value if the `LendJoin::get` impl relies
/// on not being called twice with the same `Index`. (S-TODO update impls:
/// probably restrict, entry, and drain)
pub unsafe trait Join {
    /// Type of joined components.
    type Type;
    /// Type of joined storages.
    type Value;
    /// Type of joined bit mask.
    type Mask: BitSetLike;

    /// Create a joined iterator over the contents.
    fn join(self) -> JoinIter<Self>
    where
        Self: Sized,
    {
        JoinIter::new(self)
    }

    /// Open this join by returning the mask and the storages.
    ///
    /// # Safety
    ///
    /// This is unsafe because implementations of this trait can permit the
    /// `Value` to be mutated independently of the `Mask`. If the `Mask` does
    /// not correctly report the status of the `Value` then illegal memory
    /// access can occur.
    unsafe fn open(self) -> (Self::Mask, Self::Value);

    /// Get a joined component value by a given index.
    ///
    // S-TODO: evaluate all impls (TODO: probably restrict, entry, and drain)
    ///
    /// # Safety
    ///
    /// * A call to `get` must be preceded by a check if `id` is part of
    ///   `Self::Mask`.
    /// * Multiple calls with the same `id` are not allowed, for a particular
    ///   instance of the values from [`open`](Join::open).
    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type;

    /// If this `Join` typically returns all indices in the mask, then iterating
    /// over only it or combined with other joins that are also dangerous will
    /// cause the `JoinIter` to go through all indices which is usually not what
    /// is wanted and will kill performance.
    #[inline]
    fn is_unconstrained() -> bool {
        false
    }
}

/// `JoinIter` is an `Iterator` over a group of storages.
#[must_use]
pub struct JoinIter<J: Join> {
    keys: BitIter<J::Mask>,
    values: J::Value,
}

impl<J: Join> JoinIter<J> {
    /// Create a new join iterator.
    pub fn new(j: J) -> Self {
        if <J as Join>::is_unconstrained() {
            log::warn!(
                "`Join` possibly iterating through all indices, \
                you might've made a join with all `MaybeJoin`s, \
                which is unbounded in length."
            );
        }

        // SAFETY: We do not swap out the mask or the values, nor do we allow it
        // by exposing them.
        let (keys, values) = unsafe { j.open() };
        JoinIter {
            keys: keys.iter(),
            values,
        }
    }
}

impl<J: Join> std::iter::Iterator for JoinIter<J> {
    type Item = J::Type;

    fn next(&mut self) -> Option<J::Type> {
        // SAFETY: Since `idx` is yielded from `keys` (the mask), it is
        // necessarily a part of it. `Join` requires that the iterator doesn't
        // repeat indices and we advance the iterator for each `get` call.
        self.keys
            .next()
            .map(|idx| unsafe { J::get(&mut self.values, idx) })
    }
}

// Implementations of `LendJoin`, `Join`, and `ParJoin` for tuples, `Fetch`,
// `Read`, `ReadExpect`, `FetchMut`, `Write`, and `WriteExpect`.

macro_rules! define_open {
    // use variables to indicate the arity of the tuple
    ($($from:ident),*) => {
        // SAFETY: The returned mask in `open` is the intersection of the masks
        // from each type in this tuple. So if an `id` is present in the
        // combined mask, it will be safe to retrieve the corresponding items.
        // Iterating the mask does not repeat indices.
        #[nougat::gat]
        unsafe impl<$($from,)*> LendJoin for ($($from),*,)
            where $($from: LendJoin),*,
                  ($(<$from as LendJoin>::Mask,)*): BitAnd,
        {
            type Type<'next> = ($(<$from as LendJoin>::Type<'next>),*,);
            type Value = ($($from::Value),*,);
            type Mask = <($($from::Mask,)*) as BitAnd>::Value;

            #[allow(non_snake_case)]
            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                let ($($from,)*) = self;
                // SAFETY: While we do expose the mask and the values and
                // therefore would allow swapping them, this method is `unsafe`
                // and relies on the same invariants.
                let ($($from,)*) = unsafe { ($($from.open(),)*) };
                (
                    ($($from.0),*,).and(),
                    ($($from.1),*,)
                )
            }

            #[allow(non_snake_case)]
            unsafe fn get<'next>(v: &'next mut Self::Value, i: Index) -> Self::Type<'next>
            where
                Self: 'next,
            {
                let &mut ($(ref mut $from,)*) = v;
                // SAFETY: `get` is safe to call as the caller must have checked
                // the mask, which only has a key that exists in all of the
                // storages. Requirement to not call with the same ID more than
                // once (unless `RepeatableLendGet` is implemented) is passed to
                // the caller.
                unsafe { ($($from::get($from, i),)*) }
            }

            #[inline]
            fn is_unconstrained() -> bool {
                let mut unconstrained = true;
                $( unconstrained = unconstrained && $from::is_unconstrained(); )*
                unconstrained
            }
        }

        // SAFETY: Tuple impls of `LendJoin` simply defer to the individual
        // storages. Thus, if all of them implement this, it is safe to call
        // `LendJoin::get` multiple times with the same ID.
        unsafe impl<$($from,)*> RepeatableLendGet for ($($from),*,)
            where $($from: RepeatableLendGet),*,
                  ($(<$from as LendJoin>::Mask,)*): BitAnd, {}

        // SAFETY: The returned mask in `open` is the intersection of the masks
        // from each type in this tuple. So if an `id` is present in the
        // combined mask, it will be safe to retrieve the corresponding items.
        // Iterating the mask does not repeat indices.
        unsafe impl<$($from,)*> Join for ($($from),*,)
            where $($from: Join),*,
                  ($(<$from as Join>::Mask,)*): BitAnd,
        {
            type Type = ($($from::Type),*,);
            type Value = ($($from::Value),*,);
            type Mask = <($($from::Mask,)*) as BitAnd>::Value;

            #[allow(non_snake_case)]
            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                let ($($from,)*) = self;
                // SAFETY: While we do expose the mask and the values and
                // therefore would allow swapping them, this method is `unsafe`
                // and relies on the same invariants.
                let ($($from,)*) = unsafe { ($($from.open(),)*) };
                (
                    ($($from.0),*,).and(),
                    ($($from.1),*,)
                )
            }

            #[allow(non_snake_case)]
            unsafe fn get(v: &mut Self::Value, i: Index) -> Self::Type {
                let &mut ($(ref mut $from,)*) = v;
                // SAFETY: `get` is safe to call as the caller must have checked
                // the mask, which only has a key that exists in all of the
                // storages. Requirement to not use the same ID multiple times
                // is also passed to the caller.
                unsafe { ($($from::get($from, i),)*) }
            }

            #[inline]
            fn is_unconstrained() -> bool {
                let mut unconstrained = true;
                $( unconstrained = unconstrained && $from::is_unconstrained(); )*
                unconstrained
            }
        }

        // SAFETY: This is safe to implement since all components implement
        // `ParJoin`. If the access of every individual `get` is callable from
        // multiple threads, then this `get` method will be as well.
        //
        // The returned mask in `open` is the intersection of the masks
        // from each type in this tuple. So if an `id` is present in the
        // combined mask, it will be safe to retrieve the corresponding items.
        #[cfg(feature = "parallel")]
        unsafe impl<$($from,)*> ParJoin for ($($from),*,)
            where $($from: ParJoin),*,
                  ($(<$from as ParJoin>::Mask,)*): BitAnd,
        {
            type Type = ($($from::Type),*,);
            type Value = ($($from::Value),*,);
            type Mask = <($($from::Mask,)*) as BitAnd>::Value;

            #[allow(non_snake_case)]
            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                let ($($from,)*) = self;
                // SAFETY: While we do expose the mask and the values and
                // therefore would allow swapping them, this method is `unsafe`
                // and relies on the same invariants.
                let ($($from,)*) = unsafe { ($($from.open(),)*) };
                (
                    ($($from.0),*,).and(),
                    ($($from.1),*,)
                )
            }

            #[allow(non_snake_case)]
            unsafe fn get(v: &Self::Value, i: Index) -> Self::Type {
                let &($(ref $from,)*) = v;
                // SAFETY: `get` is safe to call as the caller must have checked
                // the mask, which only has a key that exists in all of the
                // storages.
                unsafe { ($($from::get($from, i),)*) }
            }

            #[inline]
            fn is_unconstrained() -> bool {
                let mut unconstrained = true;
                $( unconstrained = unconstrained && $from::is_unconstrained(); )*
                unconstrained
            }
        }
    }
}

define_open! {A}
define_open! {A, B}
define_open! {A, B, C}
define_open! {A, B, C, D}
define_open! {A, B, C, D, E}
define_open! {A, B, C, D, E, F}
define_open! {A, B, C, D, E, F, G}
define_open! {A, B, C, D, E, F, G, H}
define_open! {A, B, C, D, E, F, G, H, I}
define_open! {A, B, C, D, E, F, G, H, I, J}
define_open! {A, B, C, D, E, F, G, H, I, J, K}
define_open! {A, B, C, D, E, F, G, H, I, J, K, L}
define_open! {A, B, C, D, E, F, G, H, I, J, K, L, M}
define_open! {A, B, C, D, E, F, G, H, I, J, K, L, M, N}
define_open! {A, B, C, D, E, F, G, H, I, J, K, L, M, N, O}
define_open! {A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P}
define_open!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
define_open!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);

/// `Fetch`/`Read`/`Write`/etc. all implement `Deref`/`DerefMut` but Rust does
/// not implicitly dereference the wrapper type when we are joining which
/// creates annoying scenarios like `&*entities` where we have to reborrow the
/// type unnecessarily.
///
/// So instead, we implement `Join` on the wrapper types and forward the
/// implementations to the underlying types so that Rust doesn't have to do
/// implicit magic to figure out what we want to do with the type.
macro_rules! immutable_resource_join {
    ($($ty:ty),*) => {
        $(
        // SAFETY: Since `T` implements `LendJoin` it is safe to deref and defer
        // to its implementation.
        #[nougat::gat]
        unsafe impl<'a, 'b, T> LendJoin for &'a $ty
        where
            &'a T: LendJoin,
            T: Resource,
        {
            type Type<'next> = <&'a T as LendJoin>::Type<'next>;
            type Value = <&'a T as LendJoin>::Value;
            type Mask = <&'a T as LendJoin>::Mask;

            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                // SAFETY: This only wraps `T` and, while exposing the mask and
                // the values, requires the same invariants as the original
                // implementation and is thus safe.
                unsafe { self.deref().open() }
            }

            unsafe fn get<'next>(v: &'next mut Self::Value, i: Index) -> Self::Type<'next>
            where
                Self: 'next,
            {
                // SAFETY: The mask of `Self` and `T` are identical, thus a
                // check to `Self`'s mask (which is required) is equal to a
                // check of `T`'s mask, which makes `get` safe to call.
                // Requirement to not call with the same ID more than once
                // (unless `RepeatableLendGet` is implemented) is passed to the
                // caller.
                unsafe { <&'a T as LendJoin>::get(v, i) }
            }

            #[inline]
            fn is_unconstrained() -> bool {
                <&'a T as LendJoin>::is_unconstrained()
            }
        }

        // SAFETY: <&'a $ty as LendJoin>::get does not rely on only being called
        // once with a particular ID as long as `&'a T` does not rely on this.
        unsafe impl<'a, 'b, T> RepeatableLendGet for &'a $ty
        where
            &'a T: RepeatableLendGet,
            T: Resource,
        {}

        // SAFETY: Since `T` implements `Join` it is safe to deref and defer to
        // its implementation.
        unsafe impl<'a, 'b, T> Join for &'a $ty
        where
            &'a T: Join,
            T: Resource,
        {
            type Type = <&'a T as Join>::Type;
            type Value = <&'a T as Join>::Value;
            type Mask = <&'a T as Join>::Mask;

            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                // SAFETY: This only wraps `T` and, while exposing the mask and
                // the values, requires the same invariants as the original
                // implementation and is thus safe.
                unsafe { self.deref().open() }
            }

            unsafe fn get(v: &mut Self::Value, i: Index) -> Self::Type {
                // SAFETY: The mask of `Self` and `T` are identical, thus a
                // check to `Self`'s mask (which is required) is equal to a
                // check of `T`'s mask, which makes `get` safe to call.
                // Requirement to not use the same ID multiple times is passed
                // to the caller.
                unsafe { <&'a T as Join>::get(v, i) }
            }

            #[inline]
            fn is_unconstrained() -> bool {
                <&'a T as Join>::is_unconstrained()
            }
        }

        // SAFETY: Since `T` implements `ParJoin` it is safe to deref and defer to
        // its implementation. S-TODO we can rely on errors if $ty is not sync?
        #[cfg(feature = "parallel")]
        unsafe impl<'a, 'b, T> ParJoin for &'a $ty
        where
            &'a T: ParJoin,
            T: Resource,
        {
            type Type = <&'a T as ParJoin>::Type;
            type Value = <&'a T as ParJoin>::Value;
            type Mask = <&'a T as ParJoin>::Mask;

            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                // SAFETY: This only wraps `T` and, while exposing the mask and
                // the values, requires the same invariants as the original
                // implementation and is thus safe.
                unsafe { self.deref().open() }
            }

            unsafe fn get(v: &Self::Value, i: Index) -> Self::Type {
                // SAFETY: The mask of `Self` and `T` are identical, thus a
                // check to `Self`'s mask (which is required) is equal to a
                // check of `T`'s mask, which makes `get` safe to call.
                unsafe { <&'a T as ParJoin>::get(v, i) }
            }

            #[inline]
            fn is_unconstrained() -> bool {
                <&'a T as ParJoin>::is_unconstrained()
            }
        }
        )*
    };
}

macro_rules! mutable_resource_join {
    ($($ty:ty),*) => {
        $(
        // SAFETY: Since `T` implements `LendJoin` it is safe to deref and defer
        // to its implementation.
        #[nougat::gat]
        unsafe impl<'a, 'b, T> LendJoin for &'a mut $ty
        where
            &'a mut T: LendJoin,
            T: Resource,
        {
            type Type<'next> = <&'a mut T as LendJoin>::Type<'next>;
            type Value = <&'a mut T as LendJoin>::Value;
            type Mask = <&'a mut T as LendJoin>::Mask;

            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                // SAFETY: This only wraps `T` and, while exposing the mask and
                // the values, requires the same invariants as the original
                // implementation and is thus safe.
                unsafe { self.deref_mut().open() }
            }

            unsafe fn get<'next>(v: &'next mut Self::Value, i: Index) -> Self::Type<'next>
            where
                Self: 'next,
            {
                // SAFETY: The mask of `Self` and `T` are identical, thus a
                // check to `Self`'s mask (which is required) is equal to a
                // check of `T`'s mask, which makes `get_mut` safe to call.
                // Requirement to not call with the same ID more than once
                // (unless `RepeatableLendGet` is implemented) is passed to the
                // caller.
                unsafe { <&'a mut T as LendJoin>::get(v, i) }
            }

            #[inline]
            fn is_unconstrained() -> bool {
                <&'a mut T as LendJoin>::is_unconstrained()
            }
        }

        // SAFETY: <&'a mut $ty as LendJoin>::get does not rely on only being
        // called once with a particular ID as long as `&'a mut T` does not rely
        // on this.
        unsafe impl<'a, 'b, T> RepeatableLendGet for &'a mut $ty
        where
            &'a mut T: RepeatableLendGet,
            T: Resource,
        {}

        // SAFETY: Since `T` implements `Join` it is safe to deref and defer to
        // its implementation.
        unsafe impl<'a, 'b, T> Join for &'a mut $ty
        where
            &'a mut T: Join,
            T: Resource,
        {
            type Type = <&'a mut T as Join>::Type;
            type Value = <&'a mut T as Join>::Value;
            type Mask = <&'a mut T as Join>::Mask;

            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                // SAFETY: This only wraps `T` and, while exposing the mask and
                // the values, requires the same invariants as the original
                // implementation and is thus safe.
                unsafe { self.deref_mut().open() }
            }

            unsafe fn get(v: &mut Self::Value, i: Index) -> Self::Type {
                // SAFETY: The mask of `Self` and `T` are identical, thus a
                // check to `Self`'s mask (which is required) is equal to a
                // check of `T`'s mask, which makes `get_mut` safe to call.
                // Requirement to not use the same ID multiple times is passed
                // to the caller.
                unsafe { <&'a mut T as Join>::get(v, i) }
            }

            #[inline]
            fn is_unconstrained() -> bool {
                <&'a mut T as Join>::is_unconstrained()
            }
        }

        // SAFETY: Since `T` implements `ParJoin` it is safe to deref and defer
        // its implementation. S-TODO we can rely on errors if $ty is not sync?
        #[cfg(feature = "parallel")]
        unsafe impl<'a, 'b, T> ParJoin for &'a mut $ty
        where
            &'a mut T: ParJoin,
            T: Resource,
        {
            type Type = <&'a mut T as ParJoin>::Type;
            type Value = <&'a mut T as ParJoin>::Value;
            type Mask = <&'a mut T as ParJoin>::Mask;

            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                // SAFETY: This only wraps `T` and, while exposing the mask and
                // the values, requires the same invariants as the original
                // implementation and is thus safe.
                unsafe { self.deref_mut().open() }
            }

            unsafe fn get(v: &Self::Value, i: Index) -> Self::Type {
                // SAFETY: The mask of `Self` and `T` are identical, thus a check to
                // `Self`'s mask (which is required) is equal to a check of `T`'s
                // mask, which makes `get_mut` safe to call.
                unsafe { <&'a mut T as ParJoin>::get(v, i) }
            }

            #[inline]
            fn is_unconstrained() -> bool {
                <&'a mut T as ParJoin>::is_unconstrained()
            }
        }
        )*
    };
}

immutable_resource_join!(Fetch<'b, T>, Read<'b, T>, ReadExpect<'b, T>);
mutable_resource_join!(FetchMut<'b, T>, Write<'b, T>, WriteExpect<'b, T>);
