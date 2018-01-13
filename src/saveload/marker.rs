//! Provides `Marker` and `MarkerAllocator` traits

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use {Component, DenseVecStorage, EntitiesRes, Entity, EntityBuilder, Join, ReadStorage, WriteStorage};
use shred::Resource;

use serde::de::DeserializeOwned;
use serde::ser::Serialize;

impl<'a> EntityBuilder<'a> {
    /// Add a `Marker` to the entity by fetching the associated allocator.
    ///
    /// ## Examples
    ///
    /// ```
    /// use specs::saveload::{U64Marker, U64MarkerAllocator};
    /// use specs::World;
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
/// use specs::{Component, Entity, DenseVecStorage};
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
///     fn get(&self, id: u64) -> Option<Entity> {
///         self.mapping.get(&id).cloned()
///     }
/// }
///
/// fn main() {
///     use specs::World;
///
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
    type Identifier: Default;
    /// Allocator for this `Marker`
    type Allocator: MarkerAllocator<Self>;

    /// Get this marker internal id
    fn id(&self) -> Self::Identifier;
}

/// This allocator is used with the `Marker` trait.
/// It provides method for allocation new `Marker`s.
/// It should also provide a `Marker -> Entity` mapping.
/// The `maintain` method can be implemented for cleanup and actualization.
/// See docs for `Marker` for an example.
pub trait MarkerAllocator<M: Marker>: Resource {
    /// Allocate new `Marker`.
    /// Stores mapping `Marker` -> `Entity`.
    /// If _id_ argument is `Some(id)` then the new marker will have this `id`.
    /// Otherwise allocator creates marker with a new unique id.
    fn allocate(&mut self, entity: Entity, id: Option<M::Identifier>) -> M;

    /// Get `Entity` by `Marker::Identifier`
    fn try_get(&self, id: &M::Identifier) -> Option<Entity>;

    /// Find an `Entity` by a `Marker` with same id
    /// and update `Marker` attached to the instance.
    /// Or create new entity and mark it.
    fn get_or_create(
        &mut self,
        id: M::Identifier,
        entities: &EntitiesRes,
        storage: &mut WriteStorage<M>,
    ) -> Entity {
        if let Some(entity) = self.try_get(&id) {
            if entities.is_alive(entity) {
                return entity;
            }
        }

        let entity = entities.create();
        let marker = self.allocate(entity, Some(id));
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
    fn maintain(&mut self, _entities: &EntitiesRes, _storage: &ReadStorage<M>) {}
}

/// Basic marker implementation usable for saving and loading
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct U64Marker(pub u64);
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

    fn try_get(&self, id: &u64) -> Option<Entity> {
        self.mapping.get(id).cloned()
    }

    fn maintain(&mut self, entities: &EntitiesRes, storage: &ReadStorage<U64Marker>) {
        // FIXME: may be too slow
        self.mapping = (entities, storage)
            .join()
            .map(|(e, m)| (m.id(), e))
            .collect();
    }
}
