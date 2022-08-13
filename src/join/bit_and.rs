use hibitset::{BitSetAnd, BitSetLike};
use tuple_utils::Split;

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
