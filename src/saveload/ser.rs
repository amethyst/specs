use std::fmt::Display;

use serde::ser::{self, Serialize, SerializeSeq, Serializer};

use error::NoError;
use join::Join;
use saveload::marker::{Marker, MarkerAllocator};
use saveload::EntityData;
use storage::{GenericReadStorage, ReadStorage, WriteStorage};
use world::{Component, EntitiesRes, Entity};

/// Converts a data type (usually a [`Component`]) into its serializable form.
///
/// This is automatically implemented for any type that is
/// [`Serialize`], yielding itself.
///
/// Implementing this yourself is usually only needed if you
/// have a component that points to another Entity, or has a field which does,
///  and you wish to [`Serialize`] it.
///
/// In most cases, you also likely want to implement the companion
/// trait [`FromDeserialize`].
///
/// *Note*: if you're using `specs_derive`
/// and your struct does not have a generic bound (i.e. `struct Foo<T>`),
/// you can use `#[derive(Saveload)]` to automatically derive this and
/// [`FromDeserialize`]. You can get around generic type bounds by exploiting
/// the newtype pattern (e.g. `struct FooU32(Foo<u32>);`).
///
/// You must add the `derive` to any type that your component holds which does
/// not auto-implement these two traits, including the component itself (similar to how
/// normal [`Serialize`] and [`Deserialize`] work).
///
/// [`Component`]: ../trait.Component.html
/// [`Serialize`]: https://docs.serde.rs/serde/trait.Serialize.html
/// [`Deserialize`]: https://docs.serde.rs/serde/trait.Deserialize.html
/// [`FromDeserialize`]: trait.FromDeserialize.html
///
/// # Example
///
/// ```rust
/// # extern crate specs;
/// # #[macro_use] extern crate serde;
/// use serde::Serialize;
/// use specs::prelude::*;
/// use specs::error::NoError;
/// use specs::saveload::{Marker, IntoSerialize};
///
/// struct Target(Entity);
///
/// impl Component for Target {
///     type Storage = VecStorage<Self>;
/// }
///
/// // We need a matching "data" struct to hold our
/// // marker. In general, you just need a single struct
/// // per component you want to make `Serialize` with each
/// // instance of `Entity` replaced with a generic "M".
/// #[derive(Serialize)]
/// struct TargetData<M>(M);
///
/// impl<M: Marker + Serialize> IntoSerialize<M> for Target {
///     type Data = TargetData<M>;
///     type Error = NoError;
///
///     fn into<F>(&self, mut ids: F) -> Result<Self::Data, Self::Error>
///     where
///         F: FnMut(Entity) -> Option<M>
///     {
///         let marker = ids(self.0).unwrap();
///         Ok(TargetData(marker))
///     }
/// }
///
/// ```
///
pub trait IntoSerialize<M> {
    /// Serializable data representation for data type
    type Data: Serialize;

    /// Error may occur during serialization or deserialization of component
    type Error;

    /// Convert this data type into serializable form (`Data`) using
    /// entity to marker mapping function
    fn into<F>(&self, ids: F) -> Result<Self::Data, Self::Error>
    where
        F: FnMut(Entity) -> Option<M>;
}

impl<C, M> IntoSerialize<M> for C
where
    C: Clone + Serialize,
{
    type Data = Self;
    type Error = NoError;

    fn into<F>(&self, _: F) -> Result<Self::Data, Self::Error>
    where
        F: FnMut(Entity) -> Option<M>,
    {
        Ok(self.clone())
    }
}

impl<M> IntoSerialize<M> for Entity
where
    M: Serialize,
{
    type Data = M;
    type Error = NoError;

    fn into<F>(&self, mut func: F) -> Result<Self::Data, Self::Error>
    where
        F: FnMut(Entity) -> Option<M>,
    {
        Ok(func(*self).unwrap())
    }
}

/// A trait which allows to serialize entities and their components.
pub trait SerializeComponents<E, M>
where
    M: Marker,
{
    /// The data representation of the components.
    type Data: Serialize;

    /// Serialize the components of a single entity using a entity -> marker mapping.
    fn serialize_entity<F>(&self, entity: Entity, ids: F) -> Result<Self::Data, E>
    where
        F: FnMut(Entity) -> Option<M>;

    /// Serialize components from specified storages
    /// of all marked entities with provided serializer.
    /// When the component gets serialized the closure passed
    /// in `ids` argument returns `None` for unmarked `Entity`.
    /// In this case serialization of this component may perform workaround or fail.
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
        for (entity, marker) in (&*entities, &*markers).join() {
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
    /// in `ids` argument marks unmarked `Entity` (the marker of which was requested)
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
                $comp : IntoSerialize<M> + Component,
                E: From<<$comp as IntoSerialize<M>>::Error>,
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
                    $comp.get(entity).map(|c| c.into(&mut ids).map(Some)).unwrap_or(Ok(None))?,
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
