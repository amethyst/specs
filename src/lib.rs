extern crate pulse;
extern crate threadpool;

use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use pulse::{Pulse, Signal};
use threadpool::ThreadPool;

pub use storage::{Storage, VecStorage};

mod storage;


pub type Entity = u32;

pub trait Component: Any + Sized {
    type Storage: Storage<Self> + Any + Send + Sync;
}

pub struct World {
    entities: RwLock<Vec<Entity>>,
    components: HashMap<TypeId, Box<Any+Send+Sync>>,
}

impl World {
    pub fn new() -> World {
        World {
            entities: RwLock::new(Vec::new()),
            components: HashMap::new(),
        }
    }
    pub fn register<T: Component>(&mut self, storage: T::Storage) {
        let any = RwLock::new(storage);
        self.components.insert(TypeId::of::<T>(), Box::new(any));
    }
    fn lock<T: Component>(&self) -> &RwLock<T::Storage> {
        use std::ops::Deref;
        let boxed = self.components.get(&TypeId::of::<T>()).unwrap();
        (boxed.deref() as &Any).downcast_ref().unwrap()
    }
}

pub struct WorldArg(Arc<World>, RefCell<Option<Pulse>>);
impl WorldArg {
    pub fn read<'a, T: Component>(&'a self) -> RwLockReadGuard<'a, T::Storage> {
        assert!(self.1.borrow().is_some());
        self.0.lock::<T>().read().unwrap()
    }
    pub fn write<'a, T: Component>(&'a self) -> RwLockWriteGuard<'a, T::Storage> {
        assert!(self.1.borrow().is_some());
        self.0.lock::<T>().write().unwrap()
    }
    pub fn entities<'a>(&'a self) -> RwLockReadGuard<'a, Vec<Entity>> {
        self.1.borrow_mut().take().unwrap().pulse();
        self.0.entities.read().unwrap()
    }
}


pub struct Scheduler {
    world: Arc<World>,
    threads: ThreadPool,
}

impl Scheduler {
    pub fn new(num_threads: usize) -> Scheduler {
        Scheduler {
            world: Arc::new(World::new()),
            threads: ThreadPool::new(num_threads),
        }
    }
    pub fn run<F>(&mut self, functor: F) where
        F: 'static + Send + FnOnce(WorldArg)
    {
        let (signal, pulse) = Signal::new();
        let world = self.world.clone();
        self.threads.execute(|| {
            let warg = WorldArg(world, RefCell::new(Some(pulse)));
            functor(warg);
        });
        signal.wait().unwrap();
    }
}

macro_rules! impl_run {
    ($name:ident [$( $write:ident ),*] [$( $read:ident ),*]) => (impl Scheduler {
        #[allow(non_snake_case, unused_mut)]
        pub fn $name<
            $($write:Component,)* $($read:Component,)*
            F: 'static + Send + FnMut( $(&mut $write,)* $(&$read,)* )
        >(&mut self, functor: F) {
            self.run(|warg| {
                let mut fun = functor;
                $( let mut $write = warg.write::<$write>(); )*
                $( let $read = warg.read::<$read>(); )*
                for &ent in warg.entities().iter() {
                    if let ( $( Some($write), )* $( Some($read), )* ) =
                        ( $( $write.get_mut(ent), )* $( $read.get(ent), )* ) {
                        fun( $($write,)* $($read,)* );
                    }
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
