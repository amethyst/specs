//! Provides `Marker` and `MarkerAllocator` traits

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use join::Join;
use shred::Resource;
use storage::{DenseVecStorage, ReadStorage, WriteStorage};
use world::{Component, EntitiesRes, Entity, EntityBuilder};

use serde::de::DeserializeOwned;
use serde::ser::Serialize;

impl<'a> EntityBuilder<'a> {
    /// Add a `Marker` to the entity by fetching the associated allocator.
    ///
    /// ## Examples
    ///
    /// ```
    /// use specs::prelude::*;
    /// use specs::saveload::{U64Marker, U64MarkerAllocator};
    ///
    /// let mut world = World::new();
    /// world.register::<U64Marker>();
    /// world.add_resource(U64MarkerAllocator::new());
    ///
    /// world
    ///     .create_entity()
    ///     /* .with(Component1) */
    ///     .marked::<U64Marker>()
    ///     .build();
    /// ```
    ///
    /// ## Panics
    ///
    /// Panics in case there's no allocator added to the `World`.
    pub fn marked<M>(self) -> Self
    where
        M: Marker,
    {
        let mut alloc = self.world.write_resource::<M::Allocator>();
        alloc.mark(self.entity, &mut self.world.write::<M>());

        self
    }
}

/// This trait should be implemented by a component which is going to be used as marker.
/// This marker should be set to entity that should be serialized.
/// If serialization strategy needs to set marker to some entity
/// then it should use newly allocated marker from `Marker::Allocator`.
///
/// ## Example
///
/// ```rust
/// extern crate specs;
/// #[macro_use] extern crate serde;
/// use std::collections::HashMap;
/// use std::ops::Range;
///
/// use specs::prelude::*;
/// use specs::saveload::{Marker, MarkerAllocator};
///
/// // Marker for entities that should be synced over network
/// #[derive(Clone, Copy, Serialize, Deserialize)]
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
///     type Identifier = u64;
///     type Allocator = NetNode;
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
///             self.range.next().expect("Id range must be virtually endless")
///         });
///         let marker = NetMarker {
///             id: id,
///             seq: 0,
///         };
///         self.mapping.insert(id, entity);
///         marker
///     }
///
///     fn try_get(&self, id: u64) -> Option<Entity> {
///         self.mapping.get(&id).cloned()
///     }
/// }
///
/// fn main() {
///     let mut world = World::new();
///     world.register::<NetMarker>();
///
///     let mut node = NetNode {
///         range: 0..100,
///         mapping: HashMap::new(),
///     };
///
///     let entity = world.create_entity().build();
///     let (marker, added) = node.mark(entity, &mut world.write::<NetMarker>());
///     assert!(added);
///     assert_eq!(
///         node.get_marked(marker.id(), &world.entities(), &mut world.write::<NetMarker>()),
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
    /// This value may never change.
    fn id(&self) -> Self::Identifier;

    /// This gets called when an entity is fetched by a marker.
    /// It can be used to update internal data that is not used for
    /// identification.
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
/// It provides method for allocation new `Marker`s.
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
            if let Some(marker_comp) = storage.get_mut(entity) {
                marker_comp.update(marker);

                return entity;
            }
        }

        let entity = entities.create();
        let marker = self.allocate(entity, Some(marker.id()));

        storage.insert(entity, marker);
        entity
    }

    /// Create new unique marker `M` and attach it to entity.
    /// Or get old marker if this entity is already marked.
    fn mark<'m>(&mut self, entity: Entity, storage: &'m mut WriteStorage<M>) -> (&'m M, bool) {
        let mut new = false;

        let marker = storage
            .entry(entity)
            .unwrap()
            .or_insert_with(|| {
                new = true;
                self.allocate(entity, None)
            });

        (marker, new)
    }

    /// Maintain internal data. Cleanup if necessary.
    fn maintain(&mut self, _entities: &EntitiesRes, _storage: &ReadStorage<M>);
}

/// Basic marker implementation usable for saving and loading
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct U64Marker(u64);

impl Component for U64Marker {
    type Storage = DenseVecStorage<Self>;
}

impl Marker for U64Marker {
    type Allocator = U64MarkerAllocator;
    type Identifier = u64;

    fn id(&self) -> u64 {
        self.0
    }
}

/// Basic marker allocator
#[derive(Clone, Debug)]
pub struct U64MarkerAllocator {
    index: u64,
    mapping: HashMap<u64, Entity>,
}

impl Default for U64MarkerAllocator {
    fn default() -> Self {
        U64MarkerAllocator::new()
    }
}

impl U64MarkerAllocator {
    /// Create new `U64MarkerAllocator` which will yield `U64Marker`s starting with `0`
    pub fn new() -> Self {
        U64MarkerAllocator {
            index: 0,
            mapping: HashMap::new(),
        }
    }
}

impl MarkerAllocator<U64Marker> for U64MarkerAllocator {
    fn allocate(&mut self, entity: Entity, id: Option<u64>) -> U64Marker {
        let marker = if let Some(id) = id {
            U64Marker(id)
        } else {
            self.index += 1;
            U64Marker(self.index - 1)
        };
        self.mapping.insert(marker.id(), entity);

        marker
    }

    fn retrieve_entity_internal(&self, id: u64) -> Option<Entity> {
        self.mapping.get(&id).cloned()
    }

    fn maintain(&mut self, entities: &EntitiesRes, storage: &ReadStorage<U64Marker>) {
        // FIXME: may be too slow
        self.mapping = (&*entities, storage)
            .join()
            .map(|(e, m)| (m.id(), e))
            .collect();
    }
}
