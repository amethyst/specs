use std::iter::repeat;

use Index;

pub const BITS: usize = 5;
pub const LAYERS: usize = 4;
pub const MAX: usize = BITS * LAYERS;
pub const MAX_EID: usize = 2 << MAX - 1;

pub const SHIFT0: usize = 0;
pub const SHIFT1: usize = SHIFT0 + BITS;
pub const SHIFT2: usize = SHIFT1 + BITS;
pub const SHIFT3: usize = SHIFT2 + BITS;

/// A `BitSet` is a simple set designed to track entity indices for which
/// a certain component exists. It does not track the `Generation` of the
/// entities that it contains.
///
/// Note, a `BitSet` is limited by design to only 1,048,576 indices.
/// Adding beyond this limit will cause the `BitSet` to panic.
#[derive(Clone)]
pub struct BitSet {
    layer3: u32,
    layer2: Vec<u32>,
    layer1: Vec<u32>,
    layer0: Vec<u32>,
}

#[inline]
fn offsets(bit: Index) -> (usize, usize, usize) {
    (bit.offset(SHIFT1), bit.offset(SHIFT2), bit.offset(SHIFT3))
}

impl BitSet {
    /// Creates an empty `BitSet`.
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

    /// Creates an empty `BitSet`, preallocated for up to `max` indices.
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
        self.layer1[p1] |= id.mask(SHIFT1);
        self.layer2[p2] |= id.mask(SHIFT2);
        self.layer3 |= id.mask(SHIFT3);
    }

    /// Adds `id` to the `BitSet`. Returns `true` if the value was
    /// already in the set.
    #[inline]
    pub fn add(&mut self, id: Index) -> bool {
        let (p0, mask) = (id.offset(SHIFT1), id.mask(SHIFT0));

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

    /// Removes `id` from the set, returns `true` if the value
    /// was removed, and `false` if the value was not set
    /// to begin with.
    #[inline]
    pub fn remove(&mut self, id: Index) -> bool {
        let (p0, p1, p2) = offsets(id);

        if p0 >= self.layer0.len() {
            return false;
        }

        if self.layer0[p0] & id.mask(SHIFT0) == 0 {
            return false;
        }

        // if the bitmask was set we need to clear
        // its bit from layer0 to 3. the layers abover only
        // should be cleared if the bit cleared was the last bit
        // in its set
        self.layer0[p0] &= !id.mask(SHIFT0);
        if self.layer0[p0] != 0 {
            return true;
        }

        self.layer1[p1] &= !id.mask(SHIFT1);
        if self.layer1[p1] != 0 {
            return true;
        }

        self.layer2[p2] &= !id.mask(SHIFT2);
        if self.layer2[p2] != 0 {
            return true;
        }

        self.layer3 &= !id.mask(SHIFT3);
        return true;
    }

    /// Returns `true` if `id` is in the set.
    #[inline]
    pub fn contains(&self, id: u32) -> bool {
        let p0 = id.offset(SHIFT1);
        if p0 >= self.layer0.len() {
            return false;
        }
        (self.layer0[p0] & id.mask(SHIFT0)) != 0
    }
}

pub trait Row: Sized + Copy {
    fn row(self, shift: usize) -> usize;
    fn offset(self, shift: usize) -> usize;

    #[inline(always)]
    fn mask(self, shift: usize) -> u32 {
        1u32 << self.row(shift)
    }
}

impl Row for Index {
    #[inline(always)]
    fn row(self, shift: usize) -> usize {
        ((self >> shift) as usize) & ((1 << BITS) - 1)
    }

    #[inline(always)]
    fn offset(self, shift: usize) -> usize {
        self as usize / (1 << shift)
    }
}

/// A generic interface for `BitSet`-like types.
///
/// Every `BitSetLike` in `specs` is hierarchical, meaning that there
/// are multiple levels that branch out in a tree like structure.
///
/// Layer0 each bit represents one Index of the set
/// Layer1 each bit represents one `u32` of Layer0, and will be
/// set only if the word below it is not zero.
/// Layer2 has the same arrangement but with Layer1, and Layer3 with Layer2.
///
/// This arrangement allows for rapid jumps across the key-space.
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
    fn iter(self) -> Iter<Self>
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

/// `BitSetAnd` takes two `BitSetLike` items, and merges the masks
/// returning a new virtual set, which represents an intersection of the
/// two original sets.
pub struct BitSetAnd<A: BitSetLike, B: BitSetLike>(pub A, pub B);

impl<A: BitSetLike, B: BitSetLike> BitSetLike for BitSetAnd<A, B> {
    #[inline] fn layer3(&self) -> u32 { self.0.layer3() & self.1.layer3() }
    #[inline] fn layer2(&self, i: usize) -> u32 { self.0.layer2(i) & self.1.layer2(i) }
    #[inline] fn layer1(&self, i: usize) -> u32 { self.0.layer1(i) & self.1.layer1(i) }
    #[inline] fn layer0(&self, i: usize) -> u32 { self.0.layer0(i) & self.1.layer0(i) }
}

pub struct Iter<T> {
    set: T,
    masks: [u32; 4],
    prefix: [u32; 3]
}

impl<T> Iterator for Iter<T>
    where T: BitSetLike
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

        assert_eq!((&odd).iter().count(), 50_000);
        assert_eq!((&even).iter().count(), 50_000);
        assert_eq!(BitSetAnd(&odd, &even).iter().count(), 0);
    }
}

