use join::Join;
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

pub trait GenericWriteStorage<'b> {
    type Component: Component;
    type Join: Join;

    fn get_mut(&mut self, entity: Entity) -> Option<&mut Self::Component>;
    fn insert(&mut self, entity: Entity, comp: Self::Component);
    fn join(&'b mut self) -> Self::Join;
}

impl<'a: 'b, 'b, T> GenericWriteStorage<'b> for WriteStorage<'a, T>
where
    T: Component,
{
    type Component = T;
    type Join = &'b mut Self;

    fn get_mut(&mut self, entity: Entity) -> Option<&mut Self::Component> {
        WriteStorage::get_mut(self, entity)
    }

    fn insert(&mut self, entity: Entity, comp: Self::Component) {
        WriteStorage::insert(self, entity, comp);
    }

    fn join(&'b mut self) -> Self::Join {
        self
    }
}

impl<'a: 'b, 'b: 'c, 'c, T> GenericWriteStorage<'c> for &'b mut WriteStorage<'a, T>
where
    T: Component,
{
    type Component = T;
    type Join = &'c mut WriteStorage<'a, T>;

    fn get_mut(&mut self, entity: Entity) -> Option<&mut Self::Component> {
        WriteStorage::get_mut(*self, entity)
    }

    fn insert(&mut self, entity: Entity, comp: Self::Component) {
        WriteStorage::insert(*self, entity, comp);
    }

    fn join(&'c mut self) -> Self::Join {
        &mut **self
    }
}
