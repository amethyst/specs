#![deny(missing_docs)]

//! # SPECS Parallel ECS
//!
//! This library provides an ECS variant designed for parallel execution
//! and convenient usage. It is highly flexible when it comes to actual
//! component data and the way it is stored and accessed.

extern crate atom;
extern crate fnv;
extern crate hibitset;
extern crate mopa;
extern crate shred;
extern crate tuple_utils;

#[cfg(feature="serialize")]
extern crate serde;
#[cfg(feature="serialize")]
#[macro_use]
extern crate serde_derive;

pub use shred::{AsyncDispatcher, Dispatcher, DispatcherBuilder, Resource, System};

pub use entity::{Component, Entity, Entities};
pub use join::{Join, JoinIter};
pub use world::World;
pub use storage::{CheckStorage, InsertResult, UnprotectedStorage};

#[cfg(feature = "serialize")]
pub use storage::{MergeError, PackedData};

/// Reexports for types implementing `SystemData`.
///
/// # Examples
///
/// These can be used in a `System` implementation
///
/// ```
/// # use specs::Resource;
/// use specs::prelude::*;
///
/// # #[derive(Debug)] struct MyComp;
/// # impl Component for MyComp { type Storage = VecStorage<MyComp>; }
/// # #[derive(Debug)] struct MyRes; impl Resource for MyRes {}
///
/// struct MySys;
///
/// impl<'a, C> System<'a, C> for MySys {
///     type SystemData = (Entities<'a>, FetchMut<'a, MyRes>, WriteStorage<'a, MyComp>);
///
///     fn work(&mut self, data: Self::SystemData, _: C) {
///         // ..
///
///         # let _ = data;
///     }
/// }
/// ```
pub mod data {
    pub use shred::{Fetch, FetchId, FetchIdMut, FetchMut, SystemData};

    pub use storage::{ReadStorage, Storage, WriteStorage};

    /// A wrapper for a fetched `Entities` resource.
    /// Note that this is just `Fetch<Entities>`, so
    /// you can easily use it in your system:
    ///
    /// ```ignore
    /// type SystemData = (Entities<'a>, ...);
    /// ```
    pub type Entities<'a> = Fetch<'a, ::entity::Entities>;
}

/// Entity related types.
pub mod entity {
    pub use world::{Component, CreateIter, CreateIterAtomic, Entity, Entities, EntityBuilder,
                    Generation};
}

/// Reexports for very common types.
pub mod prelude {
    pub use {Component, Dispatcher, DispatcherBuilder, Entity, Resource, System, World};

    pub use data::{Entities, Fetch, FetchMut, ReadStorage, WriteStorage};
    pub use join::Join;
    pub use storages::{DenseVecStorage, HashMapStorage, VecStorage};
}

pub use storage::storages;

mod join;
mod storage;
mod world;

/// `Index` type is arbitrary. It doesn't show up in any interfaces.
/// Keeping it 32bit allows for a single 64bit word per entity.
pub type Index = u32;
