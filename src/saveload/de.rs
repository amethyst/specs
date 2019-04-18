use std::{
    fmt::{self, Display, Formatter},
    marker::PhantomData,
};

use serde::de::{
    self, Deserialize, DeserializeOwned, DeserializeSeed, Deserializer, SeqAccess, Visitor,
};

use super::ConvertSaveload;
use crate::{
    saveload::{
        marker::{Marker, MarkerAllocator},
        EntityData,
    },
    storage::{GenericWriteStorage, WriteStorage},
    world::{Component, EntitiesRes, Entity},
};

/// A trait which allows to deserialize entities and their components.
pub trait DeserializeComponents<E, M>
where
    Self: Sized,
    E: Display,
    M: Marker,
{
    /// The data representation that a component group gets deserialized to.
    type Data: DeserializeOwned;

    /// Loads `Component`s to entity from `Data` deserializable representation
    fn deserialize_entity<F>(
        &mut self,
        entity: Entity,
        components: Self::Data,
        ids: F,
    ) -> Result<(), E>
    where
        F: FnMut(M) -> Option<Entity>;

    /// Deserialize entities according to markers.
    fn deserialize<'a: 'b, 'b, 'de, D>(
        &'b mut self,
        entities: &'b EntitiesRes,
        markers: &'b mut WriteStorage<'a, M>,
        allocator: &'b mut M::Allocator,
        deserializer: D,
    ) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(VisitEntities::<E, M, Self> {
            allocator,
            entities,
            markers,
            storages: self,
            pd: PhantomData,
        })
    }
}

/// Wrapper for `Entity` and tuple of `WriteStorage`s that implements
/// `serde::Deserialize`.
struct DeserializeEntity<'a: 'b, 'b, E, M: Marker, S: 'b> {
    allocator: &'b mut M::Allocator,
    entities: &'b EntitiesRes,
    storages: &'b mut S,
    markers: &'b mut WriteStorage<'a, M>,
    pd: PhantomData<E>,
}

impl<'de, 'a: 'b, 'b, E, M, S> DeserializeSeed<'de> for DeserializeEntity<'a, 'b, E, M, S>
where
    E: Display,
    M: Marker,
    S: DeserializeComponents<E, M>,
{
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<(), D::Error>
    where
        D: Deserializer<'de>,
    {
        let DeserializeEntity {
            entities,
            storages,
            markers,
            allocator,
            ..
        } = self;
        let data = EntityData::<M, S::Data>::deserialize(deserializer)?;
        let entity = allocator.retrieve_entity(data.marker, markers, entities);
        let ids = |marker: M| Some(allocator.retrieve_entity(marker, markers, entities));

        storages
            .deserialize_entity(entity, data.components, ids)
            .map_err(de::Error::custom)
    }
}

/// Wrapper for `Entities` and tuple of `WriteStorage`s that implements
/// `serde::de::Visitor`
struct VisitEntities<'a: 'b, 'b, E, M: Marker, S: 'b> {
    allocator: &'b mut M::Allocator,
    entities: &'b EntitiesRes,
    markers: &'b mut WriteStorage<'a, M>,
    storages: &'b mut S,
    pd: PhantomData<E>,
}

impl<'de, 'a: 'b, 'b, E, M, S> Visitor<'de> for VisitEntities<'a, 'b, E, M, S>
where
    E: Display,
    M: Marker,
    S: DeserializeComponents<E, M>,
{
    type Value = ();

    fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "Sequence of serialized entities")
    }

    fn visit_seq<SEQ>(self, mut seq: SEQ) -> Result<(), SEQ::Error>
    where
        SEQ: SeqAccess<'de>,
    {
        loop {
            let ret = seq.next_element_seed(DeserializeEntity {
                entities: self.entities,
                storages: self.storages,
                markers: self.markers,
                allocator: self.allocator,
                pd: self.pd,
            })?;

            if ret.is_none() {
                break Ok(());
            }
        }
    }
}

macro_rules! deserialize_components {
    ($($comp:ident => $sto:ident,)*) => {
        impl<'b, E, M, $($sto,)*> DeserializeComponents<E, M> for ($($sto,)*)
        where
            E: Display,
            M: Marker,
            $(
                $sto: GenericWriteStorage,
                <$sto as GenericWriteStorage>::Component: ConvertSaveload<M> + Component,
                E: From<<
                    <$sto as GenericWriteStorage>::Component as ConvertSaveload<M>
                >::Error>,
            )*
        {
            type Data = ($(
                Option<
                    <<$sto as GenericWriteStorage>::Component as ConvertSaveload<M>>::Data
                >,)*
            );

            #[allow(unused)]
            fn deserialize_entity<F>(
                &mut self,
                entity: Entity,
                components: Self::Data,
                mut ids: F,
            ) -> Result<(), E>
            where
                F: FnMut(M) -> Option<Entity>
            {
                #[allow(bad_style)]
                let ($(ref mut $sto,)*) = *self;
                #[allow(bad_style)]
                let ($($comp,)*) = components;
                $(
                    if let Some(component) = $comp {
                        $sto.insert(entity, ConvertSaveload::<M>::convert_from(component, &mut ids)?);
                    } else {
                        $sto.remove(entity);
                    }
                )*
                Ok(())
            }
        }

        deserialize_components!(@pop $($comp => $sto,)*);
    };
    (@pop) => {};
    (@pop $head0:ident => $head1:ident, $($tail0:ident => $tail1:ident,)*) => {
        deserialize_components!($($tail0 => $tail1,)*);
    };
}

deserialize_components!(
    CA => SA,
    CB => SB,
    CC => SC,
    CD => SD,
    CE => SE,
    CF => SF,
    CG => SG,
    CH => SH,
    CI => SI,
    CJ => SJ,
    CK => SK,
    CL => SL,
    CN => SN,
    CM => SM,
    CO => SO,
    CP => SP,
);
