//! Prelude module
//!
//! Contains all of the most common traits, structures,

pub use crate::join::Join;
#[cfg(feature = "parallel")]
pub use crate::join::ParJoin;
pub use hibitset::BitSet;
pub use shred::{
    Accessor, Dispatcher, DispatcherBuilder, Read, ReadExpect, Resource, ResourceId, RunNow,
    StaticAccessor, System, SystemData, World, Write, WriteExpect,
};
pub use shrev::ReaderId;

#[cfg(feature = "parallel")]
pub use rayon::iter::ParallelIterator;
#[cfg(feature = "parallel")]
pub use shred::AsyncDispatcher;

pub use crate::{
    changeset::ChangeSet,
    storage::{
        ComponentEvent, DefaultVecStorage, DenseVecStorage, FlaggedStorage, HashMapStorage,
        NullStorage, ReadStorage, Storage, Tracked, VecStorage, WriteStorage,
    },
    world::{Builder, Component, Entities, Entity, EntityBuilder, LazyUpdate, WorldExt},
};
