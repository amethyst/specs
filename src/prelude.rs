//! Prelude module
//!
//! Contains all of the most common traits, structures,

pub use hibitset::BitSet;
pub use join::{Join, ParJoin};
pub use shred::{Dispatcher, DispatcherBuilder, Read, ReadExpect, RunNow, System, SystemData,
                Write, WriteExpect};
pub use shrev::ReaderId;

#[cfg(not(target_os = "emscripten"))]
pub use shred::AsyncDispatcher;

pub use changeset::ChangeSet;
pub use storage::{DenseVecStorage, FlaggedStorage, InsertedFlag, ModifiedFlag, ReadStorage,
                  RemovedFlag, Storage, Tracked, VecStorage, WriteStorage};
pub use world::{Component, Entities, Entity, EntityBuilder, LazyUpdate, World};
