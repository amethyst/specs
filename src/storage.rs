use std::collections::HashMap;

use Entity;

pub trait Storage<T>: Sized {
    fn new() -> Self;
    fn get(&self, Entity) -> Option<&T>;
    fn get_mut(&mut self, Entity) -> Option<&mut T>;
    fn add(&mut self, Entity, T);
}

#[derive(Debug)]
pub struct VecStorage<T> {
    data: Vec<Option<T>>,
}

impl<T> Storage<T> for VecStorage<T> {
    fn new() -> Self {
        VecStorage {
            data: Vec::new(),
        }
    }
    fn get(&self, entity: Entity) -> Option<&T> {
        self.data.get(entity as usize).and_then(|x| x.as_ref())
    }
    fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.data.get_mut(entity as usize).and_then(|x| x.as_mut())
    }
    fn add(&mut self, entity: Entity, data: T) {
        let id = entity as usize;
        while self.data.len() <= id {
            self.data.push(None);
        }
        self.data[id] = Some(data);
    }
}
