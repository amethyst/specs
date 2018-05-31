//! Save and load entities from various formats with serde.
//!
//! ## `WorldSerialize` / `WorldDeserialize`
//!
//! This module provides two `SystemData` implementors:
//!
//! * `SerializeComponents` and
//! * `DeserializeComponents`
//!
//! Reading those makes it very easy to serialize or deserialize
//! components.
//!
//! `SerializeComponents` implements `Serialize` and `DeserializeComponents` implements
//! `DeserializeOwned`, so serializing / deserializing should be very easy.
//!
//! ## Markers
//!
//! Because you usually don't want to serialize everything, we use
//! markers to say which entities we're interested in. However, these markers
//! aren't just boolean values; we also have id spaces which allow us
//! to identify entities even if local ids are different. The allocation
//! of these ids is what `MarkerAllocator`s are responsible for. For an example,
//! see the docs for the `Marker` trait.
//!

mod macros;

mod de;
mod marker;
mod ser;
#[cfg(test)]
mod tests;

pub use self::de::{DeserializeComponents, FromDeserialize};
pub use self::marker::{Marker, MarkerAllocator, U64Marker, U64MarkerAllocator};
pub use self::ser::{IntoSerialize, SerializeComponents};

/// A struct used for deserializing entity data.
#[derive(Serialize, Deserialize)]
pub struct EntityData<M, D> {
    /// The marker the entity was mapped to.
    pub marker: M,
    /// The components associated with an entity.
    pub components: D,
}
