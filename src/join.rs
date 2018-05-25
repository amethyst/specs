//! Joining of components for iteration over entities with specific components.

use std;
use std::cell::UnsafeCell;

use hibitset::{BitIter, BitProducer, BitSetAnd, BitSetLike};
use rayon::iter::ParallelIterator;
use rayon::iter::plumbing::{bridge_unindexed, Folder, UnindexedConsumer, UnindexedProducer};
use tuple_utils::Split;

use world::{Entities, Entity, Index};

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

bitset_and!{A, B}
bitset_and!{A, B, C}
bitset_and!{A, B, C, D}
bitset_and!{A, B, C, D, E}
bitset_and!{A, B, C, D, E, F}
bitset_and!{A, B, C, D, E, F, G}
bitset_and!{A, B, C, D, E, F, G, H}
bitset_and!{A, B, C, D, E, F, G, H, I}
bitset_and!{A, B, C, D, E, F, G, H, I, J}
bitset_and!{A, B, C, D, E, F, G, H, I, J, K}
bitset_and!{A, B, C, D, E, F, G, H, I, J, K, L}
bitset_and!{A, B, C, D, E, F, G, H, I, J, K, L, M}
bitset_and!{A, B, C, D, E, F, G, H, I, J, K, L, M, N}
bitset_and!{A, B, C, D, E, F, G, H, I, J, K, L, M, N, O}
bitset_and!{A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P}

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
/// world
///     .create_entity()
///     .with(Pos)
///     .build();
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
/// let ent = world.create_entity()
///     .with(Pos)
///     .with(Vel)
///     .build();
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
///     let joined: Vec<_> = (&*entities, &pos, &vel).join().collect(); // note the `&*entities`
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

    /// Open this join by returning the mask and the storages.
    ///
    /// This is unsafe because implementations of this trait can permit
    /// the `Value` to be mutated independently of the `Mask`.
    /// If the `Mask` does not correctly report the status of the `Value`
    /// then illegal memory access can occur.
    unsafe fn open(self) -> (Self::Mask, Self::Value);

    /// Get a joined component value by a given index.
    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type;
}

/// The purpose of the `ParJoin` trait is to provide a way
/// to access multiple storages in parallel at the same time with
/// the merged bit set.
pub unsafe trait ParJoin: Join {
    /// Create a joined parallel iterator over the contents.
    fn par_join(self) -> JoinParIter<Self>
    where
        Self: Sized,
    {
        JoinParIter(self)
    }
}

/// `JoinIter` is an `Iterator` over a group of `Storages`.
#[must_use]
pub struct JoinIter<J: Join> {
    keys: BitIter<J::Mask>,
    values: J::Value,
}

impl<J: Join> JoinIter<J> {
    /// Create a new join iterator.
    pub fn new(j: J) -> Self {
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
            Some(unsafe { J::get(&mut self.values, entity.id()) })
        } else {
            None
        }
    }

    /// Allows getting joined values for specific raw index.
    ///
    /// The raw index for an `Entity` can be retrieved using `Entity::id` method.
    ///
    /// As this method operates on raw indices, there is no check to see if the entity is still alive,
    /// so the caller should ensure it instead.
    pub fn get_unchecked(&mut self, index: Index) -> Option<J::Type> {
        if self.keys.contains(index) {
            Some(unsafe { J::get(&mut self.values, index) })
        } else {
            None
        }
    }
}

impl<J: Join> std::iter::Iterator for JoinIter<J> {
    type Item = J::Type;

    fn next(&mut self) -> Option<J::Type> {
        self.keys
            .next()
            .map(|idx| unsafe { J::get(&mut self.values, idx) })
    }
}

/// `JoinParIter` is a `ParallelIterator` over a group of `Storages`.
#[must_use]
pub struct JoinParIter<J>(J);

impl<J> ParallelIterator for JoinParIter<J>
where
    J: Join + Send,
    J::Mask: Send + Sync,
    J::Type: Send,
    J::Value: Send,
{
    type Item = J::Type;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        let (keys, values) = unsafe { self.0.open() };
        // Create a bit producer which splits on up to three levels
        let producer = BitProducer((&keys).iter(), 3);
        let values = UnsafeCell::new(values);

        bridge_unindexed(JoinProducer::<J>::new(producer, &values), consumer)
    }
}

struct JoinProducer<'a, J>
where
    J: Join + Send,
    J::Mask: Send + Sync + 'a,
    J::Type: Send,
    J::Value: Send + 'a,
{
    keys: BitProducer<'a, J::Mask>,
    values: &'a UnsafeCell<J::Value>,
}

impl<'a, J> JoinProducer<'a, J>
where
    J: Join + Send,
    J::Type: Send,
    J::Value: 'a + Send,
    J::Mask: 'a + Send + Sync,
{
    fn new(keys: BitProducer<'a, J::Mask>, values: &'a UnsafeCell<J::Value>) -> Self {
        JoinProducer { keys, values }
    }
}

unsafe impl<'a, J> Send for JoinProducer<'a, J>
where
    J: Join + Send,
    J::Type: Send,
    J::Value: 'a + Send,
    J::Mask: 'a + Send + Sync,
{
}

impl<'a, J> UnindexedProducer for JoinProducer<'a, J>
where
    J: Join + Send,
    J::Type: Send,
    J::Value: 'a + Send,
    J::Mask: 'a + Send + Sync,
{
    type Item = J::Type;
    fn split(self) -> (Self, Option<Self>) {
        let (cur, other) = self.keys.split();
        let values = self.values;
        let first = JoinProducer::new(cur, values);
        let second = other.map(|o| JoinProducer::new(o, values));

        (first, second)
    }

    fn fold_with<F>(self, folder: F) -> F
    where
        F: Folder<Self::Item>,
    {
        let JoinProducer { values, keys, .. } = self;
        let iter = keys.0.map(|idx| unsafe {
            // This unsafe block should be safe if the `J::get`
            // can be safely called from different threads with distinct indices.

            // The indices here are guaranteed to be distinct because of the fact
            // that the bit set is split.
            J::get(&mut *values.get(), idx)
        });

        folder.consume_iter(iter)
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
            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                let ($($from,)*) = self;
                let ($($from,)*) = ($($from.open(),)*);
                (
                    ($($from.0),*,).and(),
                    ($($from.1),*,)
                )
            }

            #[allow(non_snake_case)]
            unsafe fn get(v: &mut Self::Value, i: Index) -> Self::Type {
                let &mut ($(ref mut $from,)*) = v;
                ($($from::get($from, i),)*)
            }
        }
        unsafe impl<$($from,)*> ParJoin for ($($from),*,)
            where $($from: ParJoin),*,
                  ($(<$from as Join>::Mask,)*): BitAnd,
        {}
    }
}

define_open!{A}
define_open!{A, B}
define_open!{A, B, C}
define_open!{A, B, C, D}
define_open!{A, B, C, D, E}
define_open!{A, B, C, D, E, F}
define_open!{A, B, C, D, E, F, G}
define_open!{A, B, C, D, E, F, G, H}
define_open!{A, B, C, D, E, F, G, H, I}
define_open!{A, B, C, D, E, F, G, H, I, J}
define_open!{A, B, C, D, E, F, G, H, I, J, K}
define_open!{A, B, C, D, E, F, G, H, I, J, K, L}
define_open!{A, B, C, D, E, F, G, H, I, J, K, L, M}
define_open!{A, B, C, D, E, F, G, H, I, J, K, L, M, N}
define_open!{A, B, C, D, E, F, G, H, I, J, K, L, M, N, O}
define_open!{A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P}
define_open!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q);
define_open!(A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P, Q, R);
