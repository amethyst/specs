use std;
use tuple_utils::Split;
use bitset::{BitIter, BitSetAnd, BitSetLike};
use Index;


/// BitAnd is a helper method to & bitsets togather resulting in a tree
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


/// The only purpose of the `Open` trait is to provide a way
/// to access the `open` or `open_mut` trait in a generic way
/// This way the fact that the type is immutable or mutable
/// is not lost when it is used later.
pub trait Open {
    type Type;
    type Value;
    type Mask: BitSetLike;
    fn open(self) -> (Self::Mask, Self::Value);
    unsafe fn get(Self::Value, Index) -> Self::Type;
}


/// Join is an Iterator over a group of `Storages`
pub struct Join<O: Open> {
    keys: BitIter<O::Mask>,
    values: O::Value,
}

impl<O: Open> From<O> for Join<O> {
    fn from(o: O) -> Self {
        let (keys, values) = o.open();
        Join {
            keys: keys.iter(),
            values: values,
        }
    }
}

impl<O: Open> std::iter::Iterator for Join<O> {
    type Item = O::Type;
    fn next(&mut self) -> Option<O::Type> {
        self.keys.next().map(|idx| unsafe {
            // We only transmute and copy during iteration, which is safe and serves
            // as a poor man's replacement for the missing re-borrowing semantic.
            let values: O::Value = std::mem::transmute_copy(&self.values);
            O::get(values, idx)
        })
    }
}


macro_rules! define_open {
    // use variables to indicate the arity of the tuple
    ($($from:ident),*) => {
        impl<'a, $($from,)*> Open for ($($from),*,)
            where $($from: Open),*,
                  ($(<$from as Open>::Mask,)*): BitAnd,
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
            unsafe fn get(v: Self::Value, i: Index) -> Self::Type {
                let ($($from,)*) = v;
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
