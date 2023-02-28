use super::MaybeJoin;
use hibitset::{BitIter, BitSetLike};

use crate::world::{Entities, Entity, Index};

/// Like the [`Join`](super::Join) trait except this is similar to a [lending
/// iterator](https://blog.rust-lang.org/2021/08/03/GATs-stabilization-push.html#so-what-are-gats)
/// in that only one item can be accessed at once.
///
/// The type returned from [`.lend_join()`](LendJoin::lend_join),
/// [`JoinLendIter`] does not implement `Iterator` like
/// [`JoinIter`](super::JoinIter) does. Instead, it provides a
/// [`next`](JoinLendIter::next) method that exclusively borrows the
/// `JoinLendIter` for the lifetime of the returned value.
///
/// This limitation allows freedom for more patterns to be soundly implemented.
/// Thus, `LendJoin` acts as the "lowest common denominator" of the
/// `Join`-like traits (i.e. if something can implement `Join` it can also
/// implement `LendJoin`).
///
/// In particular, [`Entries`](crate::storage::Entries) only implements
/// `LendJoin`. As another example,
/// [`RestrictedStorage`](crate::storage::RestrictedStorage) implements both
/// `Join` and `LendJoin`. However, for joining mutably, lend join variant
/// produces
/// [`PairedStorageWriteExclusive`](crate::storage::PairedStorageWriteExclusive)
/// values which have `get_other`/`get_other_mut` methods that aren't provided
/// by [`PairedStorageWriteShared`](crate::storage::PairedStorageWriteShared).
///
/// Finally, these limitations allow providing the [`JoinLendIter::get`] method
/// which can be useful to get a set of components from an entity without
/// calling `get` individually on each storage (see the example in that method's
/// docs).
///
/// Also see the `lend_join` example.
///
/// # Safety
///
/// The `Self::Mask` value returned with the `Self::Value` must correspond such
/// that it is safe to retrieve items from `Self::Value` whose presence is
/// indicated in the mask. As part of this, `BitSetLike::iter` must not produce
/// an iterator that repeats an `Index` value if the `LendJoin::get` impl relies
/// on not being called twice with the same `Index`.
#[nougat::gat]
pub unsafe trait LendJoin {
    /// Type of joined components.
    ///
    /// # Note
    ///
    /// This type is using macro magic to emulate GATs on stable. So to refer to
    /// it you need to use the [`LendJoinType<'next, J>`](LendJoinType) type
    /// alias.
    type Type<'next>
    where
        Self: 'next;
    /// Type of joined storages.
    type Value;
    /// Type of joined bit mask.
    type Mask: BitSetLike;

    /// Create a joined lending iterator over the contents.
    fn lend_join(self) -> JoinLendIter<Self>
    where
        Self: Sized,
    {
        JoinLendIter::new(self)
    }

    /// Returns a structure that implements `Join`/`LendJoin`/`MaybeJoin` if the
    /// contained `T` does and that yields all indices, returning `None` for all
    /// missing elements and `Some(T)` for found elements.
    ///
    /// To join over and optional component mutably this pattern can be used:
    /// `(&mut storage).maybe()`.
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
    ///         let mut join = (&mut positions, velocities.maybe()).lend_join();
    ///         while let Some ((mut position, maybe_velocity)) = join.next() {
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
    /// This is unsafe because implementations of this trait can permit the
    /// `Value` to be mutated independently of the `Mask`. If the `Mask` does
    /// not correctly report the status of the `Value` then illegal memory
    /// access can occur.
    unsafe fn open(self) -> (Self::Mask, Self::Value);

    /// Get a joined component value by a given index.
    ///
    /// # Safety
    ///
    /// * A call to `get` must be preceded by a check if `id` is part of
    ///   `Self::Mask`
    /// * Multiple calls with the same `id` are not allowed, for a particular
    ///   instance of the values from [`open`](LendJoin::open). Unless this type
    ///   implements the unsafe trait [`RepeatableLendGet`].
    unsafe fn get<'next>(value: &'next mut Self::Value, id: Index) -> Self::Type<'next>
    where
        Self: 'next;

    /// If this `LendJoin` typically returns all indices in the mask, then
    /// iterating over only it or combined with other joins that are also
    /// dangerous will cause the `JoinLendIter` to go through all indices which
    /// is usually not what is wanted and will kill performance.
    #[inline]
    fn is_unconstrained() -> bool {
        false
    }
}

/// # Safety
///
/// Implementing this trait guarantees that `<Self as LendJoin>::get` can
/// soundly be called multiple times with the same ID.
pub unsafe trait RepeatableLendGet: LendJoin {}

/// Type alias to refer to the `<J as LendJoin>::Type<'next>` (except this
/// doesn't actually exist in this form so the `nougat::Gat!` macro is needed).
pub type LendJoinType<'next, J> = nougat::Gat!(<J as LendJoin>::Type<'next>);

/// `JoinLendIter` is an is a lending/streaming iterator over components from a
/// group of storages.
#[must_use]
pub struct JoinLendIter<J: LendJoin> {
    keys: BitIter<J::Mask>,
    values: J::Value,
}

impl<J: LendJoin> JoinLendIter<J> {
    /// Create a new lending join iterator.
    pub fn new(j: J) -> Self {
        if <J as LendJoin>::is_unconstrained() {
            log::warn!(
                "`LendJoin` possibly iterating through all indices, \
                you might've made a join with all `MaybeJoin`s, \
                which is unbounded in length."
            );
        }

        // SAFETY: We do not swap out the mask or the values, nor do we allow it
        // by exposing them.
        let (keys, values) = unsafe { j.open() };
        JoinLendIter {
            keys: keys.iter(),
            values,
        }
    }
}

impl<J: LendJoin> JoinLendIter<J> {
    /// Lending `next`.
    ///
    /// Can be used to iterate with this pattern:
    ///
    /// `while let Some(components) = join_lending_iter.next() {`
    #[allow(clippy::should_implement_trait)] // we want this to look like iterator
    pub fn next(&mut self) -> Option<LendJoinType<'_, J>> {
        // SAFETY: Since `idx` is yielded from `keys` (the mask), it is
        // necessarily a part of it. `LendJoin` requires that the iterator
        // doesn't repeat indices and we advance the iterator for each `get`
        // call in all methods that don't require `RepeatableLendGet`.
        self.keys
            .next()
            .map(|idx| unsafe { J::get(&mut self.values, idx) })
    }

    /// Calls a closure on each entity in the join.
    pub fn for_each(mut self, mut f: impl FnMut(LendJoinType<'_, J>)) {
        self.keys.for_each(|idx| {
            // SAFETY: Since `idx` is yielded from `keys` (the mask), it is
            // necessarily a part of it. `LendJoin` requires that the iterator
            // doesn't repeat indices and we advance the iterator for each `get`
            // call in all methods that don't require `RepeatableLendGet`.
            let item = unsafe { J::get(&mut self.values, idx) };
            f(item);
        })
    }

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
    ///         (&mut pos, &vel).lend_join().get(entity, &world.entities()),
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
    ///         (&mut pos, &vel).lend_join().get(entity, &world.entities()),
    ///         "The entity doesn't have velocity anymore."
    ///     );
    /// }
    /// ```
    pub fn get(&mut self, entity: Entity, entities: &Entities) -> Option<LendJoinType<'_, J>>
    where
        J: RepeatableLendGet,
    {
        if self.keys.contains(entity.id()) && entities.is_alive(entity) {
            // SAFETY: the mask (`keys`) is checked as specified in the docs of
            // `get`. We require `J: RepeatableJoinGet` so this can be safely
            // called multiple time with the same ID.
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
    ///
    /// Note: Not checking is still sound (thus this method is safe to call),
    /// but this can return data from deleted entities!
    pub fn get_unchecked(&mut self, index: Index) -> Option<LendJoinType<'_, J>>
    where
        J: RepeatableLendGet,
    {
        if self.keys.contains(index) {
            // SAFETY: the mask (`keys`) is checked as specified in the docs of
            // `get`. We require `J: RepeatableJoinGet` so this can be safely
            // called multiple time with the same ID.
            Some(unsafe { J::get(&mut self.values, index) })
        } else {
            None
        }
    }
}
