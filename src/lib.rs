#![deny(missing_docs)]

//! # SPECS Parallel ECS
//!
//! This library provides an ECS variant designed for parallel execution
//! and convenient usage. It is highly flexible when it comes to actual
//! component data and the way it is stored and accessed.
//!
//! Features:
//!
//! * depending on chosen features either 0 virtual function calls or one per system
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
//! [`World`]: struct.World.html
//!
//! [`Component`]s can be easily implemented like this:
//!
//! [`Component`]: trait.Component.html
//!
//! ```rust
//! use specs::{Component, VecStorage};
//!
//! struct MyComp;
//!
//! impl Component for MyComp {
//!     type Storage = VecStorage<Self>;
//! }
//! ```
//!
//! Or alternatively, if you import the `specs-derive` crate, you can use a
//! custom `#[derive]` macro:
//!
//! ```rust,ignore
//! #[derive(Component)]
//! #[component(VecStorage)]
//! struct MyComp;
//! ```
//!
//! You can choose different storages according to your needs.
//!
//! These storages can be [`join`]ed together, for example joining a `Velocity`
//! and a `Position` storage means you'll only get entities which have both of them.
//! Thanks to rayon, this is even possible in parallel! See [`ParJoin`] for more.
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
//! use specs::{Component, DispatcherBuilder, Join, ReadStorage, System, VecStorage,
//!             WriteStorage, World};
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
//!     let mut dispatcher = DispatcherBuilder::new().add(SysA, "sys_a", &[]).build();
//!
//!     // This dispatches all the systems in parallel (but blocking).
//!     dispatcher.dispatch(&mut world.res);
//! }
//! ```
//!
//! You can also easily create new entities on the fly:
//!
//! ```
//! use specs::{Entities, FetchMut, System, WriteStorage};
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
//!
//!
//!

extern crate crossbeam;
#[macro_use]
extern crate derivative;
extern crate fnv;
extern crate hibitset;
extern crate mopa;
extern crate rayon;
extern crate shred;
extern crate tuple_utils;

#[cfg(feature = "common")]
extern crate futures;
#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;

#[cfg(feature = "rudy")]
extern crate rudy;

pub use join::{Join, JoinIter, JoinParIter, ParJoin};
pub use shred::{Dispatcher, DispatcherBuilder, Fetch, FetchId, FetchIdMut, FetchMut, RunNow,
                RunningTime, System, SystemData};

#[cfg(not(target_os = "emscripten"))]
pub use shred::AsyncDispatcher;

pub use storage::{BTreeStorage, DenseVecStorage, DistinctStorage, Entry, FlaggedStorage,
                  HashMapStorage, InsertResult, MaskedStorage, NormalRestriction, NullStorage,
                  ParallelRestriction, ReadStorage, RestrictedStorage, Storage,
                  UnprotectedStorage, VecStorage, WriteStorage};
pub use world::{Component, CreateIter, CreateIterAtomic, EntitiesRes, Entity, EntityBuilder,
                Generation, LazyUpdate, World};

#[cfg(feature = "common")]
pub mod common;

#[cfg(feature = "rudy")]
pub use storage::RudyStorage;

#[cfg(feature = "serde")]
pub use storage::{MergeError, PackedData};

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
pub type Entities<'a> = Fetch<'a, EntitiesRes>;

/// An index is basically the id of an `Entity`.
pub type Index = u32;

pub mod error;

mod bitset;
mod join;
mod storage;
mod world;
