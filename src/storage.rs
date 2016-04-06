use Entity;

pub trait Storage<T> {
    fn get(&self, Entity) -> Option<&T>;
    fn get_mut(&mut self, Entity) -> Option<&mut T>;
    fn add(&mut self, Entity, T);
}

#[derive(Debug)]
pub struct VecStorage<T> {
    data: Vec<Option<T>>,
}

impl<T> Default for VecStorage<T> {
    fn default() -> VecStorage<T> {
        VecStorage { data: Vec::new() }
    }
}

impl<T: Clone> Storage<T> for VecStorage<T> {
    fn get(&self, entity: Entity) -> Option<&T> {
        self.data.get(entity as usize).and_then(|x| x.as_ref())
    }
    fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.data.get_mut(entity as usize).and_then(|x| x.as_mut())
    }
    fn add(&mut self, entity: Entity, data: T) {
        let id = entity as usize;
        if self.data.len() <= id {
            self.data.resize(id+1, None)
        }
        self.data[id] = Some(data);
    }
}
