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
//! `SerializeComponents` implements `Serialize` and `DeserializeComponents`
//! implements `DeserializeOwned`, so serializing / deserializing should be very
//! easy.
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

use std::convert::Infallible;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::world::Entity;

mod de;
mod marker;
mod ser;
#[cfg(test)]
mod tests;
#[cfg(feature = "uuid_entity")]
mod uuid;

#[cfg(feature = "uuid_entity")]
pub use self::uuid::{UuidMarker, UuidMarkerAllocator};
pub use self::{
    de::DeserializeComponents,
    marker::{MarkedBuilder, Marker, MarkerAllocator, SimpleMarker, SimpleMarkerAllocator},
    ser::SerializeComponents,
};

/// A struct used for deserializing entity data.
#[derive(Serialize, Deserialize)]
pub struct EntityData<M, D> {
    /// The marker the entity was mapped to.
    pub marker: M,
    /// The components associated with an entity.
    pub components: D,
}

/// Converts a data type (usually a [`Component`]) into its serializable form
/// and back to actual data from it's deserialized form.
///
/// This is automatically implemented for any type that is
/// [`Clone`], [`Serialize`] and [`DeserializeOwned`].
///
/// Implementing this yourself is usually only needed if you
/// have a component that points to another [`Entity`], or has a field which
/// does,  and you wish to [`Serialize`] it.
///
/// *Note*: if you're using `specs_derive`
/// you can use `#[derive(Saveload)]` to automatically derive this.
///
/// You must add the `derive` to any type that your component holds which does
/// not auto-implement this traits, including the component itself (similar to
/// how normal [`Serialize`] and [`Deserialize`] work).
///
/// [`Component`]: ../trait.Component.html
/// [`Entity`]: ../struct.Entity.html
/// [`Serialize`]: https://docs.serde.rs/serde/trait.Serialize.html
/// [`Deserialize`]: https://docs.serde.rs/serde/trait.Deserialize.html
/// [`DeserializeOwned`]: https://docs.serde.rs/serde/de/trait.DeserializeOwned.html
///
/// # Example
///
/// ```rust
/// # extern crate specs;
/// # #[macro_use] extern crate serde;
/// use serde::{Deserialize, Serialize};
/// use specs::{
///     prelude::*,
///     saveload::{ConvertSaveload, Marker},
/// };
/// use std::convert::Infallible;
///
/// struct Target(Entity);
///
/// impl Component for Target {
///     type Storage = VecStorage<Self>;
/// }
///
/// // We need a matching "data" struct to hold our
/// // marker. In general, you just need a single struct
/// // per component you want to make `Serialize`/`Deserialize` with each
/// // instance of `Entity` replaced with a generic "M".
/// #[derive(Serialize, Deserialize)]
/// struct TargetData<M>(M);
///
/// impl<M: Marker + Serialize> ConvertSaveload<M> for Target
/// where
///     for<'de> M: Deserialize<'de>,
/// {
///     type Data = TargetData<M>;
///     type Error = Infallible;
///
///     fn convert_into<F>(&self, mut ids: F) -> Result<Self::Data, Self::Error>
///     where
///         F: FnMut(Entity) -> Option<M>,
///     {
///         let marker = ids(self.0).unwrap();
///         Ok(TargetData(marker))
///     }
///
///     fn convert_from<F>(data: Self::Data, mut ids: F) -> Result<Self, Self::Error>
///     where
///         F: FnMut(M) -> Option<Entity>,
///     {
///         let entity = ids(data.0).unwrap();
///         Ok(Target(entity))
///     }
/// }
/// ```
pub trait ConvertSaveload<M>: Sized {
    /// (De)Serializable data representation for data type
    type Data: Serialize + DeserializeOwned;

    /// Error may occur during serialization or deserialization of component
    type Error;

    /// Convert this data from a deserializable form (`Data`) using
    /// entity to marker mapping function
    fn convert_from<F>(data: Self::Data, ids: F) -> Result<Self, Self::Error>
    where
        F: FnMut(M) -> Option<Entity>;

    /// Convert this data type into serializable form (`Data`) using
    /// entity to marker mapping function
    fn convert_into<F>(&self, ids: F) -> Result<Self::Data, Self::Error>
    where
        F: FnMut(Entity) -> Option<M>;
}

impl<C, M> ConvertSaveload<M> for C
where
    C: Clone + Serialize + DeserializeOwned,
{
    type Data = Self;
    type Error = Infallible;

    fn convert_into<F>(&self, _: F) -> Result<Self::Data, Self::Error>
    where
        F: FnMut(Entity) -> Option<M>,
    {
        Ok(self.clone())
    }

    fn convert_from<F>(data: Self::Data, _: F) -> Result<Self, Self::Error>
    where
        F: FnMut(M) -> Option<Entity>,
    {
        Ok(data)
    }
}

impl<M> ConvertSaveload<M> for Entity
where
    M: Serialize + DeserializeOwned,
{
    type Data = M;
    type Error = Infallible;

    fn convert_into<F>(&self, mut func: F) -> Result<Self::Data, Self::Error>
    where
        F: FnMut(Entity) -> Option<M>,
    {
        Ok(func(*self).unwrap())
    }

    fn convert_from<F>(data: Self::Data, mut func: F) -> Result<Self, Self::Error>
    where
        F: FnMut(M) -> Option<Entity>,
    {
        Ok(func(data).unwrap())
    }
}
