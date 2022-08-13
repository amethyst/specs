use std::cell::UnsafeCell;

use hibitset::{BitProducer, BitSetLike};

use crate::join::Join;
use rayon::iter::{
    plumbing::{bridge_unindexed, Folder, UnindexedConsumer, UnindexedProducer},
    ParallelIterator,
};

/// The purpose of the `ParJoin` trait is to provide a way
/// to access multiple storages in parallel at the same time with
/// the merged bit set.
///
/// # Safety
///
/// The implementation of `ParallelIterator` for `ParJoin` makes multiple
/// assumptions on the structure of `Self`. In particular, `ParJoin::get` must
/// be callable from multiple threads, simultaneously, without creating mutable
/// references not exclusively associated with `id`.
///
/// The `Self::Mask` value returned with the `Self::Value` must correspond such
/// that it is safe to retrieve items from `Self::Value` whose presence is
/// indicated in the mask.
pub unsafe trait ParJoin {
    /// Type of joined components.
    type Type;
    /// Type of joined storages.
    type Value;
    /// Type of joined bit mask.
    type Mask: BitSetLike;

    /// Create a joined parallel iterator over the contents.
    fn par_join(self) -> JoinParIter<Self>
    where
        Self: Sized,
    {
        if Self::is_unconstrained() {
            log::warn!(
                "`ParJoin` possibly iterating through all indices, \
                you might've made a join with all `MaybeJoin`s, \
                which is unbounded in length."
            );
        }

        JoinParIter(self)
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
    ///   `Self::Mask`.
    /// * The use of the mutable reference returned from this method must end
    ///   before subsequent calls with the same `id`.
    unsafe fn get(value: &Self::Value, id: Index) -> Self::Type;

    /// If this `LendJoin` typically returns all indices in the mask, then
    /// iterating over only it or combined with other joins that are also
    /// dangerous will cause the `JoinLendIter` to go through all indices which
    /// is usually not what is wanted and will kill performance.
    #[inline]
    fn is_unconstrained() -> bool {
        false
    }
}

/// `JoinParIter` is a `ParallelIterator` over a group of storages.
#[must_use]
pub struct JoinParIter<J>(J);

impl<J> ParallelIterator for JoinParIter<J>
where
    J: ParJoin + Send,
    J::Mask: Send + Sync,
    J::Type: Send,
    J::Value: Send,
{
    type Item = J::Type;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        // SAFETY: `keys` and `values` are not exposed outside this module and
        // we only use `values` for calling `ParJoin::get`.
        let (keys, values) = unsafe { self.0.open() };
        // Create a bit producer which splits on up to three levels
        let producer = BitProducer((&keys).iter(), 3);

        bridge_unindexed(JoinProducer::<J>::new(producer, &values), consumer)
    }
}

struct JoinProducer<'a, J>
where
    J: ParJoin + Send,
    J::Mask: Send + Sync + 'a,
    J::Type: Send,
    J::Value: Send + 'a,
{
    keys: BitProducer<'a, J::Mask>,
    values: &'a J::Value,
}

impl<'a, J> JoinProducer<'a, J>
where
    J: ParJoin + Send,
    J::Type: Send,
    J::Value: 'a + Send,
    J::Mask: 'a + Send + Sync,
{
    fn new(keys: BitProducer<'a, J::Mask>, values: &'a J::Value) -> Self {
        JoinProducer { keys, values }
    }
}

impl<'a, J> UnindexedProducer for JoinProducer<'a, J>
where
    J: ParJoin + Send,
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
        // SAFETY: `idx` is obtained from the `Mask` returned by
        // `ParJoin::open`. The indices here are guaranteed to be distinct
        // because of the fact that the bit set is split.
        let iter = keys.0.map(|idx| unsafe { J::get(values, idx) });

        folder.consume_iter(iter)
    }
}
