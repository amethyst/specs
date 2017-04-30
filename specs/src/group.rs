
use ::{World, Entity};
#[cfg(feature="serialize")]
use serde::{self, Serializer, Deserializer};

/// Group of components and subgroups containing components.
pub trait ComponentGroup {
    /// Components defined in this group, not a subgroup.
    fn local_components() -> Vec<&'static str>;
    /// Components defined in this group along with subgroups.
    fn components() -> Vec<&'static str>;
    /// Subgroups included in this group.
    fn subgroups() -> Vec<&'static str>;
    
    #[cfg(feature="serialize")]
    /// Serializes the group of components from the world.
    fn serialize_group<S: Serializer, C>(world: &World<C>, serializer: S) -> Result<S::Ok, S::Error>;
    #[cfg(feature="serialize")]
    /// Helper method for serializing the world.
    fn serialize_subgroup<S: Serializer, C>(world: &World<C>, map: &mut S::SerializeMap) -> Result<(), S::Error>;
    #[cfg(feature="serialize")]
    /// Deserializes the group of components into the world.
    fn deserialize_group<D: Deserializer, C>(world: &mut World<C>, entities: &[Entity], deserializer: D) -> Result<(), D::Error>;
    #[cfg(feature="serialize")]
    /// Helper method for deserializing the world.
    fn deserialize_subgroup<V, C>(world: &mut World<C>, entities: &[Entity], key: String, visitor: &mut V) -> Result<Option<()>, V::Error>
        where V: serde::de::MapVisitor;
}
