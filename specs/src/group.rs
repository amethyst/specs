
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
    fn serialize_group<S: Serializer>(world: &specs::World, serializer: S) -> Result<S::Ok, S::Error>;
    #[cfg(feature="serialize")]
    /// Helper method for serializing the world.
    fn serialize_subgroup<S: Serializer>(world: &specs::World, map: &mut S::SerializeMap) -> Result<(), S::Error>;
    #[cfg(feature="serialize")]
    /// Deserializes the group of components into the world.
    fn deserialize_group<D: Deserializer>(world: &mut specs::World, entities: &[specs::Entity], deserializer: D) -> Result<(), D::Error>;
    #[cfg(feature="serialize")]
    /// Helper method for deserializing the world.
    fn deserialize_subgroup<V>(world: &mut specs::World, entities: &[specs::Entity], key: String, visitor: &mut V) -> Result<Option<()>, V::Error>
        where V: serde::de::MapVisitor;
}
