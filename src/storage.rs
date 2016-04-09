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
    fn sub(&mut self, entity: Entity) -> Option<T>{
        self.0[entity.get_id()].take().map(|(g, v)| {
            assert_eq!(g, entity.get_gen());
            v
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
