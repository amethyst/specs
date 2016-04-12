use tuple_utils::Split;
use {BitSetAnd, BitSetLike};

/// Join is a helper method to & bitsets togather resulting in a tree
pub trait Join {
    type Value;
    fn join(self) -> Self::Value;
}

impl<A> Join for (A,)
    where A: BitSetLike
{
    type Value = A;
    fn join(self) -> Self::Value {
        self.0
    }
}


impl<A, B> Join for (A, B)
    where A: BitSetLike,
          B: BitSetLike
{
    type Value = BitSetAnd<A,B>;
    fn join(self) -> Self::Value {
        let (l, r) = self.split();
        BitSetAnd(l.join(), r.join())
    }
}

impl<A, B, C> Join for (A, B, C)
    where A: BitSetLike,
          B: BitSetLike,
          C: BitSetLike,
{
    type Value = BitSetAnd<A,BitSetAnd<B,C>>;
    fn join(self) -> Self::Value {
        let (l, r) = self.split();
        BitSetAnd(l.join(), r.join())
    }
}

impl<A, B, C, D> Join for (A, B, C, D)
    where A: BitSetLike,
          B: BitSetLike,
          C: BitSetLike,
          D: BitSetLike,
{
    type Value = BitSetAnd<BitSetAnd<A,B>,BitSetAnd<C,D>>;
    fn join(self) -> Self::Value {
        let (l, r) = self.split();
        BitSetAnd(l.join(), r.join())
    }
}

impl<A, B, C, D, E> Join for (A, B, C, D, E)
    where A: BitSetLike,
          B: BitSetLike,
          C: BitSetLike,
          D: BitSetLike,
          E: BitSetLike,
{
    type Value = BitSetAnd<BitSetAnd<A,B>,BitSetAnd<C,BitSetAnd<D,E>>>;
    fn join(self) -> Self::Value {
        let (l, r) = self.split();
        BitSetAnd(l.join(), r.join())
    }
}

impl<A, B, C, D, E, F> Join for (A, B, C, D, E, F)
    where A: BitSetLike,
          B: BitSetLike,
          C: BitSetLike,
          D: BitSetLike,
          E: BitSetLike,
          F: BitSetLike,
{
    type Value = BitSetAnd<BitSetAnd<A,BitSetAnd<B,C>>,BitSetAnd<D,BitSetAnd<E,F>>>;
    fn join(self) -> Self::Value {
        let (l, r) = self.split();
        BitSetAnd(l.join(), r.join())
    }
}

impl<A, B, C, D, E, F, G> Join for (A, B, C, D, E, F, G)
    where A: BitSetLike,
          B: BitSetLike,
          C: BitSetLike,
          D: BitSetLike,
          E: BitSetLike,
          F: BitSetLike,
          G: BitSetLike,
{
    type Value = BitSetAnd<BitSetAnd<A,BitSetAnd<B,C>>,BitSetAnd<BitSetAnd<D,E>,BitSetAnd<F,G>>>;
    fn join(self) -> Self::Value {
        let (l, r) = self.split();
        BitSetAnd(l.join(), r.join())
    }
}

impl<A, B, C, D, E, F, G, H> Join for (A, B, C, D, E, F, G, H)
    where A: BitSetLike,
          B: BitSetLike,
          C: BitSetLike,
          D: BitSetLike,
          E: BitSetLike,
          F: BitSetLike,
          G: BitSetLike,
          H: BitSetLike,
{
    type Value = BitSetAnd<BitSetAnd<BitSetAnd<A,B>,BitSetAnd<C,D>>,BitSetAnd<BitSetAnd<E,F>,BitSetAnd<G,H>>>;
    fn join(self) -> Self::Value {
        let (l, r) = self.split();
        BitSetAnd(l.join(), r.join())
    }
}

impl<A, B, C, D, E, F, G, H, I> Join for (A, B, C, D, E, F, G, H, I)
    where A: BitSetLike,
          B: BitSetLike,
          C: BitSetLike,
          D: BitSetLike,
          E: BitSetLike,
          F: BitSetLike,
          G: BitSetLike,
          H: BitSetLike,
          I: BitSetLike,
{
    type Value = BitSetAnd<BitSetAnd<BitSetAnd<A,B>,BitSetAnd<C,D>>,BitSetAnd<BitSetAnd<E,F>,BitSetAnd<G,BitSetAnd<H,I>>>>;
    fn join(self) -> Self::Value {
        let (l, r) = self.split();
        BitSetAnd(l.join(), r.join())
    }
}

impl<A, B, C, D, E, F, G, H, I, J> Join for (A, B, C, D, E, F, G, H, I, J)
    where A: BitSetLike,
          B: BitSetLike,
          C: BitSetLike,
          D: BitSetLike,
          E: BitSetLike,
          F: BitSetLike,
          G: BitSetLike,
          H: BitSetLike,
          I: BitSetLike,
          J: BitSetLike,
{
    type Value = BitSetAnd<BitSetAnd<BitSetAnd<A,B>,BitSetAnd<C,BitSetAnd<D,E>>>,BitSetAnd<BitSetAnd<F,G>,BitSetAnd<H,BitSetAnd<I,J>>>>;
    fn join(self) -> Self::Value {
        let (l, r) = self.split();
        BitSetAnd(l.join(), r.join())
    }
}

impl<A, B, C, D, E, F, G, H, I, J, K> Join for (A, B, C, D, E, F, G, H, I, J, K)
    where A: BitSetLike,
          B: BitSetLike,
          C: BitSetLike,
          D: BitSetLike,
          E: BitSetLike,
          F: BitSetLike,
          G: BitSetLike,
          H: BitSetLike,
          I: BitSetLike,
          J: BitSetLike,
          K: BitSetLike,
{
    type Value = BitSetAnd<BitSetAnd<BitSetAnd<A,B>,BitSetAnd<C,BitSetAnd<D,E>>>,BitSetAnd<BitSetAnd<F,BitSetAnd<G,H>>,BitSetAnd<I,BitSetAnd<J,K>>>>;
    fn join(self) -> Self::Value {
        let (l, r) = self.split();
        BitSetAnd(l.join(), r.join())
    }
}

impl<A, B, C, D, E, F, G, H, I, J, K, L> Join for (A, B, C, D, E, F, G, H, I, J, K, L)
    where A: BitSetLike,
          B: BitSetLike,
          C: BitSetLike,
          D: BitSetLike,
          E: BitSetLike,
          F: BitSetLike,
          G: BitSetLike,
          H: BitSetLike,
          I: BitSetLike,
          J: BitSetLike,
          K: BitSetLike,
          L: BitSetLike,
{
    type Value = BitSetAnd<BitSetAnd<BitSetAnd<A,BitSetAnd<B,C>>,BitSetAnd<D,BitSetAnd<E,F>>>,BitSetAnd<BitSetAnd<G,BitSetAnd<H,I>>,BitSetAnd<J,BitSetAnd<K,L>>>>;
    fn join(self) -> Self::Value {
        let (l, r) = self.split();
        BitSetAnd(l.join(), r.join())
    }
}

impl<A, B, C, D, E, F, G, H, I, J, K, L, M> Join for (A, B, C, D, E, F, G, H, I, J, K, L, M)
    where A: BitSetLike,
          B: BitSetLike,
          C: BitSetLike,
          D: BitSetLike,
          E: BitSetLike,
          F: BitSetLike,
          G: BitSetLike,
          H: BitSetLike,
          I: BitSetLike,
          J: BitSetLike,
          K: BitSetLike,
          L: BitSetLike,
          M: BitSetLike,
{
    type Value = BitSetAnd<BitSetAnd<BitSetAnd<A,BitSetAnd<B,C>>,BitSetAnd<D,BitSetAnd<E,F>>>,BitSetAnd<BitSetAnd<G,BitSetAnd<H,I>>,BitSetAnd<BitSetAnd<J,K>,BitSetAnd<L,M>>>>;
    fn join(self) -> Self::Value {
        let (l, r) = self.split();
        BitSetAnd(l.join(), r.join())
    }
}

impl<A, B, C, D, E, F, G, H, I, J, K, L, M, N> Join for (A, B, C, D, E, F, G, H, I, J, K, L, M, N)
    where A: BitSetLike,
          B: BitSetLike,
          C: BitSetLike,
          D: BitSetLike,
          E: BitSetLike,
          F: BitSetLike,
          G: BitSetLike,
          H: BitSetLike,
          I: BitSetLike,
          J: BitSetLike,
          K: BitSetLike,
          L: BitSetLike,
          M: BitSetLike,
          N: BitSetLike,
{
    type Value = BitSetAnd<BitSetAnd<BitSetAnd<A,BitSetAnd<B,C>>,BitSetAnd<BitSetAnd<D,E>,BitSetAnd<F,G>>>,BitSetAnd<BitSetAnd<H,BitSetAnd<I,J>>,BitSetAnd<BitSetAnd<K,L>,BitSetAnd<M,N>>>>;
    fn join(self) -> Self::Value {
        let (l, r) = self.split();
        BitSetAnd(l.join(), r.join())
    }
}

impl<A, B, C, D, E, F, G, H, I, J, K, L, M, N, O> Join for (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O)
    where A: BitSetLike,
          B: BitSetLike,
          C: BitSetLike,
          D: BitSetLike,
          E: BitSetLike,
          F: BitSetLike,
          G: BitSetLike,
          H: BitSetLike,
          I: BitSetLike,
          J: BitSetLike,
          K: BitSetLike,
          L: BitSetLike,
          M: BitSetLike,
          N: BitSetLike,
          O: BitSetLike,
{
    type Value = BitSetAnd<BitSetAnd<BitSetAnd<A,BitSetAnd<B,C>>,BitSetAnd<BitSetAnd<D,E>,BitSetAnd<F,G>>>,BitSetAnd<BitSetAnd<BitSetAnd<H,I>,BitSetAnd<J,K>>,BitSetAnd<BitSetAnd<L,M>,BitSetAnd<N,O>>>>;
    fn join(self) -> Self::Value {
        let (l, r) = self.split();
        BitSetAnd(l.join(), r.join())
    }
}

impl<A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P> Join for (A, B, C, D, E, F, G, H, I, J, K, L, M, N, O, P)
    where A: BitSetLike,
          B: BitSetLike,
          C: BitSetLike,
          D: BitSetLike,
          E: BitSetLike,
          F: BitSetLike,
          G: BitSetLike,
          H: BitSetLike,
          I: BitSetLike,
          J: BitSetLike,
          K: BitSetLike,
          L: BitSetLike,
          M: BitSetLike,
          N: BitSetLike,
          O: BitSetLike,
          P: BitSetLike,
{
    type Value = BitSetAnd<BitSetAnd<BitSetAnd<BitSetAnd<A,B>,BitSetAnd<C,D>>,BitSetAnd<BitSetAnd<E,F>,BitSetAnd<G,H>>>,BitSetAnd<BitSetAnd<BitSetAnd<I,J>,BitSetAnd<K,L>>,BitSetAnd<BitSetAnd<M,N>,BitSetAnd<O,P>>>>;
    fn join(self) -> Self::Value {
        let (l, r) = self.split();
        BitSetAnd(l.join(), r.join())
    }
}
