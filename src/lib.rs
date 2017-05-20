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

pub use join::{Join, JoinIter};
pub use world::World;
pub use storage::{CheckStorage, InsertResult, ReadStorage, Storage, UnprotectedStorage,
                  WriteStorage};

#[cfg(feature = "serialize")]
pub use storage::{MergeError, PackedData};

/// Entity related types.
pub mod entity {
    pub use world::{Component, CreateIter, Entity, Entities, EntityBuilder, Generation};
}

/// Different types of storages you can use for your components.
pub mod storages {
    pub use storage::storages::{BTreeStorage, DenseVecStorage, HashMapStorage, NullStorage,
                                VecStorage};
}

mod join;
mod storage;
mod world;

#[cfg(feature="parallel")]
mod planner;

/// `Index` type is arbitrary. It doesn't show up in any interfaces.
/// Keeping it 32bit allows for a single 64bit word per entity.
pub type Index = u32;
