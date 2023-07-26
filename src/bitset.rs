//! Implementations and structures related to bitsets.
//!
//! Normally used for `Join`s and filtering entities.

// TODO: rustfmt bug (probably fixed in next rust release)
// #![cfg_attr(rustfmt, rustfmt::skip)]

use hibitset::{AtomicBitSet, BitSet, BitSetAnd, BitSetLike, BitSetNot, BitSetOr, BitSetXor};

#[nougat::gat(Type)]
use crate::join::LendJoin;
#[cfg(feature = "parallel")]
use crate::join::ParJoin;
use crate::join::{Join, RepeatableLendGet};
use crate::world::Index;

macro_rules! define_bit_join {
    ( impl < ( $( $lifetime:tt )* ) ( $( $arg:ident ),* ) > for $bitset:ty ) => {
        // SAFETY: `get` just returns the provided `id` (`Self::Value` is `()`
        // and corresponds with any mask instance).
        #[nougat::gat]
        unsafe impl<$( $lifetime, )* $( $arg ),*> LendJoin for $bitset
            where $( $arg: BitSetLike ),*
        {
            type Type<'next> = Index;
            type Value = ();
            type Mask = $bitset;

            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                (self, ())
            }

            unsafe fn get<'next>(_: &'next mut Self::Value, id: Index) -> Self::Type<'next>

            {
                id
            }
        }

        // SAFETY: <$biset as LendJoin>::get does not rely on only being called
        // once with a particular ID
        unsafe impl<$( $lifetime, )* $( $arg ),*> RepeatableLendGet for $bitset
            where $( $arg: BitSetLike ),* {}

        // SAFETY: `get` just returns the provided `id` (`Self::Value` is `()`
        // and corresponds with any mask instance).
        unsafe impl<$( $lifetime, )* $( $arg ),*> Join for $bitset
            where $( $arg: BitSetLike ),*
        {
            type Type = Index;
            type Value = ();
            type Mask = $bitset;

            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                (self, ())
            }

            unsafe fn get(_: &mut Self::Value, id: Index) -> Self::Type {
                id
            }
        }

        // SAFETY: `get` is safe to call concurrently and just returns the
        // provided `id` (`Self::Value` is `()` and corresponds with any mask
        // instance).
        #[cfg(feature = "parallel")]
        unsafe impl<$( $lifetime, )* $( $arg ),*> ParJoin for $bitset
            where $( $arg: BitSetLike ),*
        {
            type Type = Index;
            type Value = ();
            type Mask = $bitset;

            unsafe fn open(self) -> (Self::Mask, Self::Value) {
                (self, ())
            }

            unsafe fn get(_: &Self::Value, id: Index) -> Self::Type {
                id
            }
        }
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
