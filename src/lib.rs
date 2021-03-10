#![warn(missing_docs)]
#![cfg_attr(feature = "nightly", feature(generic_associated_types, associated_type_defaults))]

//! # SPECS Parallel ECS
//!
//! This library provides an ECS variant designed for parallel execution
//! and convenient usage. It is highly flexible when it comes to actual
//! component data and the way it is stored and accessed.
//!
//! Features:
//!
//! * depending on chosen features either 0 virtual function calls or one per
//!   system
//! * parallel iteration over components
//! * parallel execution of systems
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
//! [`World`]: world/struct.World.html
//!
//! [`Component`]s can be easily implemented like this:
//!
//! [`Component`]: trait.Component.html
//!
//! ```rust
//! use specs::prelude::*;
//!
//! struct MyComp;
//!
//! impl Component for MyComp {
//!     type Storage = VecStorage<Self>;
//! }
//! ```
//!
//! Or alternatively, if you enable the `"derive"` feature, you can use a
//! custom `#[derive]` macro:
//!
//! ```rust
//! # extern crate specs;
//! # extern crate specs_derive;
//! # #[cfg(not(feature = "specs-derive"))] use specs_derive::Component;
//! # #[cfg(not(feature = "specs-derive"))] use specs::prelude::*;
//!
//! # #[cfg(feature = "specs-derive")]
//! use specs::{prelude::*, Component};
//!
//! #[derive(Component)]
//! #[storage(VecStorage)] // default is `DenseVecStorage`
//! struct MyComp;
//! # fn main() {}
//! ```
//!
//! You can choose different storages according to your needs.
//!
//! These storages can be [`join`]ed together, for example joining a `Velocity`
//! and a `Position` storage means you'll only get entities which have both of
//! them. Thanks to rayon, this is even possible in parallel! See [`ParJoin`]
//! for more.
//!
//! [`join`]: trait.Join.html#method.join
//! [`ParJoin`]: trait.ParJoin.html
//!
//! ### System execution
//!
//! Here we have [`System`] and [`Dispatcher`] as our core types. Both types
//! are provided by a library called `shred`.
//!
//! [`Dispatcher`]: struct.Dispatcher.html
//! [`System`]: trait.System.html
//!
//! The `Dispatcher` can be seen as an optional part here;
//! it allows dispatching the systems in parallel, given a list
//! of systems and their dependencies on other systems.
//!
//! If you don't like it, you can also execute the systems yourself
//! by using [`RunNow`].
//!
//! [`RunNow`]: trait.RunNow.html
//!
//! `System`s are traits with a `run()` method and an associated
//! [`SystemData`], allowing type-safe aspects (knowledge about the
//! reads / writes of the systems).
//!
//! [`SystemData`]: trait.SystemData.html
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
//! // A component contains data which is
//! // associated with an entity.
//!
//! struct Vel(f32);
//!
//! impl Component for Vel {
//!     type Storage = VecStorage<Self>;
//! }
//!
//! struct Pos(f32);
//!
//! impl Component for Pos {
//!     type Storage = VecStorage<Self>;
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
//!     fn run(&mut self, (mut pos, vel): Self::SystemData) {
//!         // The `.join()` combines multiple components,
//!         // so we only access those entities which have
//!         // both of them.
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
//!     let mut dispatcher = DispatcherBuilder::new().with(SysA, "sys_a", &[]).build();
//!
//!     // This dispatches all the systems in parallel (but blocking).
//!     dispatcher.dispatch(&mut world);
//! }
//! ```
//!
//! You can also easily create new entities on the fly:
//!
//! ```
//! use specs::prelude::*;
//!
//! struct EnemySpawner;
//!
//! impl<'a> System<'a> for EnemySpawner {
//!     type SystemData = Entities<'a>;
//!
//!     fn run(&mut self, entities: Entities<'a>) {
//!         let enemy = entities.create();
//!     }
//! }
//! ```
//!
//! See the repository's examples directory for more examples.

pub extern crate hibitset;
#[cfg(feature = "parallel")]
pub extern crate rayon;
pub extern crate shred;
pub extern crate shrev;
#[cfg(feature = "uuid_entity")]
pub extern crate uuid;

#[cfg(feature = "serde")]
pub mod saveload;

mod bitset;
pub mod changeset;
pub mod error;
pub mod join;
pub mod prelude;
pub mod storage;
pub mod world;

pub use hibitset::BitSet;
pub use shred::{
    Accessor, AccessorCow, BatchAccessor, BatchController, BatchUncheckedWorld,
    Dispatcher, DispatcherBuilder, Read, ReadExpect, RunNow,
    RunningTime, StaticAccessor, System, SystemData, World, Write, WriteExpect,
};
pub use shrev::ReaderId;

#[cfg(feature = "parallel")]
pub use shred::AsyncDispatcher;

#[cfg(feature = "specs-derive")]
pub use specs_derive::{Component, ConvertSaveload};

#[cfg(feature = "parallel")]
pub use crate::join::ParJoin;
pub use crate::{
    changeset::ChangeSet,
    join::Join,
    storage::{
        DefaultVecStorage, DenseVecStorage, FlaggedStorage, HashMapStorage, NullStorage,
        ReadStorage, Storage, Tracked, VecStorage, WriteStorage,
    },
    world::{Builder, Component, Entities, Entity, EntityBuilder, LazyUpdate, WorldExt},
};

#[cfg(feature = "nightly")]
pub use crate::storage::DerefFlaggedStorage;