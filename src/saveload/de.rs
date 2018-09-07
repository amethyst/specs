use std::fmt::{self, Display, Formatter};
use std::marker::PhantomData;

use serde::de::{
    self, Deserialize, DeserializeOwned, DeserializeSeed, Deserializer, SeqAccess, Visitor,
};

use error::NoError;
use saveload::marker::{Marker, MarkerAllocator};
use saveload::EntityData;
use storage::{GenericWriteStorage, WriteStorage};
use world::{Component, EntitiesRes, Entity};

/// A trait which allows to deserialize entities and their components.
///
/// Instead of implementing this trait and its companion [`SerializeComponents`] directly,
/// you may wish to use the [`saveload_components`] macro.
///
/// [`SerializeComponents`]: ./SerializeComponents.t.html
/// [`saveload_components`]: ../macro.saveload_components.html
pub trait DeserializeComponents<E, M>
where
    Self: Sized,
    E: Display,
    M: Marker,
{
    /// The data representation that a component group gets deserialized to.
    type Data: DeserializeOwned;

    /// Loads `Component`s to entity from `Data` deserializable representation
    fn deserialize_entity<'a, F>(
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

/// Wrapper for `Entity` and tuple of `WriteStorage`s that implements `serde::Deserialize`.
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

/// Provides a function which converts a marked serialization wrapper
/// into its actual data type (usually a [`Component`]).
///
/// When serializing, specs will store the actual `Data` type
/// from [`IntoSerialize`] and upon deserialization, call
/// the `from` function to yield the real [`Component`].
///
/// This is automatically implemented for any type that is
/// [`DeserializeOwned`] (which includes
/// any type that derives [`Deserialize`]).
///
/// Implementing this yourself is usually only needed if you
/// have a component that points to another Entity (or has a field which does)
/// and you wish to [`Deserialize`] it.
///
/// In most cases, you also likely want to implement the companion
/// trait [`IntoSerialize`].
///
/// *Note*: if you're using `specs_derive`
/// and your struct does not have a generic bound (i.e. `struct Foo<T>`),
/// you can use `#[derive(Saveload)]` to automatically derive this and
/// [`IntoSerialize`]. You can get around generic type bounds by exploiting
/// the newtype pattern (e.g. `struct FooU32(Foo<u32>);`).
///
/// You must add the `derive` to any type that your component has a field of which does
/// not auto-implement these two traits, including the component itself (similar to how
/// normal [`Serialize`] and [`Deserialize`] work).
///
/// [`from`]: trait.FromDeserialize.html#tymethod.from
/// [`Component`]: ../trait.Component.html
/// [`Deserialize`]: https://docs.serde.rs/serde/trait.Deserialize.html
/// [`Serialize`]: https://docs.serde.rs/serde/trait.Serialize.html
/// [`DeserializeOwned`]: https://docs.serde.rs/serde/de/trait.DeserializeOwned.html
/// [`IntoSerialize`]: trait.IntoSerialize.html
///
/// # Example
///
/// ```rust
/// # extern crate specs;
/// # #[macro_use] extern crate serde;
/// use serde::Deserialize;
/// use specs::prelude::*;
/// use specs::error::NoError;
/// use specs::saveload::{Marker, FromDeserialize};
///
/// struct Target(Entity);
///
/// impl Component for Target {
///     type Storage = VecStorage<Self>;
/// }
///
/// // We need a matching "data" struct to hold our
/// // marker. In general, you just need a single struct
/// // per component you want to make `Deserialize` with each
/// // instance of `Entity` replaced with a generic "M".
/// #[derive(Deserialize)]
/// struct TargetData<M>(M);
///
/// impl<M: Marker> FromDeserialize<M> for Target
///     where
///     for<'de> M: Deserialize<'de>,
/// {
///     type Data = TargetData<M>;
///     type Error = NoError;
///
///     fn from<F>(data: Self::Data, mut ids: F) -> Result<Self, Self::Error>
///     where
///         F: FnMut(M) -> Option<Entity>
///     {
///         let entity = ids(data.0).unwrap();
///         Ok(Target(entity))
///     }
/// }
///
/// ```
///
pub trait FromDeserialize<M>: Sized {
    /// Serializable data representation
    type Data: DeserializeOwned;

    /// Error may occur during deserialization
    type Error;

    /// Convert this data from a deserializable form (`Data`) using
    /// entity to marker mapping function
    fn from<F>(data: Self::Data, ids: F) -> Result<Self, Self::Error>
    where
        F: FnMut(M) -> Option<Entity>;
}

impl<C, M> FromDeserialize<M> for C
where
    C: DeserializeOwned,
{
    type Data = Self;
    type Error = NoError;

    fn from<F>(data: Self::Data, _: F) -> Result<Self, Self::Error>
    where
        F: FnMut(M) -> Option<Entity>,
    {
        Ok(data)
    }
}

impl<M> FromDeserialize<M> for Entity
where
    M: DeserializeOwned,
{
    type Data = M;
    type Error = NoError;

    fn from<F>(data: Self::Data, mut func: F) -> Result<Self, Self::Error>
    where
        F: FnMut(M) -> Option<Entity>,
    {
        Ok(func(data).unwrap())
    }
}

/// Wrapper for `Entities` and tuple of `WriteStorage`s that implements `serde::de::Visitor`
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
                <$sto as GenericWriteStorage>::Component: FromDeserialize<M>+Component,
                E: From<<
                    <$sto as GenericWriteStorage>::Component as FromDeserialize<M>
                >::Error>,
            )*
        {
            type Data = ($(
                Option<
                    <<$sto as GenericWriteStorage>::Component as FromDeserialize<M>>::Data
                >,)*
            );

            #[allow(unused)]
            fn deserialize_entity<'a, F>(
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
                        $sto.insert(entity, FromDeserialize::<M>::from(component, &mut ids)?);
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
