//! Save and load entities from various formats with serde.
//!
//! ## `WorldSerialize` / `WorldDeserialize`
//!
//! This module provides two `SystemData` implementors:
//!
//! * `WorldSerialize` and
//! * `WorldDeserialize`
//!
//! Fetching those makes it very easy to serialize or deserialize
//! components. However, be aware that you cannot fetch storages
//! used in `WorldDeserialize` with the same system. E.g.
//! `type SystemData = (WorldDeserialize<'a, Marker, MyError, (Pos, Vel)>, WriteStorage<'a, Vel>)`
//! is not valid since both `WorldDeserialize` and `WriteStorage` would fetch the same component
//! storage mutably.
//!
//! `WorldSerialize` implements `Serialize` and `WorldDeserialize` implements
//! `DeserializeSeed`, so serializing / deserializing should be very easy.
//!
//! ## Markers
//!
//! Let's start simple. Because you usually don't want to serialize everything, we use
//! markers to say which entities we're interested in. However, these markers
//! aren't just boolean values; we'd like to also have id spaces which allow us
//! to identify entities even though local ids are different. And the allocation
//! of these ids is what `MarkerAllocator`s are responsible for. For an example,
//! see the docs for the `Marker` trait.
//!

mod de;
mod marker;
mod ser;
mod storages;

pub use self::de::DeserializeComponents;
pub use self::marker::{Marker, MarkerAllocator, U64Marker, U64MarkerAllocator};
pub use self::ser::SerializeComponents;

/// A struct used for deserializing entity data.
#[derive(Serialize, Deserialize)]
pub struct EntityData<M, D> {
    /// The marker the entity was mapped to.
    pub marker: M,
    /// The components associated with an entity.
    pub components: D,
}
