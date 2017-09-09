#![cfg_attr(rustfmt, rustfmt_skip)]

use hibitset::{BitSet, BitSetAnd, BitSetLike, BitSetNot, BitSetOr};

use {Index, Join, ParJoin};

macro_rules! define_bit_join {
    ( $bitset:ident [ $( $arg:ident ),* ] ) => {
        impl<'a, $( $arg ),*> Join for &'a $bitset<$( $arg ),*>
            where $( $arg: BitSetLike ),*
        {
            type Type = Index;
            type Value = ();
            type Mask = &'a $bitset<$( $arg ),*>;
            fn open(self) -> (Self::Mask, Self::Value) {
                (self, ())
            }
            unsafe fn get(_: &mut Self::Value, id: Index) -> Self::Type {
                id
            }
        }

        unsafe impl<'a, $( $arg ),*> ParJoin for &'a $bitset<$( $arg ),*>
            where $( $arg: BitSetLike ),*
        { }
    }
}

define_bit_join!(BitSet [ ]);
define_bit_join!(BitSetAnd [ A, B ]);
define_bit_join!(BitSetNot [ A ]);
define_bit_join!(BitSetOr [ A, B ]);
