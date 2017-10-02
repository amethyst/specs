//! Provides `Marker` and `MarkerAllocator` traits

use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

use shred::Resource;
use {Component, DenseVecStorage, Entities, Entity, Join, ReadStorage, WriteStorage};

use serde::ser::Serialize;
use serde::de::DeserializeOwned;


/// This trait should be implemetened by component which is gonna be used as marker.
/// This marker should be set to entity that should be serialized.
/// If serialization strategy needs to set marker to some entity it should use
/// new marker allocated for `Marker::Allocator`.
///
/// ## Example
///
/// ```rust,no_run
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
/// fn main() {}
/// ```
pub trait Marker: Component + DeserializeOwned + Serialize + Copy {
    /// Id of the marker
    type Identifier: Copy + Debug + Eq + Hash;

    /// Allocator for this `Marker`
    type Allocator: MarkerAllocator<Self>;

    /// Get this marker internal id
    fn id(&self) -> Self::Identifier;

    /// Update marker with new value.
    /// It must preserve internal `Identifier`.
    ///
    /// ## Panics
    ///
    /// Allowed to panic if `self.id() != update.id()`.
    /// But usually implementer may ignore `update.id()` value
    /// as deserialization algorithm ensures `id()`s match.
    fn update(&mut self, update: Self) {
        ::std::mem::drop(update);
    }
}

/// This allocator is used with `Marker` trait.
/// It provides method for allocation of `Marker`s.
/// And also should provide `Marker -> Entity` mapping
/// `maintain` method can be implemented for cleanup and actualization.
/// See docs for `Marker` for example.
pub trait MarkerAllocator<M: Marker>: Resource {
    /// Allocate new `Marker`.
    /// Stores mapping `Marker` -> `Entity`.
    /// If _id_ argument is `Some(id)` then new marker will have this `id`.
    /// Otherwise allocator creates marker with new unique id.
    fn allocate(&mut self, entity: Entity, id: Option<M::Identifier>) -> M;

    /// Get `Entity` by `Marker::Identifier`
    fn get(&self, id: M::Identifier) -> Option<Entity>;

    /// Create new unique marker `M` and attach it to entity.
    /// Or get old marker if already marked.
    fn mark<'a>(&mut self, entity: Entity, storage: &mut WriteStorage<'a, M>) -> M {
        match storage.get(entity).cloned() {
            Some(marker) => marker,
            None => {
                let marker = self.allocate(entity, None);
                storage.insert(entity, marker);
                marker
            }
        }
    }

    /// Find `Entity` by `Marker` with same id and update `Marker` attached instance.
    /// Or create new entity and mark it.
    fn get_marked<'a>(
        &mut self,
        id: M::Identifier,
        entities: &Entities<'a>,
        storage: &mut WriteStorage<'a, M>,
    ) -> Entity {
        if let Some(entity) = self.get(id) {
            if entities.is_alive(entity) {
                return entity;
            }
        }

        let entity = entities.create();
        let marker = self.allocate(entity, Some(id));
        storage.insert(entity, marker);
        entity
    }

    /// Maintain internal data. Cleanup if necessary.
    fn maintain<'a>(&mut self, _entities: &Entities<'a>, _storage: &ReadStorage<'a, M>) {}
}

/// Basic marker implementation usable for saving and loading
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct U64Marker(pub u64);
impl Component for U64Marker {
    type Storage = DenseVecStorage<Self>;
}

impl Marker for U64Marker {
    type Identifier = u64;
    type Allocator = U64MarkerAllocator;
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

    fn get(&self, id: u64) -> Option<Entity> {
        self.mapping.get(&id).cloned()
    }

    fn maintain<'a>(&mut self, entities: &Entities<'a>, storage: &ReadStorage<'a, U64Marker>) {
        // FIXME: may be too slow
        self.mapping = (&**entities, storage)
            .join()
            .map(|(e, m)| (m.id(), e))
            .collect();
    }
}
