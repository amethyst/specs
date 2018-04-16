use storage::{ReadStorage, WriteStorage};
use world::{Component, Entity};

pub trait GenericReadStorage {
    type Component: Component;

    fn get(&self, entity: Entity) -> Option<&Self::Component>;
}

impl<'a, T> GenericReadStorage for ReadStorage<'a, T>
where
    T: Component,
{
    type Component = T;

    fn get(&self, entity: Entity) -> Option<&Self::Component> {
        ReadStorage::get(self, entity)
    }
}

impl<'a: 'b, 'b, T> GenericReadStorage for &'b ReadStorage<'a, T>
where
    T: Component,
{
    type Component = T;

    fn get(&self, entity: Entity) -> Option<&Self::Component> {
        ReadStorage::get(*self, entity)
    }
}

impl<'a, T> GenericReadStorage for WriteStorage<'a, T>
where
    T: Component,
{
    type Component = T;

    fn get(&self, entity: Entity) -> Option<&Self::Component> {
        WriteStorage::get(self, entity)
    }
}

impl<'a: 'b, 'b, T> GenericReadStorage for &'b WriteStorage<'a, T>
where
    T: Component,
{
    type Component = T;

    fn get(&self, entity: Entity) -> Option<&Self::Component> {
        WriteStorage::get(*self, entity)
    }
}

pub trait GenericWriteStorage {
    type Component: Component;

    fn get_mut(&mut self, entity: Entity) -> Option<&mut Self::Component>;
    fn insert(&mut self, entity: Entity, comp: Self::Component);
    fn remove(&mut self, entity: Entity);
}

impl<'a, T> GenericWriteStorage for WriteStorage<'a, T>
where
    T: Component,
{
    type Component = T;

    fn get_mut(&mut self, entity: Entity) -> Option<&mut Self::Component> {
        WriteStorage::get_mut(self, entity)
    }

    fn insert(&mut self, entity: Entity, comp: Self::Component) {
        WriteStorage::insert(self, entity, comp);
    }

    fn remove(&mut self, entity: Entity) {
        WriteStorage::remove(self, entity);
    }
}

impl<'a: 'b, 'b, T> GenericWriteStorage for &'b mut WriteStorage<'a, T>
where
    T: Component,
{
    type Component = T;

    fn get_mut(&mut self, entity: Entity) -> Option<&mut Self::Component> {
        WriteStorage::get_mut(*self, entity)
    }

    fn insert(&mut self, entity: Entity, comp: Self::Component) {
        WriteStorage::insert(*self, entity, comp);
    }

    fn remove(&mut self, entity: Entity) {
        WriteStorage::remove(*self, entity);
    }
}
