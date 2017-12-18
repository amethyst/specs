#![cfg_attr(rustfmt, rustfmt_skip)]

use hibitset::{AtomicBitSet, BitSet, BitSetAnd, BitSetLike, BitSetNot, BitSetOr, BitSetXor};

use storage::Flag;
use join::{Join, ParJoin};
use world::Index;

macro_rules! define_bit_join {
    ( impl < ( $( $lifetime:tt )* ) ( $( $arg:ident ),* ) > for $bitset:ty ) => {
        impl<$( $lifetime, )* $( $arg ),*> Join for $bitset
            where $( $arg: BitSetLike ),* 
        {
            type Type = Index;
            type Value = ();
            type Mask = $bitset;
            fn open(self) -> (Self::Mask, Self::Value) {
                (self, ())
            }
            unsafe fn get(_: &mut Self::Value, id: Index) -> Self::Type {
                id
            }
        }

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

macro_rules! extend {
    ( $bitset:ident ) => {
        impl Extend<Flag> for $bitset {
            fn extend<T>(&mut self, iter: T)
            where
                T: IntoIterator<Item = Flag>,
            {
                for item in iter.into_iter() {
                    match item {
                        Flag::Flag(index) => self.add(index),
                        Flag::Unflag(index) => self.remove(index),
                    };
                }
            }
        }

        impl<'a> Extend<&'a Flag> for $bitset {
            fn extend<T>(&mut self, iter: T)
            where
                T: IntoIterator<Item = &'a Flag>,
            {
                for item in iter.into_iter() {
                    match *item {
                        Flag::Flag(index) => self.add(index),
                        Flag::Unflag(index) => self.remove(index),
                    };
                }
            }
        }
    };
}

extend!(BitSet);
extend!(AtomicBitSet);
