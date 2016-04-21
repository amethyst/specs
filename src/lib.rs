#![deny(missing_docs)]

//! # SPECS Parallel ECS
//!
//! This library provides an ECS variant designed for parallel execution
//! and convenient usage. It is highly flexible when it comes to actual
//! component data and the way it is stored and accessed.

#[macro_use]
extern crate mopa;
extern crate pulse;
extern crate threadpool;
extern crate fnv;
extern crate tuple_utils;

use std::cell::RefCell;
use std::sync::{mpsc, Arc};
use pulse::{Pulse, Signal, Signals};
use threadpool::ThreadPool;

pub use storage::{Storage, VecStorage, HashMapStorage, UnprotectedStorage};
pub use world::{Component, World, FetchArg,
    EntityBuilder, Entities, CreateEntities};
pub use bitset::{BitSetAnd, BitSet, BitSetLike, AtomicBitSet};
//pub use join::Join; //TEMP

mod storage;
mod world;
mod bitset;
//mod join; TEMP

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
    pub fn get_id(&self) -> Index { self.0 }
    /// Returns the `Generation` of the `Entity`.
    #[inline]
    pub fn get_gen(&self) -> Generation { self.1 }
}


/// System closure run-time argument.
pub struct RunArg {
    world: Arc<World>,
    pulse: RefCell<Option<Pulse>>,
}

impl RunArg {
    /// Borrows the world, allowing the system to lock some components and get the entity
    /// iterator. Must be called only once.
    pub fn fetch<'a, U, F>(&'a self, f: F) -> U
        where F: FnOnce(FetchArg<'a>) -> U
    {
        let pulse = self.pulse.borrow_mut().take()
                        .expect("fetch may only be called once.");
        let u = f(FetchArg::new(&self.world));
        pulse.pulse();
        u
    }
    /// Creates a new entity dynamically.
    pub fn create(&self) -> Entity {
        self.world.create_later()
    }
    /// Deletes an entity dynamically.
    pub fn delete(&self, entity: Entity) {
        self.world.delete_later(entity)
    }
}

/// Generic system that runs through the entities and do something
/// with their components, with an ability to add new entities and
/// delete existing ones.
pub trait System<C>: Send {
    /// Run the system, given its context.
    fn run(&mut self, RunArg, C);
}

impl<C> System<C> for () {
    fn run(&mut self, _: RunArg, _: C) {}
}

/// System information package, where the system itself is accompanied
/// by its name and priority.
pub struct SystemInfo<C> {
    /// Name of the system. Can be used for lookups or debug output.
    pub name: String,
    /// Priority of the system. The higher priority systems are started
    /// before lower priority ones.
    pub priority: i32,
    /// System trait object itself.
    pub object: Box<System<C>>,
}

struct SystemGuard<C> {
    info: Option<SystemInfo<C>>,
    chan: mpsc::Sender<SystemInfo<C>>,
}

impl<C> Drop for SystemGuard<C> {
    fn drop(&mut self) {
        let info = self.info.take().unwrap_or_else(|| SystemInfo {
            name: String::new(),
            priority: 0,
            object: Box::new(()),
        });
        let _ = self.chan.send(info);
    }
}


/// System execution planner. Allows running systems via closures,
/// distributes the load in parallel using a thread pool.
pub struct Planner<C> {
    /// Shared `World`.
    pub world: Arc<World>,
    /// Permanent systems in the planner.
    pub systems: Vec<SystemInfo<C>>,
    wait_count: usize,
    chan_out: mpsc::Sender<SystemInfo<C>>,
    chan_in: mpsc::Receiver<SystemInfo<C>>,
    threader: ThreadPool,
}

impl<C: 'static> Planner<C> {
    /// Creates a new planner, given the world and the thread count.
    pub fn new(world: World, num_threads: usize) -> Planner<C> {
        let (sout, sin) = mpsc::channel();
        Planner {
            world: Arc::new(world),
            systems: Vec::new(),
            wait_count: 0,
            chan_out: sout,
            chan_in: sin,
            threader: ThreadPool::new(num_threads),
        }
    }
    /// Runs a custom system.
    pub fn run_custom<F>(&mut self, functor: F) where
        F: 'static + Send + FnOnce(RunArg)
    {
        let (signal, pulse) = Signal::new();
        let guard = SystemGuard {
            info: None,
            chan: self.chan_out.clone(),
        };
        let arg = RunArg {
            world: self.world.clone(),
            pulse: RefCell::new(Some(pulse)),
        };
        self.threader.execute(move || {
            let _ = guard; //for drop()
            functor(arg);
        });
        self.wait_count += 1;
        signal.wait().expect("task panicked before args were captured.");
    }
    /// Waits for all currently executing systems to finish, and then
    /// merges all queued changes.
    pub fn wait(&mut self) {
        while self.wait_count > 0 {
            let sinfo = self.chan_in.recv().expect("one or more task as panicked.");
            if !sinfo.name.is_empty() {
                self.systems.push(sinfo);
            }
            self.wait_count -= 1;
        }
        self.world.merge();
    }
}

impl<C: Clone + Send + 'static> Planner<C> {
    /// Dispatch all systems according to their associated priorities.
    pub fn dispatch(&mut self, context: C) {
        self.wait();
        self.systems.sort_by_key(|sinfo| -sinfo.priority);
        for sinfo in self.systems.drain(..) {
            assert!(!sinfo.name.is_empty());
            let ctx = context.clone();
            let (signal, pulse) = Signal::new();
            let guard = SystemGuard {
                info: Some(sinfo),
                chan: self.chan_out.clone(),
            };
            let arg = RunArg {
                world: self.world.clone(),
                pulse: RefCell::new(Some(pulse)),
            };
            self.threader.execute(move || {
                let mut g = guard;
                g.info.as_mut().unwrap().object.run(arg, ctx);
            });
            self.wait_count += 1;
            signal.wait().expect("task panicked before args were captured.");
        }
    }
}

macro_rules! impl_run {
    ($name:ident [$( $write:ident ),*] [$( $read:ident ),*]) => (impl<C: 'static> Planner<C> {
        #[allow(missing_docs, non_snake_case, unused_mut)]
        pub fn $name<$($write,)* $($read,)*
            F: 'static + Send + FnMut( $(&mut $write,)* $(&$read,)* )
        >(&mut self, functor: F)
            where $($write:Component,)*
                  $($read:Component,)*
        {
            self.run_custom(|run| {
                let mut fun = functor;
                let ($(mut $write,)* $($read,)*) = run.fetch(|w|
                    ($(w.write::<$write>(),)*
                     $(w.read::<$read>(),)*)
                );

                for ($($write,)* $($read,)*) in ($(&mut $write,)* $(&$read,)*).join() {
                    fun( $($write,)* $($read,)* );
                }
            });
        }
    })
}

impl_run!( run0w1r [] [R0] );
impl_run!( run0w2r [] [R0, R1] );
impl_run!( run0w3r [] [R0, R1, R2] );
impl_run!( run0w4r [] [R0, R1, R2, R3] );
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
