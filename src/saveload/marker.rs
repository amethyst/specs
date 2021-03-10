//! Provides `Marker` and `MarkerAllocator` traits

use std::{
    collections::HashMap,
    fmt::{self, Debug},
    hash::{Hash, Hasher},
    marker::PhantomData,
};

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    prelude::*,
    world::{EntitiesRes, EntityResBuilder, LazyBuilder},
};

/// A common trait for `EntityBuilder` and `LazyBuilder` with a marker function,
/// allowing either to be used.
pub trait MarkedBuilder {
    /// Add a `Marker` to the entity by fetching the associated allocator.
    ///
    /// ## Examples
    ///
    /// ```
    /// use specs::{
    ///     prelude::*,
    ///     saveload::{MarkedBuilder, SimpleMarker, SimpleMarkerAllocator},
    /// };
    ///
    /// struct NetworkSync;
    ///
    /// fn mark_entity<M: Builder + MarkedBuilder>(markable: M) -> Entity {
    ///     markable
    ///    /* .with(Component1) */
    ///     .marked::<SimpleMarker<NetworkSync>>()
    ///     .build()
    /// }
    ///
    /// let mut world = World::new();
    /// world.register::<SimpleMarker<NetworkSync>>();
    /// world.insert(SimpleMarkerAllocator::<NetworkSync>::new());
    ///
    /// mark_entity(world.create_entity());
    /// ```
    fn marked<M: Marker>(self) -> Self;
}

impl<'a> MarkedBuilder for EntityBuilder<'a> {
    fn marked<M>(self) -> Self
    where
        M: Marker,
    {
        let mut alloc = self.world.write_resource::<M::Allocator>();
        alloc.mark(self.entity, &mut self.world.write_storage::<M>());

        self
    }
}

impl<'a> MarkedBuilder for LazyBuilder<'a> {
    /// Add a `Marker` to the entity by fetching the associated allocator.
    ///
    /// This will be applied on the next `world.maintain()`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use specs::{
    ///     prelude::*,
    ///     saveload::{MarkedBuilder, SimpleMarker, SimpleMarkerAllocator},
    /// };
    ///
    /// struct NetworkSync;
    ///
    /// let mut world = World::new();
    /// world.register::<SimpleMarker<NetworkSync>>();
    /// world.insert(SimpleMarkerAllocator::<NetworkSync>::new());
    ///
    /// # let lazy = world.read_resource::<LazyUpdate>();
    /// # let entities = world.entities();
    /// let my_entity = lazy
    ///     .create_entity(&entities)
    ///     /* .with(Component1) */
    ///     .marked::<SimpleMarker<NetworkSync>>()
    ///     .build();
    /// ```
    ///
    /// ## Panics
    ///
    /// Panics during `world.maintain()` in case there's no allocator
    /// added to the `World`.
    fn marked<M>(self) -> Self
    where
        M: Marker,
    {
        let entity = self.entity;
        self.lazy.exec(move |world| {
            let mut alloc = world.write_resource::<M::Allocator>();
            alloc.mark(entity, &mut world.write_storage::<M>());
        });

        self
    }
}

impl<'a> EntityResBuilder<'a> {
    /// Add a `Marker` to the entity with the associated allocator,
    /// and component storage.
    ///
    /// ## Examples
    ///
    /// ```
    /// use specs::{
    ///     prelude::*,
    ///     saveload::{SimpleMarker, SimpleMarkerAllocator},
    /// };
    ///
    /// struct NetworkSync;
    ///
    /// let mut world = World::new();
    /// world.register::<SimpleMarker<NetworkSync>>();
    /// world.insert(SimpleMarkerAllocator::<NetworkSync>::new());
    ///
    /// let mut storage = world.write_storage::<SimpleMarker<NetworkSync>>();
    /// let mut alloc = world.write_resource::<SimpleMarkerAllocator<NetworkSync>>();
    ///
    /// let entities = world.entities();
    /// entities
    ///     .build_entity()
    ///     /* .with(Component1) */
    ///     .marked(&mut storage, &mut alloc)
    ///     .build();
    /// ```
    pub fn marked<M>(self, storage: &mut WriteStorage<M>, alloc: &mut M::Allocator) -> Self
    where
        M: Marker,
    {
        alloc.mark(self.entity, storage);
        self
    }
}

/// This trait should be implemented by a component which is going to be used as
/// marker. This marker should be set to entity that should be serialized.
/// If serialization strategy needs to set marker to some entity
/// then it should use newly allocated marker from `Marker::Allocator`.
///
/// ## Example
///
/// ```rust
/// extern crate specs;
/// #[macro_use]
/// extern crate serde;
/// use std::{collections::HashMap, ops::Range};
///
/// use specs::{
///     prelude::*,
///     saveload::{MarkedBuilder, Marker, MarkerAllocator},
///     world::EntitiesRes,
/// };
///
/// // Marker for entities that should be synced over network
/// #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
/// struct NetMarker {
///     id: u64,
///     seq: u64,
/// }
///
/// impl Component for NetMarker {
///     type Storage = DenseVecStorage<Self>;
/// }
///
/// impl Marker for NetMarker {
///     type Allocator = NetNode;
///     type Identifier = u64;
///
///     fn id(&self) -> u64 {
///         self.id
///     }
///
///     // Updates sequence id.
///     // Entities with too old sequence id get deleted.
///     fn update(&mut self, update: Self) {
///         assert_eq!(self.id, update.id);
///         self.seq = update.seq;
///     }
/// }
///
/// // Each client and server has one
/// // Contains id range and `NetMarker -> Entity` mapping
/// struct NetNode {
///     range: Range<u64>,
///     mapping: HashMap<u64, Entity>,
/// }
///
/// impl MarkerAllocator<NetMarker> for NetNode {
///     fn allocate(&mut self, entity: Entity, id: Option<u64>) -> NetMarker {
///         let id = id.unwrap_or_else(|| {
///             self.range
///                 .next()
///                 .expect("Id range must be virtually endless")
///         });
///         let marker = NetMarker { id, seq: 0 };
///         self.mapping.insert(id, entity);
///         marker
///     }
///
///     fn retrieve_entity_internal(&self, id: u64) -> Option<Entity> {
///         self.mapping.get(&id).cloned()
///     }
///
///     fn maintain(&mut self, entities: &EntitiesRes, storage: &ReadStorage<NetMarker>) {
///         self.mapping = (entities, storage)
///             .join()
///             .map(|(e, m)| (m.id(), e))
///             .collect();
///     }
/// }
///
/// fn main() {
///     let mut world = World::new();
///     world.register::<NetMarker>();
///
///     world.insert(NetNode {
///         range: 0..100,
///         mapping: HashMap::new(),
///     });
///
///     let entity = world.create_entity().marked::<NetMarker>().build();
///     let storage = &mut world.write_storage::<NetMarker>();
///     let marker = storage.get(entity).unwrap().clone();
///     assert_eq!(
///         world
///             .write_resource::<NetNode>()
///             .retrieve_entity(marker, storage, &world.entities()),
///         entity
///     );
/// }
/// ```
pub trait Marker: Clone + Component + Debug + Eq + Hash + DeserializeOwned + Serialize {
    /// Id of the marker
    type Identifier;
    /// Allocator for this `Marker`
    type Allocator: MarkerAllocator<Self>;

    /// Get this marker internal id.
    /// The value of this method should be constant.
    fn id(&self) -> Self::Identifier;

    /// This gets called when an entity is deserialized by
    /// `DeserializeComponents`. It can be used to update internal data that
    /// is not used for identification.
    ///
    /// ## Contract
    ///
    /// This function may assume that `self.id() == new_revision.id()`.
    /// However, it must not exhibit undefined behavior in such a case.
    ///
    /// ## Panics
    ///
    /// May panic if `self.id()` != `new_revision.id()`.
    ///
    /// ## Default implementation
    ///
    /// The default implementation just sets `self` to `new_revision`.
    ///
    /// ## Examples
    ///
    /// ```rust,ignore
    /// #[derive(Clone, Debug, Deserialize, Eq, Hash, Serialize)]
    /// struct MyMarker {
    ///     id: u64,
    ///     last_modified: String,
    /// }
    ///
    /// impl Marker for MyMarker {
    ///     type Identifier = u64;
    ///
    ///     fn id(&self) -> u64 {
    ///         self.id
    ///     }
    ///
    ///     fn update(&self, new: Self) {
    ///         self.last_modified = new.last_modified;
    ///     }
    /// }
    /// ```
    ///
    /// Now, the marker always contains the name of the client who updated the
    /// entity associated with this marker.
    fn update(&mut self, new_revision: Self) {
        *self = new_revision;
    }
}

/// This allocator is used with the `Marker` trait.
/// It provides a method for allocating new `Marker`s.
/// It should also provide a `Marker -> Entity` mapping.
/// The `maintain` method can be implemented for cleanup and actualization.
/// See docs for `Marker` for an example.
pub trait MarkerAllocator<M: Marker>: Resource {
    /// Allocates a new marker for a given entity.
    /// If you don't pass an id, a new unique id will be created.
    fn allocate(&mut self, entity: Entity, id: Option<M::Identifier>) -> M;

    /// Get an `Entity` by a marker identifier.
    /// This function only accepts an id; it does not update the marker data.
    ///
    /// Implementors usually maintain a marker -> entity mapping
    /// and use that to retrieve the entity.
    fn retrieve_entity_internal(&self, id: M::Identifier) -> Option<Entity>;

    /// Tries to retrieve an entity by the id of the marker;
    /// if no entity has a marker with the same id, a new entity
    /// will be created and `marker` will be inserted for it.
    ///
    /// In case the entity existed,
    /// this method will update the marker data using `Marker::update`.
    fn retrieve_entity(
        &mut self,
        marker: M,
        storage: &mut WriteStorage<M>,
        entities: &EntitiesRes,
    ) -> Entity {
        if let Some(entity) = self.retrieve_entity_internal(marker.id()) {
            if let Some(mut marker_comp) = storage.get_mut(entity) {
                marker_comp.update(marker);

                return entity;
            }
        }

        let entity = entities.create();
        let marker = self.allocate(entity, Some(marker.id()));

        // It's not possible for this to fail, as there's no way a freshly
        // created entity could be dead this fast.
        storage.insert(entity, marker).unwrap();
        entity
    }

    /// Create new unique marker `M` and attach it to entity.
    /// Or get old marker if this entity is already marked.
    /// If entity is dead then this will return `None`.
    fn mark<'m>(
        &mut self,
        entity: Entity,
        storage: &'m mut WriteStorage<M>,
    ) -> Option<(&'m M, bool)> {
        let new = if let Ok(entry) = storage.entry(entity) {
            let mut new = false;
            let _marker = entry.or_insert_with(|| {
                new = true;
                self.allocate(entity, None)
            });

            new
        } else {
            return None
        };
        Some((storage.get(entity).unwrap(), new))
    }

    /// Maintain internal data. Cleanup if necessary.
    fn maintain(&mut self, _entities: &EntitiesRes, _storage: &ReadStorage<M>);
}

/// Basic marker implementation usable for saving and loading, uses `u64` as
/// identifier
#[derive(Serialize, Deserialize)]
#[repr(transparent)]
pub struct SimpleMarker<T: ?Sized>(u64, #[serde(skip)] PhantomData<T>);

impl<T: ?Sized> Clone for SimpleMarker<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for SimpleMarker<T> {}

impl<T: ?Sized> PartialEq for SimpleMarker<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T: ?Sized> Eq for SimpleMarker<T> {}

impl<T: ?Sized> Hash for SimpleMarker<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl<T: ?Sized> Debug for SimpleMarker<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_tuple("SimpleMarker")
            .field(&self.0)
            .field(&self.1)
            .finish()
    }
}

impl<T> Component for SimpleMarker<T>
where
    T: 'static + ?Sized + Send + Sync,
{
    type Storage = DenseVecStorage<Self>;
}

impl<T> Marker for SimpleMarker<T>
where
    T: 'static + ?Sized + Send + Sync,
{
    type Allocator = SimpleMarkerAllocator<T>;
    type Identifier = u64;

    fn id(&self) -> u64 {
        self.0
    }
}

/// Basic marker allocator, uses `u64` as identifier
pub struct SimpleMarkerAllocator<T: ?Sized> {
    index: u64,
    mapping: HashMap<u64, Entity>,
    _phantom_data: PhantomData<T>,
}

impl<T: ?Sized> Debug for SimpleMarkerAllocator<T> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("SimpleMarkerAllocator")
            .field("index", &self.index)
            .field("mapping", &self.mapping)
            .field("_phantom_data", &self._phantom_data)
            .finish()
    }
}

impl<T: ?Sized> Clone for SimpleMarkerAllocator<T> {
    fn clone(&self) -> Self {
        Self {
            index: self.index,
            mapping: self.mapping.clone(),
            _phantom_data: PhantomData,
        }
    }
}

impl<T: ?Sized> Default for SimpleMarkerAllocator<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: ?Sized> SimpleMarkerAllocator<T> {
    /// Create new `SimpleMarkerAllocator` which will yield `SimpleMarker`s
    /// starting with `0`
    pub fn new() -> Self {
        Self {
            index: 0,
            mapping: HashMap::new(),
            _phantom_data: PhantomData,
        }
    }
}

impl<T> MarkerAllocator<SimpleMarker<T>> for SimpleMarkerAllocator<T>
where
    T: 'static + ?Sized + Send + Sync,
{
    fn allocate(&mut self, entity: Entity, id: Option<u64>) -> SimpleMarker<T> {
        let marker = if let Some(id) = id {
            if id >= self.index {
                self.index = id + 1;
            }
            SimpleMarker(id, PhantomData)
        } else {
            self.index += 1;
            SimpleMarker(self.index - 1, PhantomData)
        };
        self.mapping.insert(marker.id(), entity);

        marker
    }

    fn retrieve_entity_internal(&self, id: u64) -> Option<Entity> {
        self.mapping.get(&id).cloned()
    }

    fn maintain(&mut self, entities: &EntitiesRes, storage: &ReadStorage<SimpleMarker<T>>) {
        // FIXME: may be too slow
        self.mapping = (entities, storage)
            .join()
            .map(|(e, m)| (m.id(), e))
            .collect();
    }
}
