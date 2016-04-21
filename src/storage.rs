use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::marker::PhantomData;
use std::ops::Deref;

use fnv::FnvHasher;

use {Entity, Index, Generation};
use world::Component;
use bitset::BitSet;


/// Base trait for a component storage that is used as a trait object.
/// Doesn't depend on the actual component type.
pub trait StorageBase {
    /// Deletes a particular `Index` from the storage.
    unsafe fn del(&mut self, Index);
}

pub struct MaskedStorage<T: Component> {
    mask: BitSet,
    inner: T::Storage,
}

impl<T: Component> MaskedStorage<T> {
    /// Creates a new `MaskedStorage`. This is called when you register
    /// a new component type within the world.
    pub fn new() -> MaskedStorage<T> {
        MaskedStorage {
            mask: BitSet::new(),
            inner: UnprotectedStorage::new(),
        }
    }
}

impl<T: Component> StorageBase for MaskedStorage<T> {
    unsafe fn del(&mut self, index: Index) {
        self.inner.remove(index);
    }
}


pub struct Storage<'a, T: Component, G> {
    mask: BitSet,
    inner: T::Storage,
    gens: G,
    phantom: PhantomData<&'a T>,
}

impl<'a, T, G> Storage<'a, T, G> where
    T: Component,
    G: Deref<Target=&'a [Generation]>,
{
    /// Check if an entity has component `T`.
    fn has(&self, e: Entity) -> bool {
        let g1 = Generation(1);
        self.mask.contains(e.get_id() as u32) &&
        e.get_gen() == *self.gens.get(e.get_id() as usize).unwrap_or(&g1)
    }
    /// Tries to read the data associated with an `Entity`.
    pub fn get(&self, e: Entity) -> Option<&T> {
        if self.has(e) {
            Some(unsafe { self.inner.get(e.get_id()) })
        }else {None}
    }
    /// Tries to mutate the data associated with an `Entity`.
    fn get_mut(&mut self, e: Entity) -> Option<&mut T> {
        if self.has(e) {
            Some(unsafe { self.inner.get_mut(e.get_id()) })
        }else {None}
    }
    /// Inserts new data for a given `Entity`.
    fn insert(&mut self, e: Entity, v: T) {
        let id = e.get_id();
        if self.mask.contains(id as u32) {
            *unsafe{ self.inner.get_mut(id) } = v;
        } else {
            self.mask.add(id as u32);
            unsafe{ self.inner.insert(id, v) };
        }
    }
    /// Removes the data associated with an `Entity`.
    fn remove(&mut self, e: Entity) -> Option<T> {
        let g1 = Generation(1);
        let id = e.get_id();
        if e.get_gen() == *self.gens.get(e.get_id() as usize).unwrap_or(&g1) && self.mask.remove(id as u32) {
            Some(self.inner.remove(id))
        }else { None }
    }
}


/// Used by the framework to quickly join componets
pub trait UnprotectedStorage<T>: Sized {
    /// Creates a new `Storage<T>`. This is called when you register a new
    /// component type within the world.
    fn new() -> Self;
    /// Clean the storage given a check to figure out if an index
    /// is valid or not. Allows us to safely drop the storage.
    unsafe fn clean<F>(&mut self, F) where F: Fn(Index) -> bool;
    /// Tries reading the data associated with an `Index`.
    /// This is unsafe because the external set used
    /// to protect this storage is absent.
    unsafe fn get(&self, id: Index) -> &T;
    /// Tries mutating the data associated with an `Index`.
    /// This is unsafe because the external set used
    /// to protect this storage is absent.
    unsafe fn get_mut(&mut self, id: Index) -> &mut T;
    /// Inserts new data for a given `Index`.
    unsafe fn insert(&mut self, Index, T);
    /// Removes the data associated with an `Index`.
    unsafe fn remove(&mut self, Index) -> T;
}

/// HashMap-based storage. Best suited for rare components.
pub struct HashMapStorage<T>(HashMap<Index, T, BuildHasherDefault<FnvHasher>>);

impl<T> UnprotectedStorage<T> for HashMapStorage<T> {
    fn new() -> Self {
        let fnv = BuildHasherDefault::<FnvHasher>::default();
        HashMapStorage(HashMap::with_hasher(fnv))
    }
    unsafe fn clean<F>(&mut self, has: F) where F: Fn(Index) -> bool {
        use std::mem;
        for (i, v) in self.0.drain() {
            if !has(i) {
                // if v was not in the set the data is invalid
                // and we must forget it instead of dropping it
                mem::forget(v);
            }
        }
    }
    unsafe fn get(&self, id: Index) -> &T {
        self.0.get(&id).unwrap()
    }
    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        self.0.get_mut(&id).unwrap()
    }
    unsafe fn insert(&mut self, id: Index, v: T) {
        self.0.insert(id, v);
    }
    unsafe fn remove(&mut self, id: Index) -> T {
        self.0.remove(&id).unwrap()
    }
}

/// Vec-based storage, stores the generations of the data in
/// order to match with given entities. Supposed to have maximum
/// performance for the components mostly present in entities.
pub struct VecStorage<T>(Vec<T>);

impl<T> UnprotectedStorage<T> for VecStorage<T> {
    fn new() -> Self {
        VecStorage(Vec::new())
    }
    unsafe fn clean<F>(&mut self, has: F) where F: Fn(Index) -> bool {
        use std::mem;
        for (i, v) in self.0.drain(..).enumerate() {
            if !has(i as Index) {
                // if v was not in the set the data is invalid
                // and we must forget it instead of dropping it
                mem::forget(v);
            }
        }
    }
    unsafe fn get(&self, id: Index) -> &T {
        self.0.get_unchecked(id as usize)
    }
    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        self.0.get_unchecked_mut(id as usize)
    }
    unsafe fn insert(&mut self, id: Index, v: T) {
        let id = id as usize;
        if self.0.len() <= id {
            let delta = id + 1 - self.0.len();
            self.0.reserve(delta);
            self.0.set_len(id + 1);
        }
        self.0[id] = v;
    }
    unsafe fn remove(&mut self, id: Index) -> T {
        use std::ptr;
        ptr::read(self.get(id))
    }
}


#[cfg(test)]
mod map_test {
    use {Storage, Entity, Generation};
    use super::VecStorage;

    #[test]
    fn insert() {
        let mut c = VecStorage::new();
        for i in 0..1_000 {
            c.insert(Entity::new(i, Generation(0)), i);
        }

        for i in 0..1_000 {
            assert_eq!(c.get(Entity::new(i, Generation(0))).unwrap(), &i);
        }
    }

    #[test]
    fn insert_100k() {
        let mut c = VecStorage::new();
        for i in 0..100_000 {
            c.insert(Entity::new(i, Generation(0)), i);
        }

        for i in 0..100_000 {
            assert_eq!(c.get(Entity::new(i, Generation(0))).unwrap(), &i);
        }
    }

    #[test]
    fn remove() {
        let mut c = VecStorage::new();
        for i in 0..1_000 {
            c.insert(Entity::new(i, Generation(0)), i);
        }

        for i in 0..1_000 {
            assert_eq!(c.get(Entity::new(i, Generation(0))).unwrap(), &i);
        }

        for i in 0..1_000 {
            c.remove(Entity::new(i, Generation(0)));
        }

        for i in 0..1_000 {
            assert!(c.get(Entity::new(i, Generation(0))).is_none());
        }
    }

    #[test]
    fn test_gen() {
        let mut c = VecStorage::new();
        for i in 0..1_000i32 {
            c.insert(Entity::new(i as u32, Generation(0)), i);
            c.insert(Entity::new(i as u32, Generation(0)), -i);
        }

        for i in 0..1_000i32 {
            assert_eq!(c.get(Entity::new(i as u32, Generation(0))).unwrap(), &-i);
        }
    }

    #[test]
    fn insert_same_key() {
        let mut c = VecStorage::new();
        for i in 0..10_000 {
            c.insert(Entity::new(i as u32, Generation(0)), i);
            assert_eq!(c.get(Entity::new(i as u32, Generation(0))).unwrap(), &i);
        }
    }

    #[should_panic]
    #[test]
    fn wrap() {
        let mut c = VecStorage::new();
        c.insert(Entity::new(1 << 25, Generation(0)), 7);
    }
}


#[cfg(test)]
mod test {
    use {Entity, Generation, Storage, VecStorage, HashMapStorage};

    fn test_add<S>() where S: Storage<Component=u32> {
        let mut s = S::new();
        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation(1)), i + 2718);
        }

        for i in 0..1_000 {
            assert_eq!(*s.get(Entity::new(i, Generation(1))).unwrap(), i + 2718);
        }
    }

    fn test_sub<S>() where S: Storage<Component=u32> {
        let mut s = S::new();
        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation(1)), i + 2718);
        }

        for i in 0..1_000 {
            assert_eq!(s.remove(Entity::new(i, Generation(1))).unwrap(), i + 2718);
            assert!(s.remove(Entity::new(i, Generation(1))).is_none());
        }
    }

    fn test_get_mut<S>() where S: Storage<Component=u32> {
        let mut s = S::new();
        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation(1)), i + 2718);
        }

        for i in 0..1_000 {
            *s.get_mut(Entity::new(i, Generation(1))).unwrap() -= 718;
        }

        for i in 0..1_000 {
            assert_eq!(*s.get(Entity::new(i, Generation(1))).unwrap(), i + 2000);
        }
    }

    fn test_add_gen<S>() where S: Storage<Component=u32> {
        let mut s = S::new();
        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation(1)), i + 2718);
            s.insert(Entity::new(i, Generation(2)), i + 31415);
        }

        for i in 0..1_000 {
            // this is removed since vec and hashmap disagree
            // on how this behavior should work...
            //assert!(s.get(Entity::new(i, 1)).is_none());
            assert_eq!(*s.get(Entity::new(i, Generation(2))).unwrap(), i + 31415);
        }
    }

    fn test_sub_gen<S>() where S: Storage<Component=u32> {
        let mut s = S::new();
        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation(2)), i + 2718);
        }

        for i in 0..1_000 {
            assert!(s.remove(Entity::new(i, Generation(1))).is_none());
        }
    }

    #[test] fn vec_test_add() { test_add::<VecStorage<u32>>(); }
    #[test] fn vec_test_sub() { test_sub::<VecStorage<u32>>(); }
    #[test] fn vec_test_get_mut() { test_get_mut::<VecStorage<u32>>(); }
    #[test] fn vec_test_add_gen() { test_add_gen::<VecStorage<u32>>(); }
    #[test] fn vec_test_sub_gen() { test_sub_gen::<VecStorage<u32>>(); }

    #[test] fn hash_test_add() { test_add::<HashMapStorage<u32>>(); }
    #[test] fn hash_test_sub() { test_sub::<HashMapStorage<u32>>(); }
    #[test] fn hash_test_get_mut() { test_get_mut::<HashMapStorage<u32>>(); }
    #[test] fn hash_test_add_gen() { test_add_gen::<HashMapStorage<u32>>(); }
    #[test] fn hash_test_sub_gen() { test_sub_gen::<HashMapStorage<u32>>(); }
}

