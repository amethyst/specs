use std::iter::repeat;

use Index;

pub const BITS: usize = 5;
pub const LAYERS: usize = 4;
pub const MAX: usize = BITS * LAYERS;
pub const MAX_EID: usize = 2 << MAX - 1;

/// A BitSet is a simple set designed for tracking entity indexes
/// are present or not. It does not track the `Generation` of the
/// entities that it contains.
///
/// Note, the BitSet is limited by design to only 1,048,576 indexs
/// adding beyond this will cause the BitSet to panic.
#[derive(Clone)]
pub struct BitSet {
    layer3: u32,
    layer2: Vec<u32>,
    layer1: Vec<u32>,
    layer0: Vec<u32>,
}

#[inline]
fn offsets(bit: Index) -> (usize, usize, usize) {
    (bit.offset::<Shift1>(), bit.offset::<Shift2>(), bit.offset::<Shift3>())
}

impl BitSet {
    /// Create an empty BitSet
    pub fn new() -> BitSet {
        BitSet {
            layer3: 0,
            layer2: Vec::new(),
            layer1: Vec::new(),
            layer0: Vec::new(),
        }
    }

    #[inline]
    fn valid_range(max: Index) {
        if (MAX_EID as u32) < max {
            panic!("Expected index to be less then {}, found {}", MAX_EID, max);
        }
    }

    /// Create an empty BitSet with up to max Index
    pub fn with_capacity(max: Index) -> BitSet {
        Self::valid_range(max);
        let mut value = BitSet::new();
        value.extend(max);
        value
    }

    #[inline(never)]
    fn extend(&mut self, id: Index) {
        Self::valid_range(id);
        let (p0, p1, p2) = offsets(id);

        if self.layer2.len() <= p2 {
            let count = p2 - self.layer2.len() + 1;
            self.layer2.extend(repeat(0).take(count));
        }
        if self.layer1.len() <= p1 {
            let count = p1 - self.layer1.len() + 1;
            self.layer1.extend(repeat(0).take(count));
        }
        if self.layer0.len() <= p0 {
            let count = p0 - self.layer0.len() + 1;
            self.layer0.extend(repeat(0).take(count));
        }
    }

    /// this is used to set the levels in the hierarchy
    /// when the lowest layer was set from 0
    fn add_slow(&mut self, id: Index) {
        let (_, p1, p2) = offsets(id);
        self.layer1[p1] |= id.mask::<Shift1>();
        self.layer2[p2] |= id.mask::<Shift2>();
        self.layer3 |= id.mask::<Shift3>();
    }

    /// Add `id` to the bitset. Returning if the value was
    /// already in the set before it was added
    #[inline]
    pub fn add(&mut self, id: Index) -> bool {
        let (p0, mask) = (id.offset::<Shift1>(), id.mask::<Shift0>());

        if p0 >= self.layer0.len() {
            self.extend(id);
        }

        if self.layer0[p0] & mask != 0 {
            return true;
        }

        // we need to set the bit on every layer to indicate
        // that the value can be found here.
        let old = self.layer0[p0];
        self.layer0[p0] |= mask;
        if old == 0 {
            self.add_slow(id);
        } else {
            self.layer0[p0] |= mask;
        }
        false
    }

    /// Remove `id` from the set, returns true if the value
    /// was removed, returns false if the value was not set
    /// to begin with
    #[inline]
    pub fn remove(&mut self, id: Index) -> bool {
        let (p0, p1, p2) = offsets(id);

        if p0 >= self.layer0.len() {
            return false;
        }

        if self.layer0[p0] & id.mask::<Shift0>() == 0 {
            return false;
        }

        // if the bitmask was set we need to clear
        // its bit from layer0 to 3. the layers abover only
        // should be cleared if the bit cleared was the last bit
        // in its set
        self.layer0[p0] &= !id.mask::<Shift0>();
        if self.layer0[p0] != 0 {
            return true;
        }

        self.layer1[p1] &= !id.mask::<Shift1>();
        if self.layer1[p1] != 0 {
            return true;
        }

        self.layer2[p2] &= !id.mask::<Shift2>();
        if self.layer2[p2] != 0 {
            return true;
        }

        self.layer3 &= !id.mask::<Shift3>();
        return true;
    }

    /// Check to see if `id` was included in the set
    /// return true if it was, false otherwise
    #[inline]
    pub fn contains(&self, bit: u32) -> bool {
        let p0 = bit.offset::<Shift1>();
        if p0 >= self.layer0.len() {
            return false;
        }
        (self.layer0[p0] & bit.mask::<Shift0>()) != 0
    }

    /// Create an iterator over the the keyspace
    pub fn iter<'a>(&'a self) -> Iter<'a, Self> {
        Iter{
            prefix: [0; 3],
            masks: [0, 0, 0, self.layer3],
            set: self
        }
    }
}

pub trait Shift {
    fn shift() -> usize;
    fn bits() -> usize;

    #[inline]
    fn mask() -> usize {
        ((1 << Self::bits()) - 1) << Self::shift()
    }
}

pub trait Row: Sized + Copy {
    fn row<S>(self) -> usize where S: Shift;
    fn offset<S>(self) -> usize where S: Shift;

    #[inline(always)]
    fn mask<S: Shift>(self) -> u32 {
        1u32 << self.row::<S>()
    }
}

impl Row for Index {
    #[inline(always)]
    fn row<S>(self) -> usize
        where S: Shift
    {
        let size = S::bits();
        let shift = S::shift();

        ((self >> shift) as usize) & ((1 << size) - 1)
    }

    #[inline(always)]
    fn offset<S>(self) -> usize
        where S: Shift
    {
        self as usize / (1 << S::shift())
    }
}

#[derive(Copy, Clone)]
pub struct Shift0;
impl Shift for Shift0 {
    #[inline(always)]
    fn shift() -> usize {
        0
    }
    #[inline(always)]
    fn bits() -> usize {
        BITS
    }
}

#[derive(Copy, Clone)]
pub struct Shift1;
impl Shift for Shift1 {
    #[inline(always)]
    fn shift() -> usize {
        Shift0::bits()
    }
    #[inline(always)]
    fn bits() -> usize {
        BITS
    }
}

#[derive(Copy, Clone)]
pub struct Shift2;
impl Shift for Shift2 {
    #[inline(always)]
    fn shift() -> usize {
        Shift1::bits() + Shift1::shift()
    }
    #[inline(always)]
    fn bits() -> usize {
        BITS
    }
}

#[derive(Copy, Clone)]
pub struct Shift3;
impl Shift for Shift3 {
    #[inline(always)]
    fn shift() -> usize {
        Shift2::bits() + Shift2::shift()
    }
    #[inline(always)]
    fn bits() -> usize {
        BITS
    }
}

/// A generic interface for BitSet like type
/// A bitset in `specs` is hierarchal meaning that there
/// are multiple levels that branch out in a tree like structure
///
/// Layer0 has each bit representing one Index of the set
/// Layer1 each bit represents one u32 of Layer0, and will be
/// set only if the word below it is none zero.
/// Layer2 has the same arrangement but with Layer1, and Layer4 with Layer4
///
/// This arrangement allows for rapid jumps across the key-space
pub trait BitSetLike {
    /// Return a u32 where each bit represents if any word in layer2
    /// has been set. 
    fn layer3(&self) -> u32;
    /// Return the u32 from the array of u32s that indicates if any
    /// bit has been set in layer1
    fn layer2(&self, i: usize) -> u32;
    /// Return the u32 from the array of u32s that indicates if any
    /// bit has been set in layer0
    fn layer1(&self, i: usize) -> u32;
    /// Return a u32 that maps to the direct 1:1 association with
    /// each index of the set
    fn layer0(&self, i: usize) -> u32;

    /// Create an iterator that will scan over the keyspace
    fn iter<'a>(&'a self) -> Iter<'a, Self>
        where Self: Sized
    {
        Iter{
            prefix: [0; 3],
            masks: [0, 0, 0, self.layer3()],
            set: self
        }
    }
}

impl<'a, T> BitSetLike for &'a T where T: BitSetLike
{
    #[inline] fn layer3(&self) -> u32 { (*self).layer3() }
    #[inline] fn layer2(&self, i: usize) -> u32 { (*self).layer2(i) }
    #[inline] fn layer1(&self, i: usize) -> u32 { (*self).layer1(i) }
    #[inline] fn layer0(&self, i: usize) -> u32 { (*self).layer0(i) }
}

impl BitSetLike for BitSet {
    #[inline] fn layer3(&self) -> u32 { self.layer3 }
    #[inline] fn layer2(&self, i: usize) -> u32 { self.layer2[i] }
    #[inline] fn layer1(&self, i: usize) -> u32 { self.layer1[i] }
    #[inline] fn layer0(&self, i: usize) -> u32 { self.layer0[i] }
}

/// BitSet and takes two BitSetLike items and merges the masks
/// returning a new virtual set.
pub struct BitSetAnd<A: BitSetLike, B: BitSetLike>(pub A, pub B);

impl<A: BitSetLike, B: BitSetLike> BitSetLike for BitSetAnd<A, B> {
    #[inline] fn layer3(&self) -> u32 { self.0.layer3() & self.1.layer3() }
    #[inline] fn layer2(&self, i: usize) -> u32 { self.0.layer2(i) & self.1.layer2(i) }
    #[inline] fn layer1(&self, i: usize) -> u32 { self.0.layer1(i) & self.1.layer1(i) }
    #[inline] fn layer0(&self, i: usize) -> u32 { self.0.layer0(i) & self.1.layer0(i) }
}

pub struct Iter<'a, T:'a> {
    set: &'a T,
    masks: [u32; 4],
    prefix: [u32; 3]
}

impl<'a, T> Iterator for Iter<'a, T>
    where T: BitSetLike+'a
{
    type Item = Index;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.masks[0] != 0 {
                let bit = self.masks[0].trailing_zeros();
                self.masks[0] &= !(1 << bit);
                return Some(self.prefix[0] | bit);
            }

            if self.masks[1] != 0 {
                let bit = self.masks[1].trailing_zeros();
                self.masks[1] &= !(1 << bit);
                let idx = self.prefix[1] | bit;
                self.masks[0] = self.set.layer0(idx as usize);
                self.prefix[0] = idx << BITS;
                continue;
            }

            if self.masks[2] != 0 {
                let bit = self.masks[2].trailing_zeros();
                self.masks[2] &= !(1 << bit);
                let idx = self.prefix[2] | bit;
                self.masks[1] = self.set.layer1(idx as usize);
                self.prefix[1] = idx << BITS;
                continue;
            }

            if self.masks[3] != 0 {
                let bit = self.masks[3].trailing_zeros();
                self.masks[3] &= !(1 << bit);
                self.masks[2] = self.set.layer2(bit as usize);
                self.prefix[2] = bit << BITS;
                continue;
            }
            return None;
        }
    }
}

#[cfg(test)]
mod set_test {
    use super::{BitSet, BitSetAnd, BitSetLike};

    #[test]
    fn insert() {
        let mut c = BitSet::new();
        for i in 0..1_000 {
            assert!(!c.add(i));
            assert!(c.add(i));
        }

        for i in 0..1_000 {
            assert!(c.contains(i));
        }
    }

    #[test]
    fn insert_100k() {
        let mut c = BitSet::new();
        for i in 0..100_000 {
            assert!(!c.add(i));
            assert!(c.add(i));
        }

        for i in 0..100_000 {
            assert!(c.contains(i));
        }
    }

    #[test]
    fn remove() {
        let mut c = BitSet::new();
        for i in 0..1_000 {
            assert!(!c.add(i));
        }

        for i in 0..1_000 {
            assert!(c.contains(i));
            assert!(c.remove(i));
            assert!(!c.contains(i));
            assert!(!c.remove(i));
        }
    }

    #[test]
    fn iter() {
        let mut c = BitSet::new();
        for i in 0..100_000 {
            c.add(i);
        }

        let mut count = 0;
        for (idx, i) in c.iter().enumerate() {
            count += 1;
            assert_eq!(idx, i as usize);
        }
        assert_eq!(count, 100_000);
    }

    #[test]
    fn iter_odd_even() {
        let mut odd = BitSet::new();
        let mut even = BitSet::new();
        for i in 0..100_000 {
            if i % 2 == 1 {
                odd.add(i);
            } else {
                even.add(i);
            }
        }

        assert_eq!(odd.iter().count(), 50_000);
        assert_eq!(even.iter().count(), 50_000);
        assert_eq!(BitSetAnd(&odd, &even).iter().count(), 0);
    }
}

