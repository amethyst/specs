use std::fmt::Display;

use serde::ser::{self, Serialize, SerializeSeq, Serializer};

use {Component, EntitiesRes, Entity, Join, ReadStorage, WriteStorage};

use saveload::EntityData;
use saveload::marker::{Marker, MarkerAllocator};

//use saveload::storages::{GenericReadStorage, GenericWriteStorage};

pub trait IntoSerialize<M>: Component {
    /// Serializable data representation for component
    type Data: Serialize;

    /// Error may occur during serialization or deserialization of component
    type Error;

    /// Convert this component into serializable form (`Data`) using
    /// entity to marker mapping function
    fn into<F>(&self, ids: F) -> Result<&Self::Data, Self::Error>
    where
        F: FnMut(Entity) -> Option<M>;
}

/// A trait which allows to serialize entities and their components.
pub trait SerializeComponents<E, M>
where
    M: Marker,
{
    /// The data representation of the components.
    type Data: Serialize;

    /// Serialize the components of a single entiy using a entity -> marker mapping.
    fn serialize_entity<F>(&self, entity: Entity, ids: F) -> Result<Self::Data, E>
    where
        F: FnMut(Entity) -> Option<M>;

    /// Serialize components from specified storages via `SerializableComponent::save`
    /// of all marked entities with provided serializer.
    /// When the component gets serialized with `SerializableComponent::save` method
    /// the closure passed in `ids` argument returns `None` for unmarked `Entity`.
    /// In this case `SerializableComponent::save` may perform workaround (forget about `Entity`)
    /// or fail.
    /// So the function doesn't recursively mark referenced entities.
    /// For recursive marking see `serialize_recursive`
    fn serialize<'ms, S>(
        &self,
        entities: &EntitiesRes,
        markers: &ReadStorage<M>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        E: Display,
        S: Serializer,
    {
        let mut serseq = serializer.serialize_seq(None)?;
        let ids = |entity| -> Option<M> { markers.get(entity).cloned() };
        for (entity, marker) in (entities, &*markers).join() {
            serseq.serialize_element(&EntityData::<M, Self::Data> {
                marker: marker.clone(),
                components: self.serialize_entity(entity, &ids)
                    .map_err(ser::Error::custom)?, // TODO: revise
            })?;
        }
        serseq.end()
    }

    /// Serialize components from specified storages via `SerializableComponent::save`
    /// of all marked entities with provided serializer.
    /// When the component gets serialized with `SerializableComponent::save` method
    /// the closure passed in `ids` argument marks unmarked `Entity` (the marker of which was requested)
    /// and it will get serialized recursively.
    /// For serializing without such recursion see `serialize` function.
    fn serialize_recursive<MS, S>(
        &self,
        entities: &EntitiesRes,
        markers: &mut WriteStorage<M>,
        allocator: &mut M::Allocator,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        E: Display,
        M: Marker,
        S: Serializer,
    {
        let mut serseq = serializer.serialize_seq(None)?;
        let mut to_serialize: Vec<(Entity, M)> = (entities, &*markers)
            .join()
            .map(|(e, m)| (e, m.clone()))
            .collect();
        while !to_serialize.is_empty() {
            let mut add = vec![];
            {
                let mut ids = |entity| -> Option<M> {
                    let (marker, added) = allocator.mark(entity, markers);
                    if added {
                        add.push((entity, marker.clone()));
                    }
                    Some(marker.clone())
                };
                for (entity, marker) in to_serialize {
                    serseq.serialize_element(&EntityData::<M, Self::Data> {
                        marker,
                        components: self.serialize_entity(entity, &mut ids)
                            .map_err(ser::Error::custom)?,
                    })?;
                }
            }
            to_serialize = add;
        }
        serseq.end()
    }
}

/*
macro_rules! serialize_components {
    ($($comp:ident,)*) => {
        impl<'a, E, M, $($comp,)*> SerializeComponents<E, M> for ($(ReadStorage<'a, $comp>,)*)
        where
            $($comp : IntoSerialize<M>,)*
        {

        }
    };
    (;;) => {};
    (@pop) => {};
    (@pop $head:ident, $($tail:ident,)*) => {
        $($tail,)*
    };
}
*/
