use super::{
    comp::Component,
    entity::{Allocator, EntitiesRes, Entity},
    CreateIter, EntityBuilder, LazyUpdate,
};

use crate::{
    error::WrongGeneration,
    storage::{AnyStorage, MaskedStorage},
    ReadStorage, WriteStorage,
};
use shred::{Fetch, FetchMut, MetaTable, Read, Resource, SystemData, World};

/// This trait provides some extension methods to make working with
/// `shred::World` easier.
///
/// Many methods take `&self` which works because everything
/// is stored with **interior mutability**. In case you violate
/// the borrowing rules of Rust (multiple reads xor one write),
/// you will get a panic.
///
/// ## Examples
///
/// ```
/// use specs::prelude::*;
/// # #[derive(Debug, PartialEq)]
/// # struct Pos { x: f32, y: f32, } impl Component for Pos { type Storage = VecStorage<Self>; }
/// # #[derive(Debug, PartialEq)]
/// # struct Vel { x: f32, y: f32, } impl Component for Vel { type Storage = VecStorage<Self>; }
/// # struct DeltaTime(f32);
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
/// let b = world
///     .create_entity()
///     .with(Pos { x: 3.0, y: 5.0 })
///     .with(Vel { x: 1.0, y: 0.0 })
///     .build();
///
/// let c = world
///     .create_entity()
///     .with(Pos { x: 0.0, y: 1.0 })
///     .with(Vel { x: 0.0, y: 1.0 })
///     .build();
///
/// {
///     // `World::read_storage` returns a component storage.
///     let pos_storage = world.read_storage::<Pos>();
///     let vel_storage = world.read_storage::<Vel>();
///
///     // `Storage::get` allows to get a component from it:
///     assert_eq!(pos_storage.get(b), Some(&Pos { x: 3.0, y: 5.0 }));
///     assert_eq!(vel_storage.get(c), Some(&Vel { x: 0.0, y: 1.0 }));
/// }
///
/// let empty = world.create_entity().build();
///
/// {
///     // This time, we write to the `Pos` storage:
///     let mut pos_storage = world.write_storage::<Pos>();
///     let vel_storage = world.read_storage::<Vel>();
///
///     assert!(pos_storage.get(empty).is_none());
///
///     // You can also insert components after creating the entity:
///     pos_storage.insert(empty, Pos { x: 3.1, y: 4.15 });
///
///     assert!(pos_storage.get(empty).is_some());
/// }
/// ```
pub trait WorldExt {
    /// Constructs a new World instance.
    fn new() -> Self;

    /// Registers a new component, adding the component storage.
    ///
    /// Calls `register_with_storage` with `Default::default()`.
    ///
    /// Does nothing if the component was already
    /// registered.
    ///
    /// ## Examples
    ///
    /// ```
    /// use specs::prelude::*;
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
    fn register<T: Component>(&mut self)
    where
        T::Storage: Default;

    /// Registers a new component with a given storage.
    ///
    /// Does nothing if the component was already registered.
    fn register_with_storage<F, T>(&mut self, storage: F)
    where
        F: FnOnce() -> T::Storage,
        T: Component;

    /// Gets `SystemData` `T` from the `World`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use specs::prelude::*;
    /// # struct Pos; struct Vel;
    /// # impl Component for Pos { type Storage = VecStorage<Self>; }
    /// # impl Component for Vel { type Storage = VecStorage<Self>; }
    ///
    /// let mut world = World::new();
    /// world.register::<Pos>();
    /// world.register::<Vel>();
    /// let storages: (WriteStorage<Pos>, ReadStorage<Vel>) = world.system_data();
    /// ```
    ///
    /// # Panics
    ///
    /// * Panics if `T` is already borrowed in an incompatible way.
    fn system_data<'a, T>(&'a self) -> T
    where
        T: SystemData<'a>;

    /// Sets up system data `T` for fetching afterwards.
    fn setup<'a, T: SystemData<'a>>(&mut self);

    /// Executes `f` once, right now with the specified system data.
    ///
    /// This sets up the system data `f` expects, fetches it and then
    /// executes `f`. You can see this like a system that only runs once.
    ///
    /// This is especially useful if you either need a lot of system data or
    /// you want to build an entity and for that you need to access resources
    /// first
    /// - just fetching the resources and building the entity would cause a
    ///   double borrow.
    ///
    /// **Calling this method is equivalent to:**
    ///
    /// ```
    /// # use specs::prelude::*; use specs::shred::ResourceId;
    /// # struct MySystemData; impl MySystemData { fn do_something(&self) {} }
    /// # impl<'a> SystemData<'a> for MySystemData {
    /// #     fn fetch(res: &World) -> Self { MySystemData }
    /// #     fn reads() -> Vec<ResourceId> { vec![] }
    /// #     fn writes() -> Vec<ResourceId> { vec![] }
    /// #     fn setup(res: &mut World) {}
    /// # }
    /// # let mut world = World::new();
    /// {
    ///     // note the extra scope
    ///     world.setup::<MySystemData>();
    ///     let my_data: MySystemData = world.system_data();
    ///     my_data.do_something();
    /// }
    /// ```
    ///
    /// ## Examples
    ///
    /// ```
    /// # use specs::prelude::*;
    /// let mut world = World::new();
    ///
    /// struct MyComp;
    ///
    /// impl Component for MyComp {
    ///     type Storage = DenseVecStorage<Self>;
    /// }
    ///
    /// #[derive(Default)]
    /// struct MyRes {
    ///     field: i32,
    /// }
    ///
    /// world.exec(|(mut my_res,): (Write<MyRes>,)| {
    ///     assert_eq!(my_res.field, 0);
    ///     my_res.field = 5;
    /// });
    ///
    /// assert_eq!(world.read_resource::<MyRes>().field, 5);
    /// ```
    fn exec<'a, F, R, T>(&'a mut self, f: F) -> R
    where
        F: FnOnce(T) -> R,
        T: SystemData<'a>;

    /// Adds a resource to the world.
    ///
    /// If the resource already exists it will be overwritten.
    ///
    /// ## Difference between resources and components
    ///
    /// While components exist per entity, resources are like globals in the
    /// `World`. Components are stored in component storages, which are
    /// resources themselves.
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
    /// to execute code at the end of the frame, which is done in
    /// `World::maintain`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use specs::prelude::*;
    ///
    /// # let timer = ();
    /// # let server_con = ();
    /// let mut world = World::new();
    /// world.add_resource(timer);
    /// world.add_resource(server_con);
    /// ```
    fn add_resource<T: Resource>(&mut self, res: T);

    /// Fetches a component storage for reading.
    ///
    /// ## Panics
    ///
    /// Panics if it is already borrowed mutably.
    /// Panics if the component has not been registered.
    fn read_component<T: Component>(&self) -> ReadStorage<T>;

    /// Fetches a component storage for writing.
    ///
    /// # Panics
    ///
    /// Panics if it is already borrowed.
    /// Panics if the component has not been registered.
    fn write_component<T: Component>(&self) -> WriteStorage<T>;

    /// Fetches a component storage for reading.
    ///
    /// ## Panics
    ///
    /// Panics if it is already borrowed mutably.
    /// Panics if the component has not been registered.
    fn read_storage<T: Component>(&self) -> ReadStorage<T> {
        self.read_component()
    }

    /// Fetches a component storage for writing.
    ///
    /// # Panics
    ///
    /// Panics if it is already borrowed.
    /// Panics if the component has not been registered.
    fn write_storage<T: Component>(&self) -> WriteStorage<T> {
        self.write_component()
    }

    /// Fetches a resource for reading.
    ///
    /// ## Panics
    ///
    /// Panics if it is already borrowed mutably.
    /// Panics if the resource has not been added.
    fn read_resource<T: Resource>(&self) -> Fetch<T>;

    /// Fetches a resource for writing.
    ///
    /// # Panics
    ///
    /// Panics if it is already borrowed.
    /// Panics if the resource has not been added.
    fn write_resource<T: Resource>(&self) -> FetchMut<T>;

    /// Convenience method for fetching entities.
    ///
    /// Creation and deletion of entities with the `Entities` struct
    /// are atomically, so the actual changes will be applied
    /// with the next call to `maintain()`.
    fn entities(&self) -> Read<EntitiesRes>;

    /// Convenience method for fetching entities.
    fn entities_mut(&self) -> FetchMut<EntitiesRes>;

    /// Allows building an entity with its components.
    ///
    /// This takes a mutable reference to the `World`, since no
    /// component storage this builder accesses may be borrowed.
    /// If it's necessary that you borrow a resource from the `World`
    /// while this builder is alive, you can use `create_entity_unchecked`.
    fn create_entity(&mut self) -> EntityBuilder;

    /// Allows building an entity with its components.
    ///
    /// **You have to make sure that no component storage is borrowed
    /// during the building!**
    ///
    /// This variant is only recommended if you need to borrow a resource
    /// during the entity building. If possible, try to use `create_entity`.
    fn create_entity_unchecked(&self) -> EntityBuilder;

    /// Returns an iterator for entity creation.
    /// This makes it easy to create a whole collection
    /// of them.
    ///
    /// ## Examples
    ///
    /// ```
    /// use specs::prelude::*;
    ///
    /// let mut world = World::new();
    /// let five_entities: Vec<_> = world.create_iter().take(5).collect();
    /// #
    /// # assert_eq!(five_entities.len(), 5);
    /// ```
    fn create_iter(&mut self) -> CreateIter;

    /// Deletes an entity and its components.
    fn delete_entity(&mut self, entity: Entity) -> Result<(), WrongGeneration>;

    /// Deletes the specified entities and their components.
    fn delete_entities(&mut self, delete: &[Entity]) -> Result<(), WrongGeneration>;

    /// Deletes all entities and their components.
    fn delete_all(&mut self);

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
    fn is_alive(&self, e: Entity) -> bool;

    /// Merges in the appendix, recording all the dynamically created
    /// and deleted entities into the persistent generations vector.
    /// Also removes all the abandoned components.
    ///
    /// Additionally, `LazyUpdate` will be merged.
    fn maintain(&mut self);

    #[doc(hidden)]
    fn delete_components(&mut self, delete: &[Entity]);
}

impl WorldExt for World {
    fn new() -> Self {
        let mut world = World::default();
        world.add_resource(EntitiesRes::default());
        world.add_resource(MetaTable::<AnyStorage>::default());
        world.add_resource(LazyUpdate::default());

        world
    }

    /// Registers a new component, adding the component storage.
    ///
    /// Calls `register_with_storage` with `Default::default()`.
    ///
    /// Does nothing if the component was already
    /// registered.
    ///
    /// ## Examples
    ///
    /// ```
    /// use specs::prelude::*;
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
    fn register<T: Component>(&mut self)
    where
        T::Storage: Default,
    {
        self.register_with_storage::<_, T>(Default::default);
    }

    /// Registers a new component with a given storage.
    ///
    /// Does nothing if the component was already registered.
    fn register_with_storage<F, T>(&mut self, storage: F)
    where
        F: FnOnce() -> T::Storage,
        T: Component,
    {
        self.entry()
            .or_insert_with(move || MaskedStorage::<T>::new(storage()));
        self.entry::<MetaTable<AnyStorage>>()
            .or_insert_with(Default::default);
        self.fetch_mut::<MetaTable<AnyStorage>>()
            .register(&*self.fetch::<MaskedStorage<T>>());
    }

    /// Gets `SystemData` `T` from the `World`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use specs::prelude::*;
    /// # struct Pos; struct Vel;
    /// # impl Component for Pos { type Storage = VecStorage<Self>; }
    /// # impl Component for Vel { type Storage = VecStorage<Self>; }
    ///
    /// let mut world = World::new();
    /// world.register::<Pos>();
    /// world.register::<Vel>();
    /// let storages: (WriteStorage<Pos>, ReadStorage<Vel>) = world.system_data();
    /// ```
    ///
    /// # Panics
    ///
    /// * Panics if `T` is already borrowed in an incompatible way.
    fn system_data<'a, T>(&'a self) -> T
    where
        T: SystemData<'a>,
    {
        SystemData::fetch(&self)
    }

    /// Sets up system data `T` for fetching afterwards.
    fn setup<'a, T: SystemData<'a>>(&mut self) {
        T::setup(self);
    }

    /// Executes `f` once, right now with the specified system data.
    ///
    /// This sets up the system data `f` expects, fetches it and then
    /// executes `f`. You can see this like a system that only runs once.
    ///
    /// This is especially useful if you either need a lot of system data or
    /// you want to build an entity and for that you need to access resources
    /// first
    /// - just fetching the resources and building the entity would cause a
    ///   double borrow.
    ///
    /// **Calling this method is equivalent to:**
    ///
    /// ```
    /// # use specs::prelude::*; use specs::shred::ResourceId;
    /// # struct MySystemData; impl MySystemData { fn do_something(&self) {} }
    /// # impl<'a> SystemData<'a> for MySystemData {
    /// #     fn fetch(res: &World) -> Self { MySystemData }
    /// #     fn reads() -> Vec<ResourceId> { vec![] }
    /// #     fn writes() -> Vec<ResourceId> { vec![] }
    /// #     fn setup(res: &mut World) {}
    /// # }
    /// # let mut world = World::new();
    /// {
    ///     // note the extra scope
    ///     world.setup::<MySystemData>();
    ///     let my_data: MySystemData = world.system_data();
    ///     my_data.do_something();
    /// }
    /// ```
    ///
    /// ## Examples
    ///
    /// ```
    /// # use specs::prelude::*;
    /// let mut world = World::new();
    ///
    /// struct MyComp;
    ///
    /// impl Component for MyComp {
    ///     type Storage = DenseVecStorage<Self>;
    /// }
    ///
    /// #[derive(Default)]
    /// struct MyRes {
    ///     field: i32,
    /// }
    ///
    /// world.exec(|(mut my_res,): (Write<MyRes>,)| {
    ///     assert_eq!(my_res.field, 0);
    ///     my_res.field = 5;
    /// });
    ///
    /// assert_eq!(world.read_resource::<MyRes>().field, 5);
    /// ```
    fn exec<'a, F, R, T>(&'a mut self, f: F) -> R
    where
        F: FnOnce(T) -> R,
        T: SystemData<'a>,
    {
        self.setup::<T>();
        f(self.system_data())
    }

    /// Adds a resource to the world.
    ///
    /// If the resource already exists it will be overwritten.
    ///
    /// ## Difference between resources and components
    ///
    /// While components exist per entity, resources are like globals in the
    /// `World`. Components are stored in component storages, which are
    /// resources themselves.
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
    /// to execute code at the end of the frame, which is done in
    /// `World::maintain`.
    ///
    /// ## Examples
    ///
    /// ```
    /// use specs::prelude::*;
    ///
    /// # let timer = ();
    /// # let server_con = ();
    /// let mut world = World::new();
    /// world.add_resource(timer);
    /// world.add_resource(server_con);
    /// ```
    fn add_resource<T: Resource>(&mut self, res: T) {
        if self.has_value::<T>() {
            *self.fetch_mut() = res;
        } else {
            self.insert(res);
        }
    }

    fn read_component<T: Component>(&self) -> ReadStorage<T> {
        self.system_data()
    }

    fn write_component<T: Component>(&self) -> WriteStorage<T> {
        self.system_data()
    }

    /// Fetches a resource for reading.
    ///
    /// ## Panics
    ///
    /// Panics if it is already borrowed mutably.
    /// Panics if the resource has not been added.
    fn read_resource<T: Resource>(&self) -> Fetch<T> {
        self.fetch()
    }

    /// Fetches a resource for writing.
    ///
    /// # Panics
    ///
    /// Panics if it is already borrowed.
    /// Panics if the resource has not been added.
    fn write_resource<T: Resource>(&self) -> FetchMut<T> {
        self.fetch_mut()
    }

    /// Convenience method for fetching entities.
    ///
    /// Creation and deletion of entities with the `Entities` struct
    /// are atomically, so the actual changes will be applied
    /// with the next call to `maintain()`.
    fn entities(&self) -> Read<EntitiesRes> {
        Read::fetch(&self)
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
    fn create_entity(&mut self) -> EntityBuilder {
        self.create_entity_unchecked()
    }

    /// Allows building an entity with its components.
    ///
    /// **You have to make sure that no component storage is borrowed
    /// during the building!**
    ///
    /// This variant is only recommended if you need to borrow a resource
    /// during the entity building. If possible, try to use `create_entity`.
    fn create_entity_unchecked(&self) -> EntityBuilder {
        let entity = self.entities_mut().alloc.allocate();

        EntityBuilder {
            entity,
            world: self,
            built: false,
        }
    }

    /// Returns an iterator for entity creation.
    /// This makes it easy to create a whole collection
    /// of them.
    ///
    /// ## Examples
    ///
    /// ```
    /// use specs::prelude::*;
    ///
    /// let mut world = World::new();
    /// let five_entities: Vec<_> = world.create_iter().take(5).collect();
    /// #
    /// # assert_eq!(five_entities.len(), 5);
    /// ```
    fn create_iter(&mut self) -> CreateIter {
        CreateIter(self.entities_mut())
    }

    /// Deletes an entity and its components.
    fn delete_entity(&mut self, entity: Entity) -> Result<(), WrongGeneration> {
        self.delete_entities(&[entity])
    }

    /// Deletes the specified entities and their components.
    fn delete_entities(&mut self, delete: &[Entity]) -> Result<(), WrongGeneration> {
        self.delete_components(delete);

        self.entities_mut().alloc.kill(delete)
    }

    /// Deletes all entities and their components.
    fn delete_all(&mut self) {
        use crate::join::Join;

        let entities: Vec<_> = self.entities().join().collect();

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
    fn is_alive(&self, e: Entity) -> bool {
        assert!(e.gen().is_alive(), "Generation is dead");

        let alloc: &Allocator = &self.entities().alloc;
        alloc.generation(e.id()) == Some(e.gen())
    }

    /// Merges in the appendix, recording all the dynamically created
    /// and deleted entities into the persistent generations vector.
    /// Also removes all the abandoned components.
    ///
    /// Additionally, `LazyUpdate` will be merged.
    fn maintain(&mut self) {
        let deleted = self.entities_mut().alloc.merge();
        if !deleted.is_empty() {
            self.delete_components(&deleted);
        }

        // we need to swap the queue out to be able to reborrow self mutable here
        let mut lazy = self.write_resource::<LazyUpdate>().take();
        lazy.maintain(&mut *self);
        self.write_resource::<LazyUpdate>().restore(lazy);
    }

    fn delete_components(&mut self, delete: &[Entity]) {
        self.entry::<MetaTable<AnyStorage>>()
            .or_insert_with(Default::default);
        for storage in self.fetch_mut::<MetaTable<AnyStorage>>().iter_mut(&self) {
            storage.drop(delete);
        }
    }
}
