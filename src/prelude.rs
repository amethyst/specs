//! Prelude module
//!
//! Contains all of the most common traits, structures,

pub use hibitset::BitSet;
pub use join::{Join, ParJoin};
pub use shred::{
    Accessor, Dispatcher, DispatcherBuilder, Read, ReadExpect, Resources, RunNow, StaticAccessor,
    System, SystemData, Write, WriteExpect,
};
pub use shrev::ReaderId;

#[cfg(not(target_os = "emscripten"))]
pub use rayon::iter::ParallelIterator;
#[cfg(not(target_os = "emscripten"))]
pub use shred::AsyncDispatcher;

pub use changeset::ChangeSet;
pub use storage::{
    DenseVecStorage, FlaggedStorage, HashMapStorage, NullStorage, ReadStorage, Storage, Tracked,
    VecStorage, WriteStorage, ComponentEvent,
};
pub use world::{Builder, Component, Entities, Entity, EntityBuilder, LazyUpdate, World};
