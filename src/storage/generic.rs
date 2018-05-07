use storage::{InsertResult, ReadStorage, WriteStorage};
use world::{Component, Entity};

pub struct Seal;

/// Provides generic read access to both `ReadStorage` and `WriteStorage`
pub trait GenericReadStorage {
    /// The component type of the storage
    type Component: Component;

    /// Get immutable access to an `Entity`s component
    fn get(&self, entity: Entity) -> Option<&Self::Component>;

    /// Private function to seal the trait
    fn _private() -> Seal;
}

impl<'a, T> GenericReadStorage for ReadStorage<'a, T>
where
    T: Component,
{
    type Component = T;

    fn get(&self, entity: Entity) -> Option<&Self::Component> {
        ReadStorage::get(self, entity)
    }

    fn _private() -> Seal {
        Seal
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

    fn _private() -> Seal {
        Seal
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

    fn _private() -> Seal {
        Seal
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

    fn _private() -> Seal {
        Seal
    }
}

/// Provides generic write access to `WriteStorage`, both as a value and a mutable reference.
pub trait GenericWriteStorage {
    /// The component type of the storage
    type Component: Component;

    /// Get mutable access to an `Entity`s component
    fn get_mut(&mut self, entity: Entity) -> Option<&mut Self::Component>;

    /// Insert a component for an `Entity`
    fn insert(&mut self, entity: Entity, comp: Self::Component) -> InsertResult<Self::Component>;

    /// Remove the component for an `Entity`
    fn remove(&mut self, entity: Entity);

    /// Private function to seal the trait
    fn _private() -> Seal;
}

impl<'a, T> GenericWriteStorage for WriteStorage<'a, T>
where
    T: Component,
{
    type Component = T;

    fn get_mut(&mut self, entity: Entity) -> Option<&mut Self::Component> {
        WriteStorage::get_mut(self, entity)
    }

    fn insert(&mut self, entity: Entity, comp: Self::Component) -> InsertResult<Self::Component> {
        WriteStorage::insert(self, entity, comp)
    }

    fn remove(&mut self, entity: Entity) {
        WriteStorage::remove(self, entity);
    }

    fn _private() -> Seal {
        Seal
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

    fn insert(&mut self, entity: Entity, comp: Self::Component) -> InsertResult<Self::Component> {
        WriteStorage::insert(*self, entity, comp)
    }

    fn remove(&mut self, entity: Entity) {
        WriteStorage::remove(*self, entity);
    }

    fn _private() -> Seal {
        Seal
    }
}
