//! Prelude module
//!
//! Contains all of the most common traits, structures,

pub use hibitset::BitSet;
pub use join::{Join, ParJoin};
pub use shred::{Dispatcher, DispatcherBuilder, Fetch, FetchId, FetchIdMut, FetchMut, RunNow,
                RunningTime, System, SystemData};
pub use shrev::ReaderId;

#[cfg(not(target_os = "emscripten"))]
pub use shred::AsyncDispatcher;

pub use storage::{DenseVecStorage, FlaggedStorage, HashMapStorage, InsertedFlag, ModifiedFlag,
                  NullStorage, ReadStorage, RemovedFlag, Storage, Tracked, VecStorage,
                  WriteStorage};
pub use world::{Component, Entities, EntitiesRes, Entity, EntityBuilder, Generation, LazyUpdate,
                World};
