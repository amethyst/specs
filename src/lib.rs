#![deny(missing_docs)]

//! # SPECS Parallel ECS
//!
//! This library provides an ECS variant designed for parallel execution
//! and convenient usage. It is highly flexible when it comes to actual
//! component data and the way it is stored and accessed.

extern crate atom;
extern crate fnv;
#[macro_use]
extern crate mopa;
extern crate ticketed_lock;
extern crate tuple_utils;

#[cfg(feature="parallel")]
extern crate pulse;
#[cfg(feature="parallel")]
extern crate rayon;

pub use gate::Gate;
pub use join::{Join, JoinIter};
pub use storage::{AntiStorage, BTreeStorage, GatedStorage, HashMapStorage, InsertResult,
                  MaskedStorage, NullStorage, Storage, UnprotectedStorage, VecStorage,
                  DenseVecStorage};
pub use world::{Allocator, Component, CreateEntities, Entities, World};

#[cfg(feature="parallel")]
pub use planner::{Planner, Priority, RunArg, System, SystemInfo, ExternalSystem};


#[doc(hidden)]
pub mod bitset;
mod gate;
mod join;
mod storage;
mod world;

#[cfg(feature="parallel")]
mod planner;


/// Index generation. When a new entity is placed at an old index,
/// it bumps the `Generation` by 1. This allows to avoid using components
/// from the entities that were deleted.
#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Generation(i32);

impl Generation {
    /// Returns `true` if entities of this `Generation` are alive.
    pub fn is_alive(&self) -> bool {
        self.0 > 0
    }

    /// Kills this `Generation`.
    fn die(&mut self) {
        debug_assert!(self.is_alive());
        self.0 = -self.0;
    }

    /// Revives and increments a dead `Generation`.
    fn raised(self) -> Generation {
        debug_assert!(!self.is_alive());
        Generation(1 - self.0)
    }
}

/// `Index` type is arbitrary. It doesn't show up in any interfaces.
/// Keeping it 32bit allows for a single 64bit word per entity.
pub type Index = u32;

/// `Entity` type, as seen by the user.
#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Entity(Index, Generation);

impl Entity {
    #[cfg(test)]
    /// Creates a new entity (externally from ECS).
    pub fn new(index: Index, gen: Generation) -> Entity {
        Entity(index, gen)
    }

    /// Returns the index of the `Entity`.
    #[inline]
    pub fn get_id(&self) -> Index {
        self.0
    }

    /// Returns the `Generation` of the `Entity`.
    #[inline]
    pub fn get_gen(&self) -> Generation {
        self.1
    }
}
