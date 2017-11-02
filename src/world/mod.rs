pub use self::comp::Component;
pub use self::entity::{Allocator, CreateIterAtomic, EntitiesRes, Entity, EntityIndex, Generation};
pub use self::lazy::LazyUpdate;

use std::borrow::Borrow;

use shred::{Fetch, FetchMut, Resource, Resources};

use {ReadStorage, Storage, WriteStorage};
use error::WrongGeneration;
use storage::{AnyStorage, DenseVecStorage, MaskedStorage};

mod comp;
mod entity;
mod lazy;
#[cfg(test)]
mod tests;

const COMPONENT_NOT_REGISTERED: &str = "No component with the given id. Did you forget to register \
                                        the component with `World::register::<ComponentName>()`?";
const RESOURCE_NOT_ADDED: &str = "No resource with the given id. Did you forget to add \
                                  the resource with `World::add_resource(resource)`?";

/// An iterator for entity creation.
/// Please note that you have to consume
/// it because iterators are lazy.
///
/// Returned from `World::create_iter`.
pub struct CreateIter<'a>(FetchMut<'a, EntitiesRes>);

impl<'a> Iterator for CreateIter<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Entity> {
        Some(self.0.alloc.allocate())
    }
}

/// The entity builder, allowing to
/// build an entity together with its components.
///
/// ## Examples
///
/// ```
/// use specs::{Component, DenseVecStorage, HashMapStorage, World};
///
/// struct Health(f32);
///
/// impl Component for Health {
///     type Storage = HashMapStorage<Self>;
/// }
///
/// struct Pos {
///     x: f32,
///     y: f32,
/// }
///
/// impl Component for Pos {
///     type Storage = DenseVecStorage<Self>;
/// }
///
/// let mut world = World::new();
/// world.register::<Health>();
/// world.register::<Pos>();
///
/// let entity = world
///     .create_entity() // This call returns `EntityBuilder`
///     .with(Health(4.0))
///     .with(Pos { x: 1.0, y: 3.0 })
///     .build(); // Returns the `Entity`
/// ```
pub struct EntityBuilder<'a> {
    /// The (already created) entity for which components will be inserted.
    pub entity: Entity,
    /// A reference to the `World` for component insertions.
    pub world: &'a World,
}

impl<'a> EntityBuilder<'a> {
    /// Appends a component with the default component id.
    ///
    /// # Panics
    ///
    /// Panics if the component hasn't been `register()`ed in the
    /// `World`.
    #[inline]
    pub fn with<T: Component>(self, c: T) -> Self {
        self.with_id(c, 0)
    }

    /// Appends a component with a component id.
    ///
    /// # Panics
    ///
    /// Panics if the component hasn't been `register_with_id()`ed in the
    /// `World`.
    #[inline]
    pub fn with_id<T: Component>(self, c: T, id: usize) -> Self {
        {
            let mut storage = self.world.write_with_id(id);
            storage.insert(self.entity, c);
        }

        self
    }

    /// Finishes the building and returns the entity.
    #[inline]
    pub fn build(self) -> Entity {
        self.entity
    }
}

/// The `World` struct contains the component storages and
/// other resources.
///
/// Many methods take `&self` which works because everything
/// is stored with **interior mutability**. In case you violate
/// the borrowing rules of Rust (multiple reads xor one write),
/// you will get a panic.
///
/// ## Component / Resources ids
///
/// Components and resources may, in addition to their type, be identified
/// by an id of type `usize`. The convenience methods dealing
/// with components assume that it's `0`.
///
/// If a system attempts to access a component/resource that has not been
/// registered/added, it will panic when run. Add all components with
/// `World::register` before running any systems. Also add all resources
/// with `World::add_resource`.
///
/// ## Examples
///
/// ```
/// # use specs::{Component, VecStorage};
/// # struct Pos { x: f32, y: f32, } impl Component for Pos { type Storage = VecStorage<Self>; }
/// # struct Vel { x: f32, y: f32, } impl Component for Vel { type Storage = VecStorage<Self>; }
/// # struct DeltaTime(f32);
/// use specs::World;
///
/// let mut world = World::new();
/// world.register::<Pos>();
/// world.register::<Vel>();
///
/// world.add_resource(DeltaTime(0.02));
///
/// world
///     .create_entity()
///     .with(Pos { x: 1.0, y: 2.0 })
///     .with(Vel { x: -1.0, y: 0.0 })
///     .build();
///
/// world
///     .create_entity()
///     .with(Pos { x: 3.0, y: 5.0 })
///     .with(Vel { x: 1.0, y: 0.0 })
///     .build();
///
/// world
///     .create_entity()
///     .with(Pos { x: 0.0, y: 1.0 })
///     .with(Vel { x: 0.0, y: 1.0 })
///     .build();
/// ```
pub struct World {
    /// The resources used for this world.
    pub res: Resources,
    storages: Vec<*mut AnyStorage>,
}

impl World {
    /// Creates a new empty `World`.
    pub fn new() -> World {
        Default::default()
    }

    /// Registers a new component.
    ///
    /// Calls `register_with_id` with id `0`, which
    /// is the default for component ids.
    ///
    /// Does nothing if the component was already
    /// registered.
    ///
    /// ## Examples
    ///
    /// ```
    /// use specs::{Component, DenseVecStorage, World};
    ///
    /// struct Pos {
    ///     x: f32,
    ///     y: f32,
    /// }
    ///
    /// impl Component for Pos {
    ///     type Storage = DenseVecStorage<Self>;
    /// }
    ///
    /// let mut world = World::new();
    /// world.register::<Pos>();
    /// // Register all other components like this
    /// ```
    pub fn register<T: Component>(&mut self) {
        self.register_with_id::<T>(0);
    }

    /// Registers a new component with a given id.
    ///
    /// Does nothing if the component was already
    /// registered.
    pub fn register_with_id<T: Component>(&mut self, id: usize) {
        use shred::ResourceId;

        if self.res
            .has_value(ResourceId::new_with_id::<MaskedStorage<T>>(id))
        {
            return;
        }

        self.res.add_with_id(MaskedStorage::<T>::new(), id);

        let mut storage = self.res.fetch_mut::<MaskedStorage<T>>(id);
        self.storages.push(&mut *storage as *mut AnyStorage);
    }

    /// Adds a resource with the default ID (`0`).
    ///
    /// If the resource already exists it will be overwritten.
    ///
    /// ## Difference between resources and components
    ///
    /// While components exist per entity, resources are like globals in the `World`.
    /// Components are stored in component storages, which are resources themselves.
    ///
    /// Everything that is `Any + Send + Sync` can be a resource.
    ///
    /// ## Built-in resources
    ///
    /// There are two built-in resources:
    ///
    /// * `LazyUpdate` and
    /// * `EntitiesRes`
    ///
    /// Both of them should only be fetched immutably, which is why
    /// the latter one has a type def for convenience: `Entities` which
    /// is just `Fetch<EntitiesRes>`. Both resources are special and need
    /// to execute code at the end of the frame, which is done in `World::maintain`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use specs::World;
    ///
    /// # let timer = ();
    /// # let server_con = ();
    /// let mut world = World::new();
    /// world.add_resource(timer);
    /// world.add_resource(server_con);
    /// ```
    pub fn add_resource<T: Resource>(&mut self, res: T) {
        self.add_resource_with_id(res, 0);
    }

    /// Adds a resource with a given ID.
    ///
    /// If the resource already exists it will be overwritten.
    pub fn add_resource_with_id<T: Resource>(&mut self, res: T, id: usize) {
        use shred::ResourceId;

        if self.res.has_value(ResourceId::new_with_id::<T>(id)) {
            *self.write_resource_with_id(id) = res;
        } else {
            self.res.add_with_id(res, id);
        }
    }

    /// Fetches a component's storage with the default id for reading.
    ///
    /// Convenience method for `read_with_id`, using the default component
    /// id (`0`).
    ///
    /// ## Panics
    ///
    /// Panics if it is already borrowed mutably.
    /// Panics if the component has not been registered.
    pub fn read<T: Component>(&self) -> ReadStorage<T> {
        self.read_with_id(0)
    }

    /// Fetches a component's storage with the default id for writing.
    ///
    /// Convenience method for `write_with_id`, using the default component
    /// id (`0`).
    ///
    /// ## Panics
    ///
    /// Panics if it is already borrowed.
    /// Panics if the component has not been registered.
    pub fn write<T: Component>(&self) -> WriteStorage<T> {
        self.write_with_id(0)
    }

    /// Fetches a component's storage with a specified id for reading.
    ///
    /// ## Panics
    ///
    /// Panics if it is already borrowed mutably.
    /// Also panics if the component is not registered with `World::register`.
    pub fn read_with_id<T: Component>(&self, id: usize) -> ReadStorage<T> {
        let entities = self.entities();
        let resource = self.res.try_fetch::<MaskedStorage<T>>(id);

        Storage::new(entities, resource.expect(COMPONENT_NOT_REGISTERED))
    }

    /// Fetches a component's storage with a specified id for writing.
    ///
    /// # Panics
    ///
    /// Panics if it is already borrowed.
    /// Also panics if the component is not registered with `World::register`.
    pub fn write_with_id<T: Component>(&self, id: usize) -> WriteStorage<T> {
        let entities = self.entities();
        let resource = self.res.try_fetch_mut::<MaskedStorage<T>>(id);

        Storage::new(entities, resource.expect(COMPONENT_NOT_REGISTERED))
    }

    /// Fetches a resource with a specified id for reading.
    ///
    /// ## Panics
    ///
    /// Panics if it is already borrowed mutably.
    /// Panics if the resource has not been added.
    pub fn read_resource_with_id<T: Resource>(&self, id: usize) -> Fetch<T> {
        self.res.try_fetch(id).expect(RESOURCE_NOT_ADDED)
    }

    /// Fetches a resource with a specified id for writing.
    ///
    /// ## Panics
    ///
    /// Panics if it is already borrowed.
    /// Panics if the resource has not been added.
    pub fn write_resource_with_id<T: Resource>(&self, id: usize) -> FetchMut<T> {
        self.res.try_fetch_mut(id).expect(RESOURCE_NOT_ADDED)
    }

    /// Fetches a resource with the default id for reading.
    ///
    /// Convenience method for `read_resource_with_id`, using the default component
    /// id (`0`).
    ///
    /// ## Panics
    ///
    /// Panics if it is already borrowed mutably.
    /// Panics if the resource has not been added.
    pub fn read_resource<T: Resource>(&self) -> Fetch<T> {
        self.read_resource_with_id(0)
    }

    /// Fetches a resource with the default id for writing.
    ///
    /// Convenience method for `write_resource_with_id`, using the default component
    /// id (`0`).
    ///
    /// # Panics
    ///
    /// Panics if it is already borrowed.
    /// Panics if the resource has not been added.
    pub fn write_resource<T: Resource>(&self) -> FetchMut<T> {
        self.write_resource_with_id(0)
    }

    /// Convenience method for fetching entities.
    ///
    /// Creation and deletion of entities with the `Entities` struct
    /// are atomically, so the actual changes will be applied
    /// with the next call to `maintain()`.
    pub fn entities(&self) -> Fetch<EntitiesRes> {
        self.read_resource()
    }

    /// Convenience method for fetching entities.
    fn entities_mut(&self) -> FetchMut<EntitiesRes> {
        self.write_resource()
    }

    /// Allows building an entity with its components.
    ///
    /// This takes a mutable reference to the `World`, since no
    /// component storage this builder accesses may be borrowed.
    /// If it's necessary that you borrow a resource from the `World`
    /// while this builder is alive, you can use `create_entity_unchecked`.
    pub fn create_entity(&mut self) -> EntityBuilder {
        self.create_entity_unchecked()
    }

    /// Allows building an entity with its components.
    ///
    /// **You have to make sure that no component storage is borrowed
    /// during the building!**
    ///
    /// This variant is only recommended if you need to borrow a resource
    /// during the entity building. If possible, try to use `create_entity`.
    pub fn create_entity_unchecked(&self) -> EntityBuilder {
        let entity = self.entities_mut().alloc.allocate();

        EntityBuilder {
            entity,
            world: self,
        }
    }

    /// Returns an iterator for entity creation.
    /// This makes it easy to create a whole collection
    /// of them.
    ///
    /// ## Examples
    ///
    /// ```
    /// use specs::World;
    ///
    /// let mut world = World::new();
    /// let five_entities: Vec<_> = world.create_iter().take(5).collect();
    /// #
    /// # assert_eq!(five_entities.len(), 5);
    /// ```
    pub fn create_iter(&mut self) -> CreateIter {
        CreateIter(self.entities_mut())
    }

    /// Deletes an entity and its components.
    pub fn delete_entity(&mut self, entity: Entity) -> Result<(), WrongGeneration> {
        self.delete_entities(&[entity])
    }

    /// Deletes the specified entities and their components.
    pub fn delete_entities(&mut self, delete: &[Entity]) -> Result<(), WrongGeneration> {
        self.delete_components(delete);

        self.entities_mut().alloc.kill(delete)
    }

    /// Deletes all entities and their components.
    pub fn delete_all(&mut self) {
        use join::Join;

        let entities: Vec<_> = (&*self.entities()).join().collect();

        self.delete_entities(&entities).expect(
            "Bug: previously collected entities are not valid \
             even though access should be exclusive",
        );
    }

    /// Checks if an entity is alive.
    /// Please note that atomically created or deleted entities
    /// (the ones created / deleted with the `Entities` struct)
    /// are not handled by this method. Therefore, you
    /// should have called `maintain()` before using this
    /// method.
    ///
    /// If you want to get this functionality before a `maintain()`,
    /// you are most likely in a system; from there, just access the
    /// `Entities` resource and call the `is_alive` method.
    ///
    /// # Panics
    ///
    /// Panics if generation is dead.
    pub fn is_alive(&self, e: Entity) -> bool {
        assert!(e.gen().is_alive(), "Generation is dead");

        let alloc: &Allocator = &self.entities().alloc;
        alloc
            .generations
            .get(e.id() as usize)
            .map(|&x| x == e.gen())
            .unwrap_or(false)
    }

    /// Merges in the appendix, recording all the dynamically created
    /// and deleted entities into the persistent generations vector.
    /// Also removes all the abandoned components.
    ///
    /// Additionally, `LazyUpdate` will be merged.
    pub fn maintain(&mut self) {
        let deleted = self.entities_mut().alloc.merge();
        self.delete_components(&deleted);

        self.write_resource::<LazyUpdate>().maintain(&*self);
    }

    fn delete_components(&mut self, delete: &[Entity]) {
        for storage in &mut self.storages {
            let storage: &mut AnyStorage = unsafe { &mut **storage };

            for entity in delete {
                storage.remove(entity.id());
            }
        }
    }

    /// Adds the given bundle of resources/components.
    pub fn add_bundle<B>(&mut self, bundle: B)
    where
        B: Bundle,
    {
        bundle.add_to_world(self);
    }
}

unsafe impl Send for World {}

unsafe impl Sync for World {}

impl Borrow<Resources> for World {
    fn borrow(&self) -> &Resources {
        &self.res
    }
}

impl Component for World {
    type Storage = DenseVecStorage<Self>;
}

impl Default for World {
    fn default() -> Self {
        let mut res = Resources::new();
        res.add(EntitiesRes::default());
        res.add(LazyUpdate::default());

        World {
            res,
            storages: Default::default(),
        }
    }
}

/// Trait used to bundle up resources/components for easy registration with `World`.
pub trait Bundle {
    /// Add resources/components to `world`.
    fn add_to_world(self, world: &mut World);
}
