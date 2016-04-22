use std::iter::repeat;
use std::sync::atomic::{AtomicUsize, Ordering};

use Index;

#[cfg(target_pointer_width= "64")]
pub const BITS: usize = 6;
#[cfg(target_pointer_width= "32")]
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
    layer3: usize,
    layer2: Vec<usize>,
    layer1: Vec<usize>,
    layer0: Vec<usize>,
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
        p0 < self.layer0.len() && (self.layer0[p0] & id.mask(SHIFT0)) != 0
    }
}

pub trait Row: Sized + Copy {
    fn row(self, shift: usize) -> usize;
    fn offset(self, shift: usize) -> usize;

    #[inline(always)]
    fn mask(self, shift: usize) -> usize {
        1usize << self.row(shift)
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
/// Layer1 each bit represents one `usize` of Layer0, and will be
/// set only if the word below it is not zero.
/// Layer2 has the same arrangement but with Layer1, and Layer3 with Layer2.
///
/// This arrangement allows for rapid jumps across the key-space.
pub trait BitSetLike {
    /// Return a usize where each bit represents if any word in layer2
    /// has been set.
    fn layer3(&self) -> usize;
    /// Return the usize from the array of usizes that indicates if any
    /// bit has been set in layer1
    fn layer2(&self, i: usize) -> usize;
    /// Return the usize from the array of usizes that indicates if any
    /// bit has been set in layer0
    fn layer1(&self, i: usize) -> usize;
    /// Return a usize that maps to the direct 1:1 association with
    /// each index of the set
    fn layer0(&self, i: usize) -> usize;

    /// Create an iterator that will scan over the keyspace
    fn iter(self) -> BitIter<Self>
        where Self: Sized
    {
        BitIter {
            prefix: [0; 3],
            masks: [0, 0, 0, self.layer3()],
            set: self
        }
    }
}

impl<'a, T> BitSetLike for &'a T where T: BitSetLike
{
    #[inline] fn layer3(&self) -> usize { (*self).layer3() }
    #[inline] fn layer2(&self, i: usize) -> usize { (*self).layer2(i) }
    #[inline] fn layer1(&self, i: usize) -> usize { (*self).layer1(i) }
    #[inline] fn layer0(&self, i: usize) -> usize { (*self).layer0(i) }
}

impl BitSetLike for BitSet {
    #[inline] fn layer3(&self) -> usize { self.layer3 }
    #[inline] fn layer2(&self, i: usize) -> usize { self.layer2.get(i).map(|&x| x).unwrap_or(0) }
    #[inline] fn layer1(&self, i: usize) -> usize { self.layer1.get(i).map(|&x| x).unwrap_or(0) }
    #[inline] fn layer0(&self, i: usize) -> usize { self.layer0.get(i).map(|&x| x).unwrap_or(0) }
}

/// `BitSetAnd` takes two `BitSetLike` items, and merges the masks
/// returning a new virtual set, which represents an intersection of the
/// two original sets.
pub struct BitSetAnd<A: BitSetLike, B: BitSetLike>(pub A, pub B);

impl<A: BitSetLike, B: BitSetLike> BitSetLike for BitSetAnd<A, B> {
    #[inline] fn layer3(&self) -> usize { self.0.layer3() & self.1.layer3() }
    #[inline] fn layer2(&self, i: usize) -> usize { self.0.layer2(i) & self.1.layer2(i) }
    #[inline] fn layer1(&self, i: usize) -> usize { self.0.layer1(i) & self.1.layer1(i) }
    #[inline] fn layer0(&self, i: usize) -> usize { self.0.layer0(i) & self.1.layer0(i) }
}

/// `BitSetOr` takes two `BitSetLike` items, and merges the masks
/// returning a new virtual set, which represents an merged of the
/// two original sets
pub struct BitSetOr<A: BitSetLike, B: BitSetLike>(pub A, pub B);

impl<A: BitSetLike, B: BitSetLike> BitSetLike for BitSetOr<A, B> {
    #[inline] fn layer3(&self) -> usize { self.0.layer3() | self.1.layer3() }
    #[inline] fn layer2(&self, i: usize) -> usize { self.0.layer2(i) | self.1.layer2(i) }
    #[inline] fn layer1(&self, i: usize) -> usize { self.0.layer1(i) | self.1.layer1(i) }
    #[inline] fn layer0(&self, i: usize) -> usize { self.0.layer0(i) | self.1.layer0(i) }
}


pub struct BitIter<T> {
    set: T,
    masks: [usize; 4],
    prefix: [u32; 3]
}

impl<T> Iterator for BitIter<T>
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

/// This is similar to a `BitSet` but allows setting of value
/// without unique ownership of the structure
///
/// An AtomicBitSet has the ability to add an item to the set
/// without unique ownership (given that the set is big enough).
/// Removing elements does require unique ownership as an effect
/// of the hierarchy it holds. Worst case multiple writers set the
/// same bit twice (but only is told they set it).
///
/// It is possible to atomically remove from the set, but not at the
/// same time as atomically adding. This is because there is no way
/// to know if layer 1-3 would be left in a consistent state if they are
/// being cleared and set at the same time.
///
/// `AtromicBitSet` resolves this race by disallowing atomic
/// clearing of bits.
pub struct AtomicBitSet {
    layer3: AtomicUsize,
    layer2: Vec<AtomicUsize>,
    layer1: Vec<AtomicUsize>,
    layer0: Vec<AtomicUsize>,
}

impl AtomicBitSet {
    /// Creates an empty `BitSet`.
    pub fn new() -> AtomicBitSet {
        AtomicBitSet {
            layer3: AtomicUsize::new(0),
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
    pub fn with_capacity(max: Index) -> AtomicBitSet {
        Self::valid_range(max);
        let mut value = AtomicBitSet::new();
        value.extend(max);
        value
    }

    #[inline(never)]
    fn extend(&mut self, id: Index) {
        Self::valid_range(id);
        let (p0, p1, p2) = offsets(id);

        if self.layer2.len() <= p2 {
            let count = p2 - self.layer2.len() + 1;
            self.layer2.extend(repeat(0).map(|_| AtomicUsize::new(0)).take(count));
        }
        if self.layer1.len() <= p1 {
            let count = p1 - self.layer1.len() + 1;
            self.layer1.extend(repeat(0).map(|_| AtomicUsize::new(0)).take(count));
        }
        if self.layer0.len() <= p0 {
            let count = p0 - self.layer0.len() + 1;
            self.layer0.extend(repeat(0).map(|_| AtomicUsize::new(0)).take(count));
        }
    }

    /// Adds `id` to the `AtomicBitSet`. Returns `true` if the value was
    /// already in the set.
    ///
    /// Because we cannot safely extend an AtomicBitSet without unique ownership
    /// this will panic if the Index is out of range
    #[inline]
    pub fn add_atomic(&self, id: Index) -> bool {
        let (p0, p1, p2) = offsets(id);

        // While it is tempting to check of the bit was set and exit here if it
        // was, this can result in a data race. If this thread and another
        // thread both set the same bit it is possible for the second thread
        // to exit before l3 was set. Resulting in the iterator to be in an
        // incorrect state. The window is small, but it exists.
        let old = self.layer0[p0].fetch_or(id.mask(SHIFT0), Ordering::Relaxed);
        self.layer1[p1].fetch_or(id.mask(SHIFT1), Ordering::Relaxed);
        self.layer2[p2].fetch_or(id.mask(SHIFT2), Ordering::Relaxed);
        self.layer3.fetch_or(id.mask(SHIFT3), Ordering::Relaxed);
        old & id.mask(SHIFT0) != 0
    }

    /// Adds `id` to the `BitSet`. Returns `true` if the value was
    /// already in the set.
    #[inline]
    pub fn add(&mut self, id: Index) -> bool {
        if id.offset(SHIFT1) >= self.layer0.len() {
            self.extend(id);
        }

        self.add_atomic(id)
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

        if self.layer0[p0].load(Ordering::Relaxed) & id.mask(SHIFT0) == 0 {
            return false;
        }

        // if the bitmask was set we need to clear
        // its bit from layer0 to 3. the layers abover only
        // should be cleared if the bit cleared was the last bit
        // in its set
        self.layer0[p0].fetch_and(!id.mask(SHIFT0), Ordering::Relaxed);
        if self.layer0[p0].load(Ordering::Relaxed) != 0 {
            return true;
        }

        self.layer1[p1].fetch_and(!id.mask(SHIFT1), Ordering::Relaxed);
        if self.layer1[p1].load(Ordering::Relaxed) != 0 {
            return true;
        }

        self.layer2[p2].fetch_and(!id.mask(SHIFT2), Ordering::Relaxed);
        if self.layer2[p2].load(Ordering::Relaxed) != 0 {
            return true;
        }

        self.layer3.fetch_and(!id.mask(SHIFT3), Ordering::Relaxed);
        return true;
    }

    /// Returns `true` if `id` is in the set.
    #[inline]
    pub fn contains(&self, id: u32) -> bool {
        let p0 = id.offset(SHIFT1);
        p0 < self.layer0.len() &&
            (self.layer0[p0].load(Ordering::Relaxed) & id.mask(SHIFT0)) != 0
    }

    /// Clear all bits in the set
    pub fn clear(&mut self) {
        use std::cmp::min;

        // This is the same hierarchical-striding used in the iterators.
        // Using this technique we can avoid clearing segments of the bitset
        // that are already clear. In the best case when the set is already cleared,
        // this will only touch the highest layer.

        let (mut m3, mut m2) = (self.layer3.swap(0, Ordering::Relaxed), 0usize);
        let mut offset = 0;

        loop {
            if m2 != 0 {
                let bit = m2.trailing_zeros() as usize;
                m2 &= !(1 << bit);

                // layer 1 & 0 are cleared unconditionally. it's only 32-64 words
                // and the extra logic to select the correct works is slower
                // then just clearing them all.
                self.layer1[offset + bit].store(0, Ordering::Relaxed);

                let start = (offset + bit) << BITS;
                let end = min(start + (1 << BITS), self.layer0.len());
                for l0 in &mut self.layer0[start..end] {
                    l0.store(0, Ordering::Relaxed);
                }
                continue;
            }

            if m3 != 0 {
                let bit = m3.trailing_zeros() as usize;
                m3 &= !(1 << bit);
                offset = bit << BITS;
                m2 = self.layer2[bit].swap(0, Ordering::Relaxed);
                continue;
            }
            break;
        }
    }

}

impl BitSetLike for AtomicBitSet {
    #[inline] fn layer3(&self) -> usize { self.layer3.load(Ordering::Relaxed) }
    #[inline] fn layer2(&self, i: usize) -> usize { self.layer2[i].load(Ordering::Relaxed) }
    #[inline] fn layer1(&self, i: usize) -> usize { self.layer1[i].load(Ordering::Relaxed) }
    #[inline] fn layer0(&self, i: usize) -> usize { self.layer0[i].load(Ordering::Relaxed) }
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

#[cfg(test)]
mod atomic_set_test {
    use super::{AtomicBitSet, BitSetAnd, BitSetLike};

    #[test]
    fn insert() {
        let mut c = AtomicBitSet::new();
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
        let mut c = AtomicBitSet::new();
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
        let mut c = AtomicBitSet::new();
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
        let mut c = AtomicBitSet::new();
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
        let mut odd = AtomicBitSet::new();
        let mut even = AtomicBitSet::new();
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

    #[test]
    fn clear() {
        let mut set = AtomicBitSet::new();
        for i in 0..1_000 {
            set.add(i);
        }

        assert_eq!((&set).iter().count(), 1_000);
        set.clear();
        assert_eq!((&set).iter().count(), 0);

        for i in 0..1_000 {
            set.add(i * 64);
        }

        assert_eq!((&set).iter().count(), 1_000);
        set.clear();
        assert_eq!((&set).iter().count(), 0);

        for i in 0..1_000 {
            set.add(i * 1_000);
        }

        assert_eq!((&set).iter().count(), 1_000);
        set.clear();
        assert_eq!((&set).iter().count(), 0);

        for i in 0..100 {
            set.add(i * 10_000);
        }

        assert_eq!((&set).iter().count(), 100);
        set.clear();
        assert_eq!((&set).iter().count(), 0);

        for i in 0..10 {
            set.add(i * 10_000);
        }

        assert_eq!((&set).iter().count(), 10);
        set.clear();
        assert_eq!((&set).iter().count(), 0);
    }

}
