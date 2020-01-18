use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    join::Join,
    saveload::{Marker, MarkerAllocator},
    storage::{ReadStorage, VecStorage},
    world::{Component, EntitiesRes, Entity},
};

/// Basic marker uuid implementation usable for saving and loading.
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct UuidMarker {
    uuid: Uuid,
}

impl Component for UuidMarker {
    type Storage = VecStorage<Self>;
}

impl Marker for UuidMarker {
    type Allocator = UuidMarkerAllocator;
    type Identifier = Uuid;

    fn id(&self) -> Uuid {
        self.uuid()
    }
}

impl UuidMarker {
    /// Creates a new `UuidMarker` Component from the specified uuid.
    pub fn new(uuid: Uuid) -> Self {
        UuidMarker { uuid }
    }

    /// Creates a new `UuidMarker` Component with a random uuid.
    pub fn new_random() -> Self {
        let uuid = Uuid::new_v4();
        UuidMarker { uuid }
    }

    /// Get the current uuid.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }
}

/// Basic marker allocator for uuid.
#[derive(Clone, Debug)]
pub struct UuidMarkerAllocator {
    mapping: HashMap<Uuid, Entity>,
}

impl Default for UuidMarkerAllocator {
    fn default() -> Self {
        UuidMarkerAllocator::new()
    }
}

impl UuidMarkerAllocator {
    /// Create new `UuidMarkerAllocator` which will yield `UuidMarker`s.
    pub fn new() -> Self {
        Self {
            mapping: HashMap::new(),
        }
    }
}

impl MarkerAllocator<UuidMarker> for UuidMarkerAllocator {
    fn allocate(&mut self, entity: Entity, id: Option<Uuid>) -> UuidMarker {
        let marker = if let Some(id) = id {
            UuidMarker::new(id)
        } else {
            UuidMarker::new_random()
        };
        self.mapping.insert(marker.uuid(), entity);

        marker
    }

    fn retrieve_entity_internal(&self, id: Uuid) -> Option<Entity> {
        self.mapping.get(&id).cloned()
    }

    fn maintain(&mut self, entities: &EntitiesRes, storage: &ReadStorage<UuidMarker>) {
        // FIXME: may be too slow
        self.mapping = (entities, storage)
            .join()
            .map(|(e, m)| (m.uuid(), e))
            .collect();
    }
}
