use std;

use hibitset::{BitSetAnd, BitIter, BitSetLike};

#[cfg(feature="parallel")]
use hibitset::BitParIter;
#[cfg(feature="parallel")]
use rayon::iter::ParallelIterator;
#[cfg(feature="parallel")]
use rayon::iter::internal::UnindexedConsumer;

use tuple_utils::Split;

use Index;

/// BitAnd is a helper method to & bitsets together resulting in a tree.
pub trait BitAnd {
    type Value: BitSetLike;

    fn and(self) -> Self::Value;
}

/// This needs to be special cased
impl<A> BitAnd for (A,)
    where A: BitSetLike
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
pub trait Join {
    /// Type of joined components.
    type Type;
    /// Type of joined storages.
    type Value;
    /// Type of joined bit mask.
    type Mask: BitSetLike;

    /// Create a joined iterator over the contents.
    fn join(self) -> JoinIter<Self>
        where Self: Sized
    {
        JoinIter::new(self)
    }
    /// Create a joined parallel iterator over the contents.
    #[cfg(feature="parallel")]
    fn par_join(self) -> JoinParIter<Self>
        where Self: Sized
    {
        JoinParIter(self)
    }
    /// Open this join by returning the mask and the storages.
    fn open(self) -> (Self::Mask, Self::Value);

    /// Get a joined component value by a given index.
    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type;
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
        let (keys, values) = j.open();
        JoinIter {
            keys: keys.iter(),
            values: values,
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

#[cfg(feature="parallel")]
pub use self::par_join::*;
/// `JoinParIter` is an `ParallelIterator` over a group of `Storages`.
#[must_use]
#[cfg(feature="parallel")]
pub struct JoinParIter<J: Join>(J);
mod par_join {
    use std::marker::PhantomData;
    use rayon::iter::ParallelIterator;
    use rayon::iter::internal::{UnindexedProducer, UnindexedConsumer, Folder, bridge_unindexed};
    use super::*;
    use hibitset::BitProducer;

    impl<J> ParallelIterator for JoinParIter<J>
    where J: Join + Send,
          J::Type: Send,
          J::Value: Split + Send,
          J::Mask: Send + Sync,
    {
        type Item = J::Type;

        fn drive_unindexed<C>(self, consumer: C) -> C::Result
            where C: UnindexedConsumer<Self::Item>
        {
            let (keys, mut values) = self.0.open();
            bridge_unindexed(JoinProducer::<J>{
                keys: BitProducer((&keys).iter()),
                values: &mut values as *mut _,
                _marker: PhantomData,
            }, consumer)
        }
    }

    struct JoinProducer<'a, 'b, J>
    where J: Join + Send,
          J::Type: Send,
          J::Value: 'a + Send,
          J::Mask: 'b + Send + Sync,
    {
        keys: BitProducer<'b, J::Mask>,
        values: *mut J::Value,
        _marker: PhantomData<&'a J::Value>,
    }

    unsafe impl<'a, 'b, J> Send for JoinProducer<'a, 'b, J>
    where J: Join + Send,
          J::Type: Send,
          J::Value: 'a + Send,
          J::Mask: 'b + Send + Sync,
    {}

    impl<'a, 'b, J> UnindexedProducer for JoinProducer<'a, 'b, J>
    where J: Join + Send,
          J::Type: Send,
          J::Value: 'a + Send,
          J::Mask: 'b + Send + Sync,
    {
        type Item = J::Type;
        fn split(self) -> (Self, Option<Self>) {
            let (cur, other) = self.keys.split();
            if let Some(other) = other {
                (JoinProducer {
                    keys: cur,
                    values: self.values,
                    _marker: PhantomData,
                },
                Some(JoinProducer {
                    keys: other,
                    values: self.values,
                    _marker: PhantomData,
                }))
            } else {
                (JoinProducer {
                    keys: cur,
                    values: self.values,
                    _marker: PhantomData,
                }, None)
            }
        }

        fn fold_with<F>(self, folder: F) -> F
        where F: Folder<Self::Item>
        {
            let JoinProducer {values, keys, ..} = self;
            let iter = keys.0.map(|idx| unsafe {
                // TODO: Figure out is creating mutable reference actually safe.
                // I am pretty sure it isn't, because this there can be multiple
                // mutable references to same memory,
                // but have no idea how else this should be done.
                J::get(&mut *values, idx)
            });
            folder.consume_iter(iter)
        }
    }
}

macro_rules! define_open {
    // use variables to indicate the arity of the tuple
    ($($from:ident),*) => {
        impl<'a, $($from,)*> Join for ($($from),*,)
            where $($from: Join),*,
                  ($(<$from as Join>::Mask,)*): BitAnd,
        {
            type Type = ($($from::Type),*,);
            type Value = ($($from::Value),*,);
            type Mask = <($($from::Mask,)*) as BitAnd>::Value;
            #[allow(non_snake_case)]
            fn open(self) -> (Self::Mask, Self::Value) {
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
