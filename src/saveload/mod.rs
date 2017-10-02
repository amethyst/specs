//! Save and load entites from various formats with serde

mod de;
mod details;
mod marker;
mod ser;

use self::details::{Components, EntityData, Storages};

pub use self::de::{deserialize, WorldDeserialize};
pub use self::details::SaveLoadComponent;
pub use self::marker::{Marker, MarkerAllocator, U64Marker, U64MarkerAllocator};
pub use self::ser::{serialize, serialize_recursive, WorldSerialize};
