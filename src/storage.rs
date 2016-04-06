use std::collections::HashMap;

use Entity;

pub trait Storage<T>: Sized {
    fn new() -> Self;
    fn get(&self, Entity) -> Option<&T>;
    fn get_mut(&mut self, Entity) -> Option<&mut T>;
    fn add(&mut self, Entity, T);
}

#[derive(Debug)]
pub struct VecStorage<T>(pub Vec<Option<T>>);

impl<T> Storage<T> for VecStorage<T> {
    fn new() -> Self {
        VecStorage(Vec::new())
    }
    fn get(&self, entity: Entity) -> Option<&T> {
        self.0.get(entity as usize).and_then(|x| x.as_ref())
    }
    fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.0.get_mut(entity as usize).and_then(|x| x.as_mut())
    }
    fn add(&mut self, entity: Entity, value: T) {
        let id = entity as usize;
        while self.0.len() <= id {
            self.0.push(None);
        }
        self.0[id] = Some(value);
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
