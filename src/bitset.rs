//! Implementations and structures related to bitsets.
//!
//! Normally used for `Join`s and filtering entities.

#![cfg_attr(rustfmt, rustfmt_skip)]

use hibitset::{AtomicBitSet, BitSet, BitSetAnd, BitSetLike, BitSetNot, BitSetOr, BitSetXor};

use crate::join::Join;
#[cfg(feature = "parallel")]
use crate::join::ParJoin;
use crate::world::Index;

macro_rules! define_bit_join {
    ( impl < ( $( $lifetime:tt )* ) ( $( $arg:ident ),* ) > for $bitset:ty ) => {
        impl<$( $lifetime, )* $( $arg ),*> Join for $bitset
            where $( $arg: BitSetLike ),*
        {
            type Type = Index;
            type Value = ();
            type Mask = $bitset;

            // SAFETY: This just moves a `BitSet`; invariants of `Join` are fulfilled, since `Self::Value` cannot be mutated.
            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                (self, ())
            }

            // SAFETY: No unsafe code and no invariants to meet.
            unsafe fn get(_: &mut Self::Value, id: Index) -> Self::Type {
                id
            }
        }

        #[cfg(feature = "parallel")]
        unsafe impl<$( $lifetime, )* $( $arg ),*> ParJoin for $bitset
            where $( $arg: BitSetLike ),*
        { }
    }
}

define_bit_join!(impl<()()> for BitSet);
define_bit_join!(impl<('a)()> for &'a BitSet);
define_bit_join!(impl<()()> for AtomicBitSet);
define_bit_join!(impl<('a)()> for &'a AtomicBitSet);
define_bit_join!(impl<()(A)> for BitSetNot<A>);
define_bit_join!(impl<('a)(A)> for &'a BitSetNot<A>);
define_bit_join!(impl<()(A, B)> for BitSetAnd<A, B>);
define_bit_join!(impl<('a)(A, B)> for &'a BitSetAnd<A, B>);
define_bit_join!(impl<()(A, B)> for BitSetOr<A, B>);
define_bit_join!(impl<('a)(A, B)> for &'a BitSetOr<A, B>);
define_bit_join!(impl<()(A, B)> for BitSetXor<A, B>);
define_bit_join!(impl<('a)()> for &'a dyn BitSetLike);
