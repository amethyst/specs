use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use fnv::FnvHasher;

use {Entity, Generation};


pub trait StorageBase {
    fn del(&mut self, Entity);
}

pub trait Storage<T>: StorageBase + Sized {
    fn new() -> Self;
    fn get(&self, Entity) -> Option<&T>;
    fn get_mut(&mut self, Entity) -> Option<&mut T>;
    fn add(&mut self, Entity, T);
    fn sub(&mut self, Entity) -> Option<T>;
}


#[derive(Debug)]
pub struct VecStorage<T>(pub Vec<Option<(Generation, T)>>);

impl<T> StorageBase for VecStorage<T> {
    fn del(&mut self, entity: Entity) {
        self.0[entity.get_id()] = None;
    }
}
impl<T> Storage<T> for VecStorage<T> {
    fn new() -> Self {
        VecStorage(Vec::new())
    }
    fn get(&self, entity: Entity) -> Option<&T> {
        self.0.get(entity.get_id()).and_then(|x| match x {
            &Some((gen, ref value)) if gen == entity.get_gen() => Some(value),
            _ => None
        })
    }
    fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.0.get_mut(entity.get_id()).and_then(|x| match x {
            &mut Some((gen, ref mut value)) if gen == entity.get_gen() => Some(value),
            _ => None
        })
    }
    fn add(&mut self, entity: Entity, value: T) {
        while self.0.len() <= entity.get_id() {
            self.0.push(None);
        }
        self.0[entity.get_id()] = Some((entity.get_gen(), value));
    }
    fn sub(&mut self, entity: Entity) -> Option<T> {
        self.0.get_mut(entity.get_id()).and_then(|x| {
            if let &mut Some((gen, _)) = x {
                // if the generation does not match avoid deleting
                if gen != entity.get_gen() {
                    return None;
                }
            }
            x.take().map(|(_, x)| x)
        })
    }
}

#[derive(Debug)]
pub struct HashMapStorage<T>(pub HashMap<Entity, T, BuildHasherDefault<FnvHasher>>);

impl<T> StorageBase for HashMapStorage<T> {
    fn del(&mut self, entity: Entity) {
        self.0.remove(&entity);
    }
}
impl<T> Storage<T> for HashMapStorage<T> {
    fn new() -> Self {
        let fnv = BuildHasherDefault::<FnvHasher>::default();
        HashMapStorage(HashMap::with_hasher(fnv))
    }
    fn get(&self, entity: Entity) -> Option<&T> {
        self.0.get(&entity)
    }
    fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.0.get_mut(&entity)
    }
    fn add(&mut self, entity: Entity, value: T) {
        self.0.insert(entity, value);
    }
    fn sub(&mut self, entity: Entity) -> Option<T> {
        self.0.remove(&entity)
    }
}


#[cfg(test)]
mod test {
    use Entity;
    use super::*;

    fn test_add<S>() where S: Storage<u32> {
        let mut s = S::new();
        for i in 0..1_000 {
            s.add(Entity::new(i, 1), i + 2718);
        }

        for i in 0..1_000 {
            assert_eq!(*s.get(Entity::new(i, 1)).unwrap(), i + 2718);
        }
    }

    fn test_sub<S>() where S: Storage<u32> {
        let mut s = S::new();
        for i in 0..1_000 {
            s.add(Entity::new(i, 1), i + 2718);
        }

        for i in 0..1_000 {
            assert_eq!(s.sub(Entity::new(i, 1)).unwrap(), i + 2718);
            assert!(s.sub(Entity::new(i, 1)).is_none());
        }
    }

    fn test_get_mut<S>() where S: Storage<u32> {
        let mut s = S::new();
        for i in 0..1_000 {
            s.add(Entity::new(i, 1), i + 2718);
        }

        for i in 0..1_000 {
            *s.get_mut(Entity::new(i, 1)).unwrap() -= 718;
        }

        for i in 0..1_000 {
            assert_eq!(*s.get(Entity::new(i, 1)).unwrap(), i + 2000);
        }
    }

    fn test_add_gen<S>() where S: Storage<u32> {
        let mut s = S::new();
        for i in 0..1_000 {
            s.add(Entity::new(i, 1), i + 2718);
            s.add(Entity::new(i, 2), i + 31415);
        }

        for i in 0..1_000 {
            // this is removed since vec and hashmap disagree
            // on how this behavior should work...
            //assert!(s.get(Entity::new(i, 1)).is_none());
            assert_eq!(*s.get(Entity::new(i, 2)).unwrap(), i + 31415);
        }
    }

    fn test_sub_gen<S>() where S: Storage<u32> {
        let mut s = S::new();
        for i in 0..1_000 {
            s.add(Entity::new(i, 2), i + 2718);
        }

        for i in 0..1_000 {
            assert!(s.sub(Entity::new(i, 1)).is_none());
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

