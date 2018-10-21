//! Prelude module
//!
//! Contains all of the most common traits, structures,

pub use hibitset::BitSet;
pub use join::Join;
#[cfg(feature = "parallel")]
pub use join::ParJoin;
pub use shred::{Accessor, Dispatcher, DispatcherBuilder, Read, ReadExpect, Resources, RunNow,
                StaticAccessor, System, SystemData, Write, WriteExpect};
pub use shrev::ReaderId;

#[cfg(feature = "parallel")]
pub use rayon::iter::ParallelIterator;
#[cfg(feature = "parallel")]
pub use shred::AsyncDispatcher;

pub use changeset::ChangeSet;
pub use storage::{
    DenseVecStorage, FlaggedStorage, HashMapStorage, NullStorage, ReadStorage, Storage, Tracked,
    VecStorage, WriteStorage, ComponentEvent,
};
pub use world::{Builder, Component, Entities, Entity, EntityBuilder, LazyUpdate, World};
