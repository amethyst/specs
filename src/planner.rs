use std::cell::RefCell;
use std::sync::{mpsc, Arc};

use pulse::{Pulse, Signal, Signals};
use threadpool::ThreadPool;

use super::{Component, JoinIter, World, Entity};

/// System closure run-time argument.
pub struct RunArg {
    world: Arc<World>,
    pulse: RefCell<Option<Pulse>>,
}

impl RunArg {
    /// Borrows the world, allowing the system to lock some components and get the entity
    /// iterator. Must be called only once.
    pub fn fetch<'a, U, F>(&'a self, f: F) -> U
        where F: FnOnce(&'a World) -> U
    {
        let pulse = self.pulse.borrow_mut().take()
                        .expect("fetch may only be called once.");
        let u = f(&self.world);
        pulse.pulse();
        u
    }
    /// Borrows the world, allowing the system to lock some components and get the entity
    /// iterator. As an alternative to `fetch()`, it must be called only once.
    /// It allows creating a number of entities instantly, returned in a vector.
    #[allow(mutable_transmutes)]
    pub fn fetch_new<'a, U, F>(&'a self, num_entities: usize, f: F) -> (Vec<Entity>, U)
        where F: FnOnce(&'a World) -> U
    {
        use std::mem::transmute;
        // The transmute is used to call `create_iter`, which is really safe for parallel use.
        // It's only receiving `&mut self` to prevent deadlocks, and these are not possible in
        // the pre-fetch phase we are in right now.
        let entities = unsafe { transmute::<&World, &mut World>(&self.world) }
            .create_iter().take(num_entities).collect();
        (entities, self.fetch(f))
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

/// System scheduling priority. Higehr priority systems are started
/// earlier than lower-priority ones.
pub type Priority = i32;

/// System information package, where the system itself is accompanied
/// by its name and priority.
pub struct SystemInfo<C> {
    /// Name of the system. Can be used for lookups or debug output.
    pub name: String,
    /// Priority of the system.
    pub priority: Priority,
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
    world: Arc<World>,
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
    /// Add a system to the dispatched list.
    pub fn add_system<S>(&mut self, sys: S, name: &str, priority: Priority) where
        S: 'static + System<C>
    {
        self.systems.push(SystemInfo {
            name: name.to_owned(),
            priority: priority,
            object: Box::new(sys),
        });
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

    fn wait_internal(&mut self) {
        while self.wait_count > 0 {
            let sinfo = self.chan_in.recv().expect("one or more task as panicked.");
            if !sinfo.name.is_empty() {
                self.systems.push(sinfo);
            }
            self.wait_count -= 1;
        }
    }

    /// Waits for all currently executing systems to finish, and then
    /// returns the mutable borrow of the world, allowing to create
    /// entities instantly.
    pub fn mut_world(&mut self) -> &mut World {
        self.wait_internal();
        Arc::get_mut(&mut self.world).unwrap()
    }

    /// Waits for all currently executing systems to finish, and then
    /// merges all queued changes.
    pub fn wait(&mut self) {
        self.mut_world().maintain();
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

                for ($($write,)* $($read,)*) in JoinIter::new(($(&mut $write,)* $(&$read,)*)) {
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
