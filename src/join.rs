use tuple_utils::Split;
use {BitSetAnd, BitSetLike};

/// Join is a helper method to & bitsets togather resulting in a tree
pub trait Join {
    type Value;
    fn join(self) -> Self::Value;
}

/// This needs to be special cased
impl<A> Join for (A,)
    where A: BitSetLike
{
    type Value = A;
    fn join(self) -> Self::Value {
        self.0
    }
}

macro_rules! merge_impl {
    // use variables to indicate the arity of the tuple
    ($($from:ident),*) => {
      impl<$($from),*> Join for ($($from),*)
          where $($from: BitSetLike),*
      {
          type Value = BitSetAnd<
            <<Self as Split>::Left as Join>::Value,
            <<Self as Split>::Right as Join>::Value
          >;
          fn join(self) -> Self::Value {
              let (l, r) = self.split();
              BitSetAnd(l.join(), r.join())
          }
      }
    }
}

merge_impl!{A, B}
merge_impl!{A, B, C}
merge_impl!{A, B, C, D}
merge_impl!{A, B, C, D, E}
merge_impl!{A, B, C, D, E, F}
merge_impl!{A, B, C, D, E, F, G}
merge_impl!{A, B, C, D, E, F, G, H}
merge_impl!{A, B, C, D, E, F, G, H, I}
merge_impl!{A, B, C, D, E, F, G, H, I, J}
merge_impl!{A, B, C, D, E, F, G, H, I, J, K}
merge_impl!{A, B, C, D, E, F, G, H, I, J, K, L}
merge_impl!{A, B, C, D, E, F, G, H, I, J, K, L, M}
merge_impl!{A, B, C, D, E, F, G, H, I, J, K, L, M, N}
merge_impl!{A, B, C, D, E, F, G, H, I, J, K, L, M, N, O}
merge_impl!{A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P}
