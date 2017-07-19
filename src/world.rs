use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam::sync::TreiberStack;
#[cfg(feature="serialize")]
use std::marker::PhantomData;
#[cfg(feature="serialize")]
use serde::de::DeserializeSeed;
#[cfg(feature="serialize")]
use serde::{Serialize, Serializer, Deserializer};
#[cfg(feature="serialize")]
use group::SerializeGroup;

use hibitset::{AtomicBitSet, BitSet, BitSetOr};
use mopa::Any;
use shred::{Fetch, FetchMut, Resource, Resources};

use storage::{AnyStorage, MaskedStorage};
use {ComponentGroup, Index, Join, ParJoin, ReadStorage, Storage, UnprotectedStorage, WriteStorage};

const COMPONENT_NOT_REGISTERED: &str = "No component with the given id. Did you forget to register \
the component with `World::register::<ComponentName>()`?";
const RESOURCE_NOT_ADDED: &str = "No resource with the given id. Did you forget to add \
the resource with `World::add_resource(resource)`?";

/// Internally used structure for `Entity` allocation.
#[derive(Default, Debug)]
pub struct Allocator {
    generations: Vec<Generation>,

    alive: BitSet,
    raised: AtomicBitSet,
    killed: AtomicBitSet,
    start_from: AtomicUsize,
}

impl Allocator {
    fn kill(&mut self, delete: &[Entity]) {
        for entity in delete {
            self.alive.remove(entity.id());
            self.raised.remove(entity.id());
            let id = entity.id() as usize;
            self.generations[id].die();
            if id < self.start_from.load(Ordering::Relaxed) {
                self.start_from.store(id, Ordering::Relaxed);
            }
        }
    }

    fn kill_atomic(&self, e: Entity) {
        self.killed.add_atomic(e.id());
    }

    /// Return `true` if the entity is alive.
    fn is_alive(&self, e: Entity) -> bool {
        e.gen() ==
        match self.generations.get(e.id() as usize) {
            Some(g) if !g.is_alive() && self.raised.contains(e.id()) => g.raised(),
            Some(g) => *g,
            None => Generation(1),
        }
    }

    /// Attempt to move the `start_from` value
    fn update_start_from(&self, start_from: usize) {
        loop {
            let current = self.start_from.load(Ordering::Relaxed);

            // if the current value is bigger then ours, we bail
            if current >= start_from {
                return;
            }

            if start_from ==
               self.start_from
                   .compare_and_swap(current, start_from, Ordering::Relaxed) {
                return;
            }
        }
    }

    /// Allocate a new entity
    fn allocate_atomic(&self) -> Entity {
        let idx = self.start_from.load(Ordering::Relaxed);
        for i in idx.. {
            if !self.alive.contains(i as Index) && !self.raised.add_atomic(i as Index) {
                self.update_start_from(i + 1);

                let gen = self.generations
                    .get(i as usize)
                    .map(|&gen| if gen.is_alive() { gen } else { gen.raised() })
                    .unwrap_or(Generation(1));

                return Entity(i as Index, gen);
            }
        }
        panic!("No entities left to allocate")
    }

    /// Allocate a new entity
    fn allocate(&mut self) -> Entity {
        let idx = self.start_from.load(Ordering::Relaxed);
        for i in idx.. {
            if !self.raised.contains(i as Index) && !self.alive.add(i as Index) {
                // this is safe since we have mutable access to everything!
                self.start_from.store(i + 1, Ordering::Relaxed);

                while self.generations.len() <= i as usize {
                    self.generations.push(Generation(0));
                }
                self.generations[i as usize] = self.generations[i as usize].raised();

                return Entity(i as Index, self.generations[i as usize]);
            }
        }
        panic!("No entities left to allocate")
    }

    fn merge(&mut self) -> Vec<Entity> {
        use hibitset::BitSetLike;

        let mut deleted = vec![];

        for i in (&self.raised).iter() {
            while self.generations.len() <= i as usize {
                self.generations.push(Generation(0));
            }
            self.generations[i as usize] = self.generations[i as usize].raised();
            self.alive.add(i);
        }
        self.raised.clear();

        if let Some(lowest) = (&self.killed).iter().next() {
            if lowest < self.start_from.load(Ordering::Relaxed) as Index {
                self.start_from.store(lowest as usize, Ordering::Relaxed);
            }
        }

        for i in (&self.killed).iter() {
            self.alive.remove(i);
            self.generations[i as usize].die();
            deleted.push(Entity(i, self.generations[i as usize]))
        }
        self.killed.clear();

        deleted
    }
}

/// Abstract component type. Doesn't have to be Copy or even Clone.
pub trait Component: Any + Sized {
    /// Associated storage type for this component.
    type Storage: UnprotectedStorage<Self> + Any + Send + Sync;
}

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

/// An iterator for entity creation.
/// Please note that you have to consume
/// it because iterators are lazy.
///
/// Returned from `Entities::create_iter`.
pub struct CreateIterAtomic<'a>(&'a Allocator);

impl<'a> Iterator for CreateIterAtomic<'a> {
    type Item = Entity;

    fn next(&mut self) -> Option<Entity> {
        Some(self.0.allocate_atomic())
    }
}

/// Any type that contains an entity's index.
///
/// e.g. Entry, Entity, etc.
pub trait EntityIndex {
    fn index(&self) -> Index;
}

impl EntityIndex for Entity {
    fn index(&self) -> Index {
        self.id()
    }
}

impl<'a> EntityIndex for &'a Entity {
    fn index(&self) -> Index {
        (*self).index()
    }
}

/// `Entity` type, as seen by the user.
#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Entity(Index, Generation);

impl Entity {
    /// Creates a new entity (externally from ECS).
    #[cfg(test)]
    pub fn new(index: Index, gen: Generation) -> Entity {
        Entity(index, gen)
    }

    /// Returns the index of the `Entity`.
    #[inline]
    pub fn id(&self) -> Index {
        self.0
    }

    /// Returns the `Generation` of the `Entity`.
    #[inline]
    pub fn gen(&self) -> Generation {
        self.1
    }
}

/// The entity builder, allowing to
/// build an entity together with its components.
pub struct EntityBuilder<'a> {
    entity: Entity,
    world: &'a mut World,
}

impl<'a> EntityBuilder<'a> {
    /// Appends a component with the default component id.
    ///
    /// # Panics
    ///
    /// Panics if the component hasn't been `register()`ed in the
    /// `World`.
    pub fn with<T: Component>(self, c: T) -> Self {
        self.with_id(c, 0)
    }

    /// Appends a component with a component id.
    ///
    /// # Panics
    ///
    /// Panics if the component hasn't been `register_with_id()`ed in the
    /// `World`.
    pub fn with_id<T: Component>(self, c: T, id: usize) -> Self {
        {
            let mut storage = self.world.write_with_id(id);
            storage.insert(self.entity, c);
        }

        self
    }

    /// Finishes the building and returns
    /// the entity.
    pub fn build(self) -> Entity {
        self.entity
    }
}

/// The entities of this ECS. This is a resource, stored in the `World`.
/// If you just want to access it in your system, you can also use the `Entities`
/// type def.
///
/// **Please note that you should never fetch
/// this mutably in a system, because it would
/// block all the other systems.**
///
/// You need to call `World::maintain` after creating / deleting
/// entities with this struct.
#[derive(Debug, Default)]
pub struct EntitiesRes {
    alloc: Allocator,
}

impl EntitiesRes {
    /// Creates a new entity atomically.
    /// This will be persistent as soon
    /// as you call `World::maintain`.
    pub fn create(&self) -> Entity {
        self.alloc.allocate_atomic()
    }

    /// Returns an iterator which creates
    /// new entities atomically.
    /// They will be persistent as soon
    /// as you call `World::maintain`.
    pub fn create_iter(&self) -> CreateIterAtomic {
        CreateIterAtomic(&self.alloc)
    }

    /// Deletes an entity atomically.
    /// The associated components will be
    /// deleted as soon as you call `World::maintain`.
    pub fn delete(&self, e: Entity) {
        self.alloc.kill_atomic(e);
    }

    /// Returns `true` if the specified entity is
    /// alive.
    #[inline]
    pub fn is_alive(&self, e: Entity) -> bool {
        self.alloc.is_alive(e)
    }
}

impl<'a> Join for &'a EntitiesRes {
    type Type = Entity;
    type Value = Self;
    type Mask = BitSetOr<&'a BitSet, &'a AtomicBitSet>;

    fn open(self) -> (Self::Mask, Self) {
        (BitSetOr(&self.alloc.alive, &self.alloc.raised), self)
    }

    unsafe fn get(v: &mut &'a EntitiesRes, idx: Index) -> Entity {
        let gen = v.alloc
            .generations
            .get(idx as usize)
            .map(|&gen| if gen.is_alive() { gen } else { gen.raised() })
            .unwrap_or(Generation(1));
        Entity(idx, gen)
    }
}

unsafe impl<'a> ParJoin for &'a EntitiesRes {}

/// Index generation. When a new entity is placed at an old index,
/// it bumps the `Generation` by 1. This allows to avoid using components
/// from the entities that were deleted.
#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Generation(i32);

impl Generation {
    #[cfg(test)]
    pub fn new(v: i32) -> Self {
        Generation(v)
    }

    /// Returns the id of the generation.
    #[inline]
    pub fn id(&self) -> i32 {
        self.0
    }

    /// Returns `true` if entities of this `Generation` are alive.
    pub fn is_alive(&self) -> bool {
        self.0 > 0
    }

    /// Kills this `Generation`.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if it's not alive.
    fn die(&mut self) {
        debug_assert!(self.is_alive());
        self.0 = -self.0;
    }

    /// Revives and increments a dead `Generation`.
    ///
    /// # Panics
    ///
    /// Panics in debug mode if it is alive.
    fn raised(self) -> Generation {
        debug_assert!(!self.is_alive());
        Generation(1 - self.0)
    }
}

/// A type implementing `LazyInsert` can be inserted
/// using `LazyInsertions`.
pub trait LazyInsert: Send + Sync {
    /// Inserts the component(s) into the world.
    fn insert(self, world: &World);
}

impl<C> LazyInsert for (Entity, C) where C: Component + Send + Sync {
    fn insert(self, world: &World) {
        world.write::<C>().insert(self.0, self.1);
    }
}

impl<L> LazyInsert for Vec<L>
    where L: LazyInsert
{
    fn insert(self, world: &World) {
        for item in self {
            item.insert(world);
        }
    }
}

trait LazyInsertInternal: Send + Sync {
    fn insert(self: Box<Self>, world: &World);
}

impl<L> LazyInsertInternal for L
    where L: LazyInsert
{
    fn insert(self: Box<Self>, world: &World) {
        L::insert(*self, world);
    }
}

/// Lazy insertions can be used after creating a new
/// entity in a system. This way, none of the actual
/// component storages have to be borrowed mutably.
///
/// This resource is added to the world by default.
pub struct LazyInsertions {
    stack: TreiberStack<Box<LazyInsertInternal>>
}

impl LazyInsertions {
    /// Adds an insertion. Please note that this method takes `&self`
    /// so there's no need to fetch it mutably.
    ///
    /// ## Examples
    ///
    /// ```
    /// # use specs::*;
    /// #
    /// struct Pos(f32, f32);
    ///
    /// impl Component for Pos {
    ///     type Storage = VecStorage<Self>;
    /// }
    ///
    /// struct InsertPos;
    ///
    /// impl<'a> System<'a> for InsertPos {
    ///     type SystemData = (Entities<'a>, Fetch<'a, LazyInsertions>);
    ///
    ///     fn run(&mut self, (ent, lazy): Self::SystemData) {
    ///         let a = ent.create();
    ///         let b = ent.create();
    ///
    ///         lazy.add(vec![(a, Pos(3.0, 1.0)), (b, Pos(0.0, 4.0))]);
    ///     }
    /// }
    /// ```
    pub fn add<L>(&self, l: L)
        where L: LazyInsert + 'static
    {
        self.stack.push(Box::new(l));
    }
}

impl Default for LazyInsertions {
    fn default() -> Self {
        // TODO: derive (`Default` is not yet implemented for `TreiberStack`)
        LazyInsertions { stack: TreiberStack::new() }
    }
}

impl Drop for LazyInsertions {
    fn drop(&mut self) {
        // TODO: remove as soon as leak is fixed in crossbeam
        while self.stack.pop().is_some() {}
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
/// # Component / Resources ids
///
/// Components and resources may, in addition to their type, be identified
/// by an id of type `usize`. The convenience methods dealing
/// with components assume that it's `0`.
///
/// If a system attempts to access a component/resource that has not been
/// registered/added, it will panic when run. Add all components with
/// `World::register` before running any systems. Also add all resources
/// with `World::add_resource`.
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
               .has_value(ResourceId::new_with_id::<MaskedStorage<T>>(id)) {
            return;
        }

        self.res.add_with_id(MaskedStorage::<T>::new(), id);

        let mut storage = self.res.fetch_mut::<MaskedStorage<T>>(id);
        self.storages.push(&mut *storage as *mut AnyStorage);
    }

    /// Registers a `ComponentGroup` into the world.
    pub fn register_group<G: ComponentGroup>(&mut self) {
        G::register(self);
    }

    /// Adds a resource with the default ID (`0`).
    ///
    /// If the resource already exists it will be overwritten.
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
    /// # Panics
    ///
    /// Panics if it is already borrowed mutably.
    pub fn read<T: Component>(&self) -> ReadStorage<T> {
        self.read_with_id(0)
    }

    /// Fetches a component's storage with the default id for writing.
    ///
    /// Convenience method for `write_with_id`, using the default component
    /// id (`0`).
    ///
    /// # Panics
    ///
    /// Panics if it is already borrowed.
    pub fn write<T: Component>(&self) -> WriteStorage<T> {
        self.write_with_id(0)
    }

    /// Fetches a component's storage with a specified id for reading.
    ///
    /// # Panics
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
    /// # Panics
    ///
    /// Panics if it is already borrowed mutably.
    pub fn read_resource_with_id<T: Resource>(&self, id: usize) -> Fetch<T> {
        self.res.try_fetch(id).expect(RESOURCE_NOT_ADDED)
    }

    /// Fetches a resource with a specified id for writing.
    ///
    /// # Panics
    ///
    /// Panics if it is already borrowed.
    pub fn write_resource_with_id<T: Resource>(&self, id: usize) -> FetchMut<T> {
        self.res.try_fetch_mut(id).expect(RESOURCE_NOT_ADDED)
    }

    /// Fetches a resource with the default id for reading.
    ///
    /// Convenience method for `read_resource_with_id`, using the default component
    /// id (`0`).
    ///
    /// # Panics
    ///
    /// Panics if it is already borrowed mutably.
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

    /// Allows building an entity with its
    /// components.
    pub fn create_entity(&mut self) -> EntityBuilder {
        let entity = self.entities_mut().alloc.allocate();

        EntityBuilder {
            entity: entity,
            world: self,
        }
    }

    /// Returns an iterator for entity creation.
    /// This makes it easy to create a whole collection
    /// of them.
    ///
    /// # Examples
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
    pub fn delete_entity(&mut self, entity: Entity) {
        self.delete_entities(&[entity]);
    }

    /// Deletes the specified entities and their components.
    pub fn delete_entities(&mut self, delete: &[Entity]) {
        self.delete_components(delete);
        self.entities_mut().alloc.kill(delete);
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
    /// Additionally, `LazyInsertions` will be merged.
    pub fn maintain(&mut self) {
        let deleted = self.entities_mut().alloc.merge();
        self.delete_components(&deleted);

        let mut lazy_insertions = self.write_resource::<LazyInsertions>();
        let lazy = &mut lazy_insertions.stack;

        while let Some(l) = lazy.pop() {
            l.insert(&*self);
        }
    }

    fn delete_components(&mut self, delete: &[Entity]) {
        for storage in &mut self.storages {
            let storage: &mut AnyStorage = unsafe { &mut **storage };

            for entity in delete {
                storage.remove(entity.id());
            }
        }
    }
}

impl Default for World {
    fn default() -> Self {
        let mut res = Resources::new();
        res.add(EntitiesRes::default());
        res.add(LazyInsertions::default());

        World {
            res: res,
            storages: Default::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use storage::VecStorage;
    use super::*;

    struct Pos;

    impl Component for Pos {
        type Storage = VecStorage<Self>;
    }

    #[test]
    fn lazy_insertion() {
        let mut world = World::new();
        world.register::<Pos>();

        let e;
        {
            let entities = world.read_resource::<EntitiesRes>();
            let lazy = world.read_resource::<LazyInsertions>();

            e = entities.create();
            lazy.add((e, Pos));
        }

        world.maintain();
        assert!(world.read::<Pos>().get(e).is_some());
    }
}

#[cfg(feature="serialize")]
/// Structure used to serialize a world using a component group.
pub struct WorldSerializer<'a, G> {
    world: &'a World,
    phantom: PhantomData<G>,
}

#[cfg(feature="serialize")]
impl<'a, G> WorldSerializer<'a, G> {
    /// Creates a new world serializer out of a world.
    pub fn new(world: &'a World) -> WorldSerializer<'a, G> {
        WorldSerializer {
            world: world,
            phantom: PhantomData,
        }
    }
}

#[cfg(feature="serialize")]
impl<'a, G> Serialize for WorldSerializer<'a, G>
    where G: ComponentGroup + SerializeGroup
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer 
    {
        G::serialize_group(self.world, serializer)
    }
}

#[cfg(feature="serialize")]
/// Structure used for stateful deserialization into the world using a component group.
pub struct WorldDeserializer<'a, G> {
    world: &'a mut World,
    entities: &'a [Entity],
    phantom: PhantomData<G>,
}

#[cfg(feature="serialize")]
impl<'a, G> WorldDeserializer<'a, G> {
    /// Creates a new world deserializer out of a world and a list of entities.
    ///
    /// The list of entities will be used to merge into the component storages.
    pub fn new(world: &'a mut World, entities: &'a [Entity]) -> WorldDeserializer<'a, G> {
        WorldDeserializer {
            world: world,
            entities: entities,
            phantom: PhantomData,
        }
    }
}

impl<'a, 'de, G> DeserializeSeed<'de> for WorldDeserializer<'a, G>
    where G: ComponentGroup + SerializeGroup,
{
    type Value = ();
    fn deserialize<D>(self, deserializer: D) -> Result<(), D::Error>
        where D: Deserializer<'de>
    {
        G::deserialize_group(self.world, self.entities, deserializer)
    }
}

