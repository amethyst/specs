#![deny(missing_docs)]

//! # SPECS Parallel ECS
//!
//! This library provides an ECS variant designed for parallel execution
//! and convenient usage. It is highly flexible when it comes to actual
//! component data and the way it is stored and accessed.
//!
//! ## High-level overview
//!
//! One could basically split this library up into two parts:
//! The data part and the execution part.
//!
//! ### The data
//!
//! `World` is where component storages, resources and entities are stored.
//! See the docs of [`World`] for more.
//!
//! [`World`]: ./world/struct.World.html
//!
//! Components can be easily implemented like this:
//!
//! ```rust
//! use specs::prelude::*;
//!
//! struct MyComp;
//!
//! impl Component for MyComp {
//!     type Storage = VecStorage<MyComp>;
//! }
//! ```
//!
//! You can choose different storages according to your needs.
//!
//! ### System execution
//!
//! One part of this is `System` and `Dispatcher`. Both types
//! are provided by a library called `shred`.
//!
//! The `Dispatcher` can be seen as an optional part here;
//! it allows dispatching the systems in parallel, given a list
//! of systems and their dependencies on other systems.
//!
//! `System`s are traits with a `run()` method and an associated
//! `SystemData`, allowing type-safe aspects (knowledge about the
//! reads / writes of the systems).
//!
//! To access components ergonomically, Specs provides a `Join` trait
//! which joins component storages together in an efficient manner.
//!
//! ## Examples
//!
//! This is a basic example of using Specs:
//!
//! ```rust
//! extern crate specs;
//!
//! use specs::prelude::*;
//!
//! // A component contains data
//! // which is associated with an entity.
//! struct Vel(f32);
//! struct Pos(f32);
//!
//! impl Component for Vel {
//!     type Storage = VecStorage<Vel>;
//! }
//!
//! impl Component for Pos {
//!     type Storage = VecStorage<Pos>;
//! }
//!
//! struct SysA;
//!
//! impl<'a> System<'a> for SysA {
//!     // These are the resources required for execution.
//!     // You can also define a struct and `#[derive(SystemData)]`,
//!     // see the `full` example.
//!     type SystemData = (WriteStorage<'a, Pos>, ReadStorage<'a, Vel>);
//!
//!     fn run(&mut self, data: Self::SystemData) {
//!         // The `.join()` combines multiple components,
//!         // so we only access those entities which have
//!         // both of them.
//!
//!         let (mut pos, vel) = data;
//!
//!         // This joins the component storages for Position
//!         // and Velocity together; it's also possible to do this
//!         // in parallel using rayon's `ParallelIterator`s.
//!         // See `ParJoin` for more.
//!         for (pos, vel) in (&mut pos, &vel).join() {
//!             pos.0 += vel.0;
//!         }
//!     }
//! }
//!
//! fn main() {
//!     // The `World` is our
//!     // container for components
//!     // and other resources.
//!
//!     let mut world = World::new();
//!     world.register::<Pos>();
//!     world.register::<Vel>();
//!
//!     // An entity may or may not contain some component.
//!
//!     world.create_entity().with(Vel(2.0)).with(Pos(0.0)).build();
//!     world.create_entity().with(Vel(4.0)).with(Pos(1.6)).build();
//!     world.create_entity().with(Vel(1.5)).with(Pos(5.4)).build();
//!
//!     // This entity does not have `Vel`, so it won't be dispatched.
//!     world.create_entity().with(Pos(2.0)).build();
//!
//!     // This builds a dispatcher.
//!     // The third parameter of `add` specifies
//!     // logical dependencies on other systems.
//!     // Since we only have one, we don't depend on anything.
//!     // See the `full` example for dependencies.
//!     let mut dispatcher = DispatcherBuilder::new().add(SysA, "sys_a", &[]).build();
//!
//!     // This dispatches all the systems in parallel (but blocking).
//!     dispatcher.dispatch(&mut world.res);
//! }
//! ```
//!
//! See the repository's examples directory for more examples.
//!

extern crate atom;
extern crate fnv;
extern crate hibitset;
extern crate mopa;
extern crate shred;
extern crate tuple_utils;
extern crate rayon;

#[cfg(feature="serialize")]
extern crate serde;
#[cfg(feature="serialize")]
#[macro_use]
extern crate serde_derive;

pub use shred::{AsyncDispatcher, Dispatcher, DispatcherBuilder, Resource, RunNow, RunningTime,
                System};

pub use entity::{Component, Entity, Entities};
pub use join::{Join, JoinIter, JoinParIter, ParJoin};
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
/// use specs::prelude::*;
///
/// # #[derive(Debug)] struct MyComp;
/// # impl Component for MyComp { type Storage = VecStorage<MyComp>; }
/// # #[derive(Debug)] struct MyRes;
///
/// struct MySys;
///
/// impl<'a> System<'a> for MySys {
///     type SystemData = (Entities<'a>, FetchMut<'a, MyRes>, WriteStorage<'a, MyComp>);
///
///     fn run(&mut self, data: Self::SystemData) {
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
    ///
    /// Please note that you should call `World::maintain`
    /// after creating / deleting entities with this resource.
    pub type Entities<'a> = Fetch<'a, ::entity::Entities>;
}

/// Entity related types.
pub mod entity {
    pub use world::{Component, CreateIter, CreateIterAtomic, Entity, Entities, EntityBuilder,
                    Generation};
}

/// Reexports for very common types.
pub mod prelude {
    pub use {Component, Dispatcher, DispatcherBuilder, Entity, Resource, RunningTime, System,
             World};

    pub use data::{Entities, Fetch, FetchMut, ReadStorage, WriteStorage};
    pub use join::{Join, ParJoin};
    pub use storages::{DenseVecStorage, HashMapStorage, VecStorage, FlaggedStorage};
}

pub use storage::storages;

mod join;
mod storage;
mod world;

/// An index is basically the id of an `Entity`.
pub type Index = u32;
