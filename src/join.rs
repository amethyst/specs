use std;
use tuple_utils::Split;
use bitset;
use Index;
use {BitSet, BitSetAnd, BitSetLike, Storage, UnprotectedStorage};

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
    type Value;
    type Mask: BitSetLike;
    fn open(self) -> (Self::Mask, Self::Value);
}

impl<'a, T> Open for &'a T
    where T: Storage
{
    type Value = GetRef<'a, T::UnprotectedStorage>;
    type Mask = &'a BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        let (l, r) = self.open();
        (l, GetRef(r))
    }
}

impl<'a, T> Open for &'a mut T
    where T: Storage
{
    type Value = GetMut<'a, T::UnprotectedStorage>;
    type Mask = &'a BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        let (l, r) = self.open_mut();
        (l, GetMut(r))
    }
}

impl<'a, T> Open for &'a std::sync::RwLockReadGuard<'a, T>
    where T: Storage
{
    type Value = GetRef<'a, T::UnprotectedStorage>;
    type Mask = &'a BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        let (l, r) = (**self).open();
        (l, GetRef(r))
    }
}

impl<'b, 'a:'b, T> Open for &'b mut std::sync::RwLockWriteGuard<'a, T>
    where T: Storage
{
    type Value = GetMut<'b, T::UnprotectedStorage>;
    type Mask = &'b BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        let (l, r) = (**self).open_mut();
        (l, GetMut(r))
    }
}

/// Get like `Open` provides the same feature of providing
/// mutable vs immutable preserving around the UnprotectedStorage
/// trait
pub trait Get {
    type Value;
    unsafe fn get(&self, idx: Index) -> Self::Value;
}

pub struct GetRef<'a, T: 'a>(&'a T);
impl<'a, T> Get for GetRef<'a, T>
    where T: UnprotectedStorage
{
    type Value = &'a T::Component;
    unsafe fn get(&self, idx: Index) -> Self::Value {
        self.0.get(idx)
    }
}

pub struct GetMut<'a, T: 'a>(&'a mut T);
impl<'a, T> Get for GetMut<'a, T>
    where T: UnprotectedStorage
{
    type Value = &'a mut T::Component;
    #[allow(mutable_transmutes)]
    unsafe fn get(&self, idx: Index) -> Self::Value {
        // This is obviously unsafe and is one of the reasons this
        // trait is marked as unsafe to being with. It is safe
        // an an external api point of view because the bitmask
        // iterator never visits the same index twice, otherwise
        // this would provide multiple aliased mutable pointers which
        // is illegal in rust.
        let x: &mut Self = std::mem::transmute(self);
        x.0.get_mut(idx)
    }
}

/// Joined is an Iterator over a group of `Storages`
pub struct Joined<K, V> {
    keys: bitset::Iter<K>,
    values: V,
}

impl<K, V> Joined<K, V>
    where K: BitSetLike
{
    fn new(k: K, v: V) -> Joined<K, V> {
        Joined {
            keys: k.iter(),
            values: v,
        }
    }
}

impl<K, V> std::iter::Iterator for Joined<K, V>
    where K: BitSetLike,
          V: Get
{
    type Item = V::Value;
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(idx) = self.keys.next() {
            unsafe { Some(self.values.get(idx)) }
        } else {
            None
        }
    }
}

/// Join one or more `Storage` components together
/// this will return an iterator that will return
/// the components of an entity
pub trait Join {
    /// The Mask selects which elements are found
    /// in the  Values collection
    type Mask;
    /// The Values is a tuple of `UnprotectedStorage`
    type Values;
    /// Join the `Storages` togather
    fn join(self) -> Joined<Self::Mask, Self::Values>;
}

macro_rules! define_open {
    // use variables to indicate the arity of the tuple
    ($($from:ident),*) => {
        impl<'a, $($from,)*> Open for ($($from),*,)
            where $($from: Open),*,
                  ($(<$from as Open>::Mask,)*): BitAnd,
        {
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
        }

        impl<'a, $($from,)*> Get for ($($from),*,)
            where $($from: Get),*,
        {
            type Value = ($($from::Value),*,);
            #[allow(non_snake_case)]
            unsafe fn get(&self, idx: Index) -> Self::Value {
                let &($(ref $from,)*) = self;
                ($($from.get(idx)),*,)
            }
        }

        impl<'a, $($from,)*> Join for ($($from),*,)
            where $($from: Open),*,
                  ($(<$from as Open>::Mask),*,): BitAnd,
        {
            type Mask = <($($from::Mask),*,) as BitAnd>::Value;
            type Values = ($($from::Value),*,);

            fn join(self) -> Joined<Self::Mask, Self::Values> {
                let (mask, value) = self.open();
                Joined::new(mask, value)
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
