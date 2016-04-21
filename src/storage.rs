use std::collections::HashMap;
use std::hash::BuildHasherDefault;

use fnv::FnvHasher;

use {Entity, Index, Generation};
use bitset::BitSet;


/// Base trait for a component storage that is used as a trait object.
/// Doesn't depend on the actual component type.
pub trait StorageBase {
    /// Deletes a particular `Entity` from the storage.
    fn del(&mut self, Entity);
}

/// Typed component storage trait.
pub trait Storage: StorageBase + Sized {
    /// The Component to get or set
    type Component;
    /// Used during iterator
    type UnprotectedStorage: UnprotectedStorage<Component=Self::Component>;

    /// Creates a new `Storage<T>`. This is called when you register a new
    /// component type within the world.
    fn new() -> Self;
    /// Inserts new data for a given `Entity`.
    fn insert(&mut self, Entity, Self::Component);
    /// Tries to read the data associated with an `Entity`.
    fn get(&self, Entity) -> Option<&Self::Component>;
    /// Tries to mutate the data associated with an `Entity`.
    fn get_mut(&mut self, Entity) -> Option<&mut Self::Component>;
    /// Removes the data associated with an `Entity`.
    fn remove(&mut self, Entity) -> Option<Self::Component>;
    /// Splits the `BitSet` from the storage for use
    /// by the `Join` iterator.
    fn open(&self) -> (&BitSet, &Self::UnprotectedStorage);
    /// Splits the `BitSet` mutably from the storage for use
    /// by the `Join` iterator.
    fn open_mut(&mut self) -> (&BitSet, &mut Self::UnprotectedStorage);
}

/// Used by the framework to quickly join componets
pub trait UnprotectedStorage {
    /// The component to get
    type Component;
    /// Tries reading the data associated with an `Entity`.
    /// This is unsafe because the external set used
    /// to protect this storage is absent.
    unsafe fn get(&self, id: Index) -> &Self::Component;
    /// Tries mutating the data associated with an `Entity`.
    /// This is unsafe because the external set used
    /// to protect this storage is absent.
    unsafe fn get_mut(&mut self, id: Index) -> &mut Self::Component;
}

pub struct InnerHashMap<T>(HashMap<Index, GenerationData<T>, BuildHasherDefault<FnvHasher>>);

/// HashMap-based storage. Best suited for rare components.
pub struct HashMapStorage<T>{
    set: BitSet,
    map: InnerHashMap<T>
}

impl<T> StorageBase for HashMapStorage<T> {
    fn del(&mut self, entity: Entity) {
        if self.set.remove(entity.get_id() as u32) {
            self.map.0.remove(&(entity.get_id() as u32));
        }
    }
}

impl<T> Storage for HashMapStorage<T> {
    type Component = T;
    type UnprotectedStorage = InnerHashMap<T>;

    fn new() -> Self {
        let fnv = BuildHasherDefault::<FnvHasher>::default();
        HashMapStorage {
            set: BitSet::new(),
            map: InnerHashMap(HashMap::with_hasher(fnv))
        }
    }
    fn get(&self, entity: Entity) -> Option<&T> {
        self.map.0.get(&(entity.get_id() as u32))
            .and_then(|x| if x.generation == entity.get_gen() { Some(&x.data) } else { None })


    }
    fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.map.0.get_mut(&(entity.get_id() as u32))
            .and_then(|x| if x.generation == entity.get_gen() { Some(&mut x.data) } else { None })

    }
    fn insert(&mut self, entity: Entity, value: T) {
        let value = GenerationData{
            data: value,
            generation: entity.get_gen()
        };
        if self.map.0.insert(entity.get_id() as u32, value).is_none() {
            self.set.add(entity.get_id() as u32);
        }
    }
    fn remove(&mut self, entity: Entity) -> Option<T> {
        if self.set.remove(entity.get_id() as u32) {
            let value = self.map.0.remove(&(entity.get_id() as u32)).unwrap();
            if value.generation == entity.get_gen() {
                Some(value.data)
            } else {
                // it should be unlikely that the generation mismatch was
                // wrong, so this is a slow path to re-add the value back
                self.insert(entity, value.data);
                None
            }
        } else {
            None
        }
    }
    fn open(&self) -> (&BitSet, &Self::UnprotectedStorage) {
        (&self.set, &self.map)
    }
    fn open_mut(&mut self) -> (&BitSet, &mut Self::UnprotectedStorage) {
        (&self.set, &mut self.map)
    }
}

impl<T> UnprotectedStorage for InnerHashMap<T> {
    type Component = T;
    unsafe fn get(&self, e: Index) -> &T {
        &self.0.get(&e).unwrap().data
    }
    unsafe fn get_mut(&mut self, e: Index) -> &mut T {
        &mut self.0.get_mut(&e).unwrap().data
    }
}

pub struct InnerVec<T>(Vec<GenerationData<T>>);

pub struct GenerationData<T> {
    pub generation: Generation,
    pub data: T
}


/// Vec-based storage, stores the generations of the data in
/// order to match with given entities. Supposed to have maximum
/// performance for the components mostly present in entities.
pub struct VecStorage<T> {
    set: BitSet,
    values: InnerVec<T>,
}

impl<T> VecStorage<T> {
    fn extend(&mut self, id: usize) {
        debug_assert!(id >= self.values.0.len());
        let delta = (id + 1) - self.values.0.len();
        self.values.0.reserve(delta);
        unsafe {
            self.values.0.set_len(id + 1);
        }
    }
}

impl<T> Drop for VecStorage<T> {
    fn drop(&mut self) {
        use std::mem;

        for (i, v) in self.values.0.drain(..).enumerate() {
            // if v was not in the set the data is invalid
            // and we must forget it instead of dropping it
            if !self.set.remove(i as u32) {
                mem::forget(v);
            }
        }
    }
}

impl<T> super::StorageBase for VecStorage<T> {
    fn del(&mut self, e: Entity) {
        self.remove(e);
    }
}

impl<T> super::Storage for VecStorage<T> {
    type Component = T;
    type UnprotectedStorage = InnerVec<T>;

    fn new() -> Self {
        VecStorage {
            set: BitSet::new(),
            values: InnerVec(Vec::new()),
        }
    }
    fn get(&self, e: Entity) -> Option<&T> {
        let id = e.get_id();
        if self.set.contains(id as u32) {
            let v = unsafe { self.values.0.get_unchecked(id) };
            if v.generation == e.get_gen() {
                return Some(&v.data);
            }
        }
        None

    }
    fn get_mut(&mut self, e: Entity) -> Option<&mut T> {
        let id = e.get_id();
        if self.set.contains(id as u32) {
            let v = unsafe { self.values.0.get_unchecked_mut(id) };
            if v.generation == e.get_gen() {
                return Some(&mut v.data);
            }
        }
        None
    }
    fn insert(&mut self, e: Entity, mut v: T) {
        use std::{ptr, mem};

        let id = e.get_id();
        if self.set.contains(id as u32) {
            let mut data = &mut self.values.0[id];
            data.generation = e.get_gen();
            mem::swap(&mut data.data, &mut v);
            Some(v)
        } else {
            self.set.add(id as u32);
            if self.values.0.len() <= id {
                self.extend(id);
            }
            unsafe {
                ptr::write(
                    &mut self.values.0[id],
                    GenerationData{
                        generation: e.get_gen(),
                        data: v
                    }
                );
            }
            None
        };
    }
    fn remove(&mut self, e: Entity) -> Option<T> {
        use std::ptr;

        let (id, gen) = (e.get_id(), e.get_gen());
        let gen_matches = self.values.0.get(id)
            .map(|x| x.generation == gen).unwrap_or(false);

        if gen_matches && self.set.remove(id as u32) {
            let value = unsafe { ptr::read(&self.values.0[id]) };
            Some(value.data)
        } else {
            None
        }
    }
    fn open(&self) -> (&BitSet, &InnerVec<T>) {
        (&self.set, &self.values)
    }
    fn open_mut(&mut self) -> (&BitSet, &mut InnerVec<T>) {
        (&self.set, &mut self.values)
    }
}

impl<T> super::UnprotectedStorage for InnerVec<T> {
    type Component = T;
    unsafe fn get(&self, e: u32) -> &T {
        &self.0.get_unchecked(e as usize).data
    }
    unsafe fn get_mut(&mut self, e: u32) -> &mut T {
        &mut self.0.get_unchecked_mut(e as usize).data
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

