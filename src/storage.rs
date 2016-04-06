use Entity;

pub trait Storage<T> {
    fn get(&self, Entity) -> Option<&T>;
    fn get_mut(&mut self, Entity) -> Option<&mut T>;
}

pub struct VecStorage<T> {
    data: Vec<T>,
}

impl<T> Storage<T> for VecStorage<T> {
    fn get(&self, entity: Entity) -> Option<&T> {
        self.data.get(entity as usize)
    }
    fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.data.get_mut(entity as usize)
    }
}
