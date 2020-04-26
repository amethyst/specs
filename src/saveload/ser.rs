use std::fmt::Display;

use serde::ser::{self, Serialize, SerializeSeq, Serializer};

use super::ConvertSaveload;
use crate::{
    join::Join,
    saveload::{
        marker::{Marker, MarkerAllocator},
        EntityData,
    },
    storage::{GenericReadStorage, ReadStorage, WriteStorage},
    world::{Component, EntitiesRes, Entity},
};

/// A trait which allows to serialize entities and their components.
pub trait SerializeComponents<E, M>
where
    M: Marker,
{
    /// The data representation of the components.
    type Data: Serialize;

    /// Serialize the components of a single entity using a entity -> marker
    /// mapping.
    fn serialize_entity<F>(&self, entity: Entity, ids: F) -> Result<Self::Data, E>
    where
        F: FnMut(Entity) -> Option<M>;

    /// Serialize components from specified storages
    /// of all marked entities with provided serializer.
    /// When the component gets serialized the closure passed
    /// in `ids` argument returns `None` for unmarked `Entity`.
    /// In this case serialization of this component may perform workaround or
    /// fail. So the function doesn't recursively mark referenced entities.
    /// For recursive marking see `serialize_recursive`
    fn serialize<S>(
        &self,
        entities: &EntitiesRes,
        markers: &ReadStorage<M>,
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        E: Display,
        S: Serializer,
    {
        let count = (entities, markers).join().count();
        let mut serseq = serializer.serialize_seq(Some(count))?;
        let ids = |entity| -> Option<M> { markers.get(entity).cloned() };
        for (entity, marker) in (entities, markers).join() {
            serseq.serialize_element(&EntityData::<M, Self::Data> {
                marker: marker.clone(),
                components: self
                    .serialize_entity(entity, &ids)
                    .map_err(ser::Error::custom)?,
            })?;
        }
        serseq.end()
    }

    /// Serialize components from specified storages
    /// of all marked entities with provided serializer.
    /// When the component gets serialized the closure passed
    /// in `ids` argument marks unmarked `Entity` (the marker of which was
    /// requested) and it will get serialized recursively.
    /// For serializing without such recursion see `serialize` function.
    fn serialize_recursive<S>(
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
                    if let Some((marker, added)) = allocator.mark(entity, markers) {
                        if added {
                            add.push((entity, marker.clone()));
                        }
                        Some(marker.clone())
                    } else {
                        None
                    }
                };
                for (entity, marker) in to_serialize {
                    serseq.serialize_element(&EntityData::<M, Self::Data> {
                        marker,
                        components: self
                            .serialize_entity(entity, &mut ids)
                            .map_err(ser::Error::custom)?,
                    })?;
                }
            }
            to_serialize = add;
        }
        serseq.end()
    }
}

macro_rules! serialize_components {
    ($($comp:ident => $sto:ident,)*) => {
        impl<'a, E, M, $($comp,)* $($sto,)*> SerializeComponents<E, M> for ($($sto,)*)
        where
            M: Marker,
            $(
                $sto: GenericReadStorage<Component = $comp>,
                $comp: ConvertSaveload<M> + Component,
                E: From<<$comp as ConvertSaveload<M>>::Error>,
            )*
        {
            type Data = ($(Option<$comp::Data>,)*);

            #[allow(unused)]
            fn serialize_entity<F>(&self, entity: Entity, mut ids: F) -> Result<Self::Data, E>
            where
                F: FnMut(Entity) -> Option<M>
            {
                #[allow(bad_style)]
                let ($(ref $comp,)*) = *self;

                Ok(($(
                    $comp.get(entity).map(|c| c.convert_into(&mut ids).map(Some)).unwrap_or(Ok(None))?,
                )*))
            }
        }

        serialize_components!(@pop $($comp => $sto,)*);
    };
    (@pop) => {};
    (@pop $head0:ident => $head1:ident, $($tail0:ident => $tail1:ident,)*) => {
        serialize_components!($($tail0 => $tail1,)*);
    };
}

serialize_components!(
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
