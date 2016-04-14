#![deny(missing_docs)]

//! SPECS Parallel ECS
//! This library provides an ECS variant designed for parallel execution
//! and convenient usage. It is highly flexible when it comes to actual
//! component data and the way it's stored and accessed.

#[macro_use]
extern crate mopa;
extern crate pulse;
extern crate threadpool;
extern crate fnv;
extern crate tuple_utils;

use std::cell::RefCell;
use std::sync::Arc;
use pulse::{Pulse, Signal, Barrier, Signals};
use threadpool::ThreadPool;

pub use storage::{Storage, StorageBase, VecStorage, HashMapStorage, UnprotectedStorage};
pub use world::{Component, World, FetchArg,
    EntityBuilder, EntityIter, CreateEntityIter, DynamicEntityIter};
pub use bitset::{BitSetAnd, BitSet, BitSetLike};
use join::Join;

mod storage;
mod world;
mod bitset;
mod join;

/// Index generation. When a new entity is placed at the old index,
/// it bumps the generation by 1. This allows to avoid using components
/// from the entities that were deleted.
#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Generation(i32);

impl Generation {
    /// Returns `true` if entities of this `Generation` are alive
    pub fn is_alive(&self) -> bool {
        self.0 > 0
    }

    /// Kills this `Generation`
    fn die(&mut self) {
        debug_assert!(self.is_alive());
        self.0 = -self.0;
    }

    /// Revive and increment a dead `Generation`
    fn raised(self) -> Generation {
        debug_assert!(!self.is_alive());
        Generation(1 - self.0)
    }

    /// Returns `true` if this is a first generation, i.e. has value `1`
    fn is_first(&self) -> bool {
        self.0 == 1
    }
}

/// Index type is arbitrary. It doesn't show up in any interfaces.
/// Keeping it 32bit allows for a single 64bit word per entity.
pub type Index = u32;
/// Entity type, as seen by the user.
#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Entity(Index, Generation);

impl Entity {
    #[cfg(test)]
    /// Create a new entity (externally from ECS)
    pub fn new(index: Index, gen: Generation) -> Entity {
        Entity(index, gen)
    }

    /// Get the index of the entity.
    #[inline]
    pub fn get_id(&self) -> usize { self.0 as usize }
    /// Get the generation of the entity.
    #[inline]
    pub fn get_gen(&self) -> Generation { self.1 }
}


/// System closure run-time argument.
pub struct RunArg {
    world: Arc<World>,
    pulse: RefCell<Option<Pulse>>,
}

impl RunArg {
    /// Borrows the world, allowing the system lock some components and get the entity
    /// iterator. Has to be called only once. Fires a pulse at the end.
    pub fn fetch<'a, U, F>(&'a self, f: F) -> U
        where F: FnOnce(FetchArg<'a>) -> U
    {
        let pulse = self.pulse.borrow_mut().take()
                        .expect("fetch may only be called once.");
        let u = f(FetchArg::new(&self.world));
        pulse.pulse();
        u
    }
    /// Create a new entity dynamically.
    pub fn create(&self) -> Entity {
        self.world.create_later()
    }
    /// Delete an entity dynamically.
    pub fn delete(&self, entity: Entity) {
        self.world.delete_later(entity)
    }
    /// Iterate dynamically added entities.
    pub fn new_entities(&self) -> DynamicEntityIter {
        self.world.dynamic_entities()
    }
}


/// System execution planner. Allows running systems via closures,
/// distributes the load in parallel using a thread pool.
pub struct Planner {
    /// Shared World.
    pub world: Arc<World>,
    threads: ThreadPool,
    pending: Vec<Signal>
}

impl Planner {
    /// Create a new planner, given the world and the thread count.
    pub fn new(world: World, num_threads: usize) -> Planner {
        Planner {
            world: Arc::new(world),
            threads: ThreadPool::new(num_threads),
            pending: vec![]
        }
    }
    /// Run a custom system.
    pub fn run<F>(&mut self, functor: F) where
        F: 'static + Send + FnOnce(RunArg)
    {
        let (signal, pulse) = Signal::new();
        let (signal_done, pulse_done) = Signal::new();
        let world = self.world.clone();
        self.threads.execute(|| {
            functor(RunArg {
                world: world,
                pulse: RefCell::new(Some(pulse)),
            });
            pulse_done.pulse();
        });
        if signal.wait().is_err() {
            panic!("task panicked before args were captured.")
        }
        self.pending.push(signal_done);
    }
    /// Wait for all the currently executed systems to finish.
    pub fn wait(&mut self) {
        Barrier::new(&self.pending[..]).wait().unwrap();
        for signal in self.pending.drain(..) {
            if signal.wait().is_err() {
                panic!("one or more task as panicked.")
            }
        }
        self.pending.clear();

        self.world.merge();
    }
}

macro_rules! impl_run {
    ($name:ident [$( $write:ident ),*] [$( $read:ident ),*]) => (impl Planner {
        #[allow(missing_docs, non_snake_case, unused_mut)]
        pub fn $name<'a,
            $($write,)* $($read,)*
            F: 'static + Send + FnMut( $(&mut $write,)* $(&$read,)* )
        >(&mut self, functor: F)
            where $($write:Component,)*
                  $($read:Component,)*
        {
            self.run(|run| {
                let mut fun = functor;
                let ($(mut $write,)* $($read,)*) = run.fetch(|w|
                    ($(w.write::<$write>(),)*
                     $(w.read::<$read>(),)*)
                );

                $(let mut $write = $write.open_mut();)*
                $(let $read = $read.open();)*

                let selector = ($($write.0,)* $($read.0,)*).join();

                for ent in selector.iter() {
                    let ($($write,)* $($read,)*) = unsafe {
                        ($($write.1.get_mut(ent),)* $($read.1.get(ent),)*)
                    };
                    fun( $($write,)* $($read,)* );
                }
            });
        }
    })
}

impl_run!( run0w1r [] [R0] );
impl_run!( run0w2r [] [R0, R1] );
impl_run!( run1w0r [W0] [] );
impl_run!( run1w1r [W0] [R0] );
impl_run!( run1w2r [W0] [R0, R1] );
impl_run!( run1w3r [W0] [R0, R1, R2] );
impl_run!( run1w4r [W0] [R0, R1, R2, R3] );
impl_run!( run1w5r [W0] [R0, R1, R2, R3, R4] );
impl_run!( run1w6r [W0] [R0, R1, R2, R3, R4, R5] );
impl_run!( run1w7r [W0] [R0, R1, R2, R3, R5, R6, R7] );
impl_run!( run2w0r [W0, W1] [] );
impl_run!( run2w1r [W0, W1] [R0] );
impl_run!( run2w2r [W0, W1] [R0, R1] );
