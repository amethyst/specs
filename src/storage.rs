use std::collections::HashMap;

use {Entity, Generation};

pub trait Storage<T>: Sized {
    fn new() -> Self;
    fn get(&self, Entity) -> Option<&T>;
    fn get_mut(&mut self, Entity) -> Option<&mut T>;
    fn add(&mut self, Entity, T);
}

#[derive(Debug)]
pub struct VecStorage<T>(pub Vec<Option<(Generation, T)>>);

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
}

#[derive(Debug)]
pub struct HashMapStorage<T>(pub HashMap<Entity, T>);

impl<T> Storage<T> for HashMapStorage<T> {
    fn new() -> Self {
        HashMapStorage(HashMap::new())
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
}
