//! Prelude module
//!
//! Contains all of the most common traits, structures,

pub use hibitset::BitSet;
pub use join::Join;
#[cfg(feature = "parallel")]
pub use join::ParJoin;
pub use shred::{Dispatcher, DispatcherBuilder, Fetch, FetchMut, RunNow, System, SystemData};
pub use shrev::ReaderId;

#[cfg(feature = "parallel")]
pub use shred::AsyncDispatcher;

pub use changeset::ChangeSet;
pub use storage::{DenseVecStorage, FlaggedStorage, InsertedFlag, ModifiedFlag, ReadStorage,
                  RemovedFlag, Storage, Tracked, VecStorage, WriteStorage};
pub use world::{Component, Entities, Entity, EntityBuilder, LazyUpdate, World};
