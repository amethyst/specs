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

    /// Get mutable access to an `Entity`s component. If the component does not exist, it
    /// is automatically created using `Default::default()`.
    /// 
    /// # Panics
    /// Panics if the component insertion or recovery fails. This should never happen, but
    /// is handled just in case.
    fn get_mut_or_default(&mut self, entity: Entity) -> &mut Self::Component where Self::Component: Default;

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

    fn get_mut_or_default(&mut self, entity: Entity) -> &mut Self::Component where Self::Component: Default {
        if !self.contains(entity) {
            self.insert(entity, Default::default()).expect("Insertion of default component failed");
        }
        // At this point it is reasonably safe to assume the component exists.
        self.get_mut(entity).expect("Fetching a default component failed")
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

    fn get_mut_or_default(&mut self, entity: Entity) -> &mut Self::Component where Self::Component: Default {
        if !self.contains(entity) {
            self.insert(entity, Default::default()).expect("Insertion of default component failed");
        }
        // At this point it is reasonably safe to assume the component exists.
        self.get_mut(entity).expect("Fetching a default component failed")
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
