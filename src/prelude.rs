//! Prelude module
//!
//! Contains all of the most common traits, structures,

pub use hibitset::BitSet;
pub use join::{Join, ParJoin};
pub use shred::{Accessor, Dispatcher, DispatcherBuilder, Read, ReadExpect, Resources, RunNow,
                StaticAccessor, System, SystemData, Write, WriteExpect};
pub use shrev::ReaderId;

#[cfg(not(target_os = "emscripten"))]
pub use shred::AsyncDispatcher;
#[cfg(not(target_os = "emscripten"))]
pub use rayon::iter::ParallelIterator;

pub use changeset::ChangeSet;
pub use storage::{DenseVecStorage, FlaggedStorage, HashMapStorage, InsertedFlag, ModifiedFlag,
                  NullStorage, ReadStorage, RemovedFlag, Storage, Tracked, VecStorage, WriteStorage};
pub use world::{Component, Entities, Entity, EntityBuilder, LazyUpdate, World};
