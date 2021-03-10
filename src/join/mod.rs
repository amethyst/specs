//! Joining of components for iteration over entities with specific components.

use hibitset::{BitIter, BitSetAll, BitSetAnd, BitSetLike};
use shred::{Fetch, FetchMut, Read, ReadExpect, Resource, Write, WriteExpect};
use std::ops::{Deref, DerefMut};
use tuple_utils::Split;

use crate::world::{Entities, Entity, Index};

#[cfg(feature = "parallel")]
mod par_join;

#[cfg(feature = "parallel")]
pub use self::par_join::{JoinParIter, ParJoin};

/// `BitAnd` is a helper method to & bitsets together resulting in a tree.
pub trait BitAnd {
    /// The combined bitsets.
    type Value: BitSetLike;
    /// Combines `Self` into a single `BitSetLike` through `BitSetAnd`.
    fn and(self) -> Self::Value;
}

/// This needs to be special cased
impl<A> BitAnd for (A,)
where
    A: BitSetLike,
{
    type Value = A;

    fn and(self) -> Self::Value {
        self.0
    }
}

macro_rules! bitset_and {
    // use variables to indicate the arity of the tuple
    ($($from:ident),*) => {
        impl<$($from),*> BitAnd for ($($from),*)
            where $($from: BitSetLike),*
        {
            type Value = BitSetAnd<
                <<Self as Split>::Left as BitAnd>::Value,
                <<Self as Split>::Right as BitAnd>::Value
            >;

            fn and(self) -> Self::Value {
                let (l, r) = self.split();
                BitSetAnd(l.and(), r.and())
            }
        }
    }
}

bitset_and! {A, B}
bitset_and! {A, B, C}
bitset_and! {A, B, C, D}
bitset_and! {A, B, C, D, E}
bitset_and! {A, B, C, D, E, F}
bitset_and! {A, B, C, D, E, F, G}
bitset_and! {A, B, C, D, E, F, G, H}
bitset_and! {A, B, C, D, E, F, G, H, I}
bitset_and! {A, B, C, D, E, F, G, H, I, J}
bitset_and! {A, B, C, D, E, F, G, H, I, J, K}
bitset_and! {A, B, C, D, E, F, G, H, I, J, K, L}
bitset_and! {A, B, C, D, E, F, G, H, I, J, K, L, M}
bitset_and! {A, B, C, D, E, F, G, H, I, J, K, L, M, N}
bitset_and! {A, B, C, D, E, F, G, H, I, J, K, L, M, N, O}
bitset_and! {A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P}

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
pub trait Join {
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

    /// Returns a `Join`-able structure that yields all indices, returning
    /// `None` for all missing elements and `Some(T)` for found elements.
    ///
    /// WARNING: Do not have a join of only `MaybeJoin`s. Otherwise the join
    /// will iterate over every single index of the bitset. If you want a
    /// join with all `MaybeJoin`s, add an `EntitiesRes` to the join as well
    /// to bound the join to all entities that are alive.
    ///
    /// ```
    /// # use specs::prelude::*;
    /// # #[derive(Debug, PartialEq)]
    /// # struct Pos { x: i32, y: i32 } impl Component for Pos { type Storage = VecStorage<Self>; }
    /// # #[derive(Debug, PartialEq)]
    /// # struct Vel { x: i32, y: i32 } impl Component for Vel { type Storage = VecStorage<Self>; }
    /// struct ExampleSystem;
    /// impl<'a> System<'a> for ExampleSystem {
    ///     type SystemData = (
    ///         WriteStorage<'a, Pos>,
    ///         ReadStorage<'a, Vel>,
    ///     );
    ///     fn run(&mut self, (mut positions, velocities): Self::SystemData) {
    ///         for (mut position, maybe_velocity) in (&mut positions, velocities.maybe()).join() {
    ///             if let Some(velocity) = maybe_velocity {
    ///                 position.x += velocity.x;
    ///                 position.y += velocity.y;
    ///             }
    ///         }
    ///     }
    /// }
    ///
    /// fn main() {
    ///     let mut world = World::new();
    ///     let mut dispatcher = DispatcherBuilder::new()
    ///         .with(ExampleSystem, "example_system", &[])
    ///         .build();
    ///
    ///     dispatcher.setup(&mut world);
    ///
    ///     let e1 = world.create_entity()
    ///         .with(Pos { x: 0, y: 0 })
    ///         .with(Vel { x: 5, y: 2 })
    ///         .build();
    ///
    ///     let e2 = world.create_entity()
    ///         .with(Pos { x: 0, y: 0 })
    ///         .build();
    ///
    ///     dispatcher.dispatch(&mut world);
    ///
    ///     let positions = world.read_storage::<Pos>();
    ///     assert_eq!(positions.get(e1), Some(&Pos { x: 5, y: 2 }));
    ///     assert_eq!(positions.get(e2), Some(&Pos { x: 0, y: 0 }));
    /// }
    /// ```
    fn maybe(self) -> MaybeJoin<Self>
    where
        Self: Sized,
    {
        MaybeJoin(self)
    }

    /// Open this join by returning the mask and the storages.
    ///
    /// # Safety
    ///
    /// This is unsafe because implementations of this trait can permit
    /// the `Value` to be mutated independently of the `Mask`.
    /// If the `Mask` does not correctly report the status of the `Value`
    /// then illegal memory access can occur.
    unsafe fn open(self) -> (Self::Mask, Self::Value);

    /// Get a joined component value by a given index.
    ///
    /// # Safety
    ///
    /// * A call to `get` must be preceded by a check if `id` is part of
    ///   `Self::Mask`
    /// * The implementation of this method may use unsafe code, but has no
    ///   invariants to meet
    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type;

    /// If this `Join` typically returns all indices in the mask, then iterating
    /// over only it or combined with other joins that are also dangerous
    /// will cause the `JoinIter`/`ParJoin` to go through all indices which
    /// is usually not what is wanted and will kill performance.
    #[inline]
    fn is_unconstrained() -> bool {
        false
    }
}

/// A `Join`-able structure that yields all indices, returning `None` for all
/// missing elements and `Some(T)` for found elements.
///
/// For usage see [`Join::maybe()`].
///
/// WARNING: Do not have a join of only `MaybeJoin`s. Otherwise the join will
/// iterate over every single index of the bitset. If you want a join with
/// all `MaybeJoin`s, add an `EntitiesRes` to the join as well to bound the
/// join to all entities that are alive.
///
/// [`Join::maybe()`]: ../join/trait.Join.html#method.maybe
pub struct MaybeJoin<J: Join>(pub J);

impl<T> Join for MaybeJoin<T>
where
    T: Join,
{
    type Mask = BitSetAll;
    type Type = Option<<T as Join>::Type>;
    type Value = (<T as Join>::Mask, <T as Join>::Value);

    // SAFETY: This wraps another implementation of `open`, making it dependent on
    // `J`'s correctness. We can safely assume `J` is valid, thus this must be
    // valid, too. No invariants to meet.
    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        let (mask, value) = self.0.open();
        (BitSetAll, (mask, value))
    }

    // SAFETY: No invariants to meet and the unsafe code checks the mask, thus
    // fulfills the requirements for calling `get`
    unsafe fn get((mask, value): &mut Self::Value, id: Index) -> Self::Type {
        if mask.contains(id) {
            Some(<T as Join>::get(value, id))
        } else {
            None
        }
    }

    #[inline]
    fn is_unconstrained() -> bool {
        true
    }
}

// SAFETY: This is safe as long as `T` implements `ParJoin` safely.  `MaybeJoin`
// relies on `T as Join` for all storage access and safely wraps the inner
// `Join` API, so it should also be able to implement `ParJoin`.
#[cfg(feature = "parallel")]
unsafe impl<T> ParJoin for MaybeJoin<T> where T: ParJoin {}

/// `JoinIter` is an `Iterator` over a group of `Storages`.
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
                "`Join` possibly iterating through all indices, you might've made a join with all `MaybeJoin`s, which is unbounded in length."
            );
        }

        // SAFETY: We do not swap out the mask or the values, nor do we allow it by
        // exposing them.
        let (keys, values) = unsafe { j.open() };
        JoinIter {
            keys: keys.iter(),
            values,
        }
    }
}

impl<J: Join> JoinIter<J> {
    /// Allows getting joined values for specific entity.
    ///
    /// ## Example
    ///
    /// ```
    /// # use specs::prelude::*;
    /// # #[derive(Debug, PartialEq)]
    /// # struct Pos; impl Component for Pos { type Storage = VecStorage<Self>; }
    /// # #[derive(Debug, PartialEq)]
    /// # struct Vel; impl Component for Vel { type Storage = VecStorage<Self>; }
    /// let mut world = World::new();
    ///
    /// world.register::<Pos>();
    /// world.register::<Vel>();
    ///
    /// // This entity could be stashed anywhere (into `Component`, `Resource`, `System`s data, etc.) as it's just a number.
    /// let entity = world
    ///     .create_entity()
    ///     .with(Pos)
    ///     .with(Vel)
    ///     .build();
    ///
    /// // Later
    /// {
    ///     let mut pos = world.write_storage::<Pos>();
    ///     let vel = world.read_storage::<Vel>();
    ///
    ///     assert_eq!(
    ///         Some((&mut Pos, &Vel)),
    ///         (&mut pos, &vel).join().get(entity, &world.entities()),
    ///         "The entity that was stashed still has the needed components and is alive."
    ///     );
    /// }
    ///
    /// // The entity has found nice spot and doesn't need to move anymore.
    /// world.write_storage::<Vel>().remove(entity);
    ///
    /// // Even later
    /// {
    ///     let mut pos = world.write_storage::<Pos>();
    ///     let vel = world.read_storage::<Vel>();
    ///
    ///     assert_eq!(
    ///         None,
    ///         (&mut pos, &vel).join().get(entity, &world.entities()),
    ///         "The entity doesn't have velocity anymore."
    ///     );
    /// }
    /// ```
    pub fn get(&mut self, entity: Entity, entities: &Entities) -> Option<J::Type> {
        if self.keys.contains(entity.id()) && entities.is_alive(entity) {
            // SAFETY: the mask (`keys`) is checked as specified in the docs of `get`.
            Some(unsafe { J::get(&mut self.values, entity.id()) })
        } else {
            None
        }
    }

    /// Allows getting joined values for specific raw index.
    ///
    /// The raw index for an `Entity` can be retrieved using `Entity::id`
    /// method.
    ///
    /// As this method operates on raw indices, there is no check to see if the
    /// entity is still alive, so the caller should ensure it instead.
    pub fn get_unchecked(&mut self, index: Index) -> Option<J::Type> {
        if self.keys.contains(index) {
            // SAFETY: the mask (`keys`) is checked as specified in the docs of `get`.
            Some(unsafe { J::get(&mut self.values, index) })
        } else {
            None
        }
    }
}

impl<J: Join> std::iter::Iterator for JoinIter<J> {
    type Item = J::Type;

    fn next(&mut self) -> Option<J::Type> {
        // SAFETY: since `idx` is yielded from `keys` (the mask), it is necessarily a
        // part of it. Thus, requirements are fulfilled for calling `get`.
        self.keys
            .next()
            .map(|idx| unsafe { J::get(&mut self.values, idx) })
    }
}

/// Clones the `JoinIter`.
///
/// # Examples
///
/// ```
/// # use specs::prelude::*;
/// # #[derive(Debug)]
/// # struct Position; impl Component for Position { type Storage = VecStorage<Self>; }
/// # #[derive(Debug)]
/// # struct Collider; impl Component for Collider { type Storage = VecStorage<Self>; }
/// let mut world = World::new();
///
/// world.register::<Position>();
/// world.register::<Collider>();
///
/// // add some entities to our world
/// for _ in 0..10 {
///     let entity = world
///         .create_entity()
///         .with(Position)
///         .with(Collider)
///         .build();
/// }
///
/// // check for collisions between entities
/// let positions = world.read_storage::<Position>();
/// let colliders = world.read_storage::<Collider>();
///
/// let mut join_iter = (&positions, &colliders).join();
/// while let Some(a) = join_iter.next() {
///     for b in join_iter.clone() {
///         # let check_collision = |a, b| true;
///         if check_collision(a, b) {
///             // do stuff
///         }
///     }
/// }
/// ```
///
/// It is *not* possible to clone a `JoinIter` which allows for
/// mutation of its content, as this would lead to shared mutable
/// access.
///
/// ```compile_fail
/// # use specs::prelude::*;
/// # #[derive(Debug)]
/// # struct Position; impl Component for Position { type Storage = VecStorage<Self>; }
/// # let mut world = World::new();
/// # world.register::<Position>();
/// # let entity = world.create_entity().with(Position).build();
/// // .. previous example
///
/// let mut positions = world.write_storage::<Position>();
///
/// let mut join_iter = (&mut positions).join();
/// // this must not compile, as the following line would cause
/// // undefined behavior!
/// let mut cloned_iter = join_iter.clone();
/// let (mut alias_one, mut alias_two) = (join_iter.next(), cloned_iter.next());
/// ```
impl<J: Join> Clone for JoinIter<J>
where
    J::Mask: Clone,
    J::Value: Clone,
{
    fn clone(&self) -> Self {
        Self {
            keys: self.keys.clone(),
            values: self.values.clone(),
        }
    }
}

macro_rules! define_open {
    // use variables to indicate the arity of the tuple
    ($($from:ident),*) => {
        impl<$($from,)*> Join for ($($from),*,)
            where $($from: Join),*,
                  ($(<$from as Join>::Mask,)*): BitAnd,
        {
            type Type = ($($from::Type),*,);
            type Value = ($($from::Value),*,);
            type Mask = <($($from::Mask,)*) as BitAnd>::Value;
            #[allow(non_snake_case)]

            // SAFETY: While we do expose the mask and the values and therefore would allow swapping them,
            // this method is `unsafe` and relies on the same invariants.
            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                let ($($from,)*) = self;
                let ($($from,)*) = ($($from.open(),)*);
                (
                    ($($from.0),*,).and(),
                    ($($from.1),*,)
                )
            }

            // SAFETY: No invariants to meet and `get` is safe to call as the caller must have checked the mask,
            // which only has a key that exists in all of the storages.
            #[allow(non_snake_case)]
            unsafe fn get(v: &mut Self::Value, i: Index) -> Self::Type {
                let &mut ($(ref mut $from,)*) = v;
                ($($from::get($from, i),)*)
            }

            #[inline]
            fn is_unconstrained() -> bool {
                let mut unconstrained = true;
                $( unconstrained = unconstrained && $from::is_unconstrained(); )*
                unconstrained
            }
        }

        // SAFETY: This is safe to implement since all components implement `ParJoin`.
        // If the access of every individual `get` leads to disjoint memory access, calling
        // all of them after another does in no case lead to access of common memory.
        #[cfg(feature = "parallel")]
        unsafe impl<$($from,)*> ParJoin for ($($from),*,)
            where $($from: ParJoin),*,
                  ($(<$from as Join>::Mask,)*): BitAnd,
        {}

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
        impl<'a, 'b, T> Join for &'a $ty
        where
            &'a T: Join,
            T: Resource,
        {
            type Type = <&'a T as Join>::Type;
            type Value = <&'a T as Join>::Value;
            type Mask = <&'a T as Join>::Mask;

            // SAFETY: This only wraps `T` and, while exposing the mask and the values,
            // requires the same invariants as the original implementation and is thus safe.
            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                self.deref().open()
            }

            // SAFETY: The mask of `Self` and `T` are identical, thus a check to `Self`'s mask (which is required)
            // is equal to a check of `T`'s mask, which makes `get` safe to call.
            unsafe fn get(v: &mut Self::Value, i: Index) -> Self::Type {
                <&'a T as Join>::get(v, i)
            }

            #[inline]
            fn is_unconstrained() -> bool {
                <&'a T as Join>::is_unconstrained()
            }
        }

        // SAFETY: This is just a wrapper of `T`'s implementation for `ParJoin` and can
        // in no case lead to other memory access patterns.
        #[cfg(feature = "parallel")]
        unsafe impl<'a, 'b, T> ParJoin for &'a $ty
        where
            &'a T: ParJoin,
            T: Resource
        {}
        )*
    };
}

macro_rules! mutable_resource_join {
    ($($ty:ty),*) => {
        $(
        impl<'a, 'b, T> Join for &'a mut $ty
        where
            &'a mut T: Join,
            T: Resource,
        {
            type Type = <&'a mut T as Join>::Type;
            type Value = <&'a mut T as Join>::Value;
            type Mask = <&'a mut T as Join>::Mask;

            // SAFETY: This only wraps `T` and, while exposing the mask and the values,
            // requires the same invariants as the original implementation and is thus safe.
            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                self.deref_mut().open()
            }

            // SAFETY: The mask of `Self` and `T` are identical, thus a check to `Self`'s mask (which is required)
            // is equal to a check of `T`'s mask, which makes `get_mut` safe to call.
            unsafe fn get(v: &mut Self::Value, i: Index) -> Self::Type {
                <&'a mut T as Join>::get(v, i)
            }

            #[inline]
            fn is_unconstrained() -> bool {
                <&'a mut T as Join>::is_unconstrained()
            }
        }

        // SAFETY: This is just a wrapper of `T`'s implementation for `ParJoin` and can
        // in no case lead to other memory access patterns.
        #[cfg(feature = "parallel")]
        unsafe impl<'a, 'b, T> ParJoin for &'a mut $ty
        where
            &'a mut T: ParJoin,
            T: Resource
        {}
        )*
    };
}

immutable_resource_join!(Fetch<'b, T>, Read<'b, T>, ReadExpect<'b, T>);
mutable_resource_join!(FetchMut<'b, T>, Write<'b, T>, WriteExpect<'b, T>);
