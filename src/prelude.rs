//! Prelude module
//!
//! Contains all of the most common traits, structures,

pub use hibitset::BitSet;
pub use crate::join::Join;
#[cfg(feature = "parallel")]
pub use crate::join::ParJoin;
pub use shred::{
    Accessor, Dispatcher, DispatcherBuilder, Read, ReadExpect, RunNow, StaticAccessor, System,
    SystemData, World, Write, WriteExpect,
};
pub use shrev::ReaderId;

#[cfg(feature = "parallel")]
pub use rayon::iter::ParallelIterator;
#[cfg(feature = "parallel")]
pub use shred::AsyncDispatcher;

pub use crate::changeset::ChangeSet;
pub use crate::storage::{
    ComponentEvent, DenseVecStorage, FlaggedStorage, HashMapStorage, NullStorage, ReadStorage,
    Storage, Tracked, VecStorage, WriteStorage,
};
pub use crate::world::{Builder, Component, Entities, Entity, EntityBuilder, LazyUpdate, WorldExt};
