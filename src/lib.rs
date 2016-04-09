#[macro_use]
extern crate mopa;
extern crate pulse;
extern crate threadpool;

use std::any::TypeId;
use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};
use mopa::Any;
use pulse::{Pulse, Signal};
use threadpool::ThreadPool;

pub use storage::{Storage, StorageBase, VecStorage, HashMapStorage};

mod storage;

/// Index generation. When a new entity is placed at the old index,
/// it bumps the generation by 1. This allows to avoid using components
/// from the entities that were deleted.
/// G<=0 - the entity of generation G is dead
/// G >0 - the entity of generation G is alive
pub type Generation = i32;
/// Index type is arbitrary. It doesn't show up in any interfaces.
/// Keeping it 32bit allows for a single 64bit word per entity.
pub type Index = u32;
/// Entity type, as seen by the user.
#[derive(Clone, Copy, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Entity(Index, Generation);

impl Entity {
    pub fn get_id(&self) -> usize { self.0 as usize }
    pub fn get_gen(&self) -> Generation { self.1 }
}

pub struct EntityIter<'a> {
    guard: RwLockReadGuard<'a, Vec<Generation>>,
    index: usize,
}

impl<'a> Iterator for EntityIter<'a> {
    type Item = Entity;
    fn next(&mut self) -> Option<Entity> {
        loop {
            match self.guard.get(self.index) {
                Some(&gen) if gen > 0 => {
                    let ent = Entity(self.index as Index, gen);
                    self.index += 1;
                    return Some(ent)
                },
                Some(_) => self.index += 1, // continue
                None => return None,
            }
        }
    }
}

pub trait Component: Any + Sized {
    type Storage: Storage<Self> + Any + Send + Sync;
}

trait StorageLock: Any + Send + Sync {
    fn del_slice(&self, &[Entity]);
}

mopafy!(StorageLock);

impl<S: StorageBase + Any + Send + Sync> StorageLock for RwLock<S> {
    fn del_slice(&self, entities: &[Entity]) {
        let mut guard = self.write().unwrap();
        for &e in entities.iter() {
            guard.del(e);
        }
    }
}


pub struct World {
    generations: RwLock<Vec<Generation>>,
    components: HashMap<TypeId, Box<StorageLock>>,
}


impl World {
    pub fn new() -> World {
        World {
            generations: RwLock::new(Vec::new()),
            components: HashMap::new(),
        }
    }
    pub fn register<T: Component>(&mut self) {
        let any = RwLock::new(T::Storage::new());
        self.components.insert(TypeId::of::<T>(), Box::new(any));
    }
    fn lock<T: Component>(&self) -> &RwLock<T::Storage> {
        let boxed = self.components.get(&TypeId::of::<T>()).unwrap();
        boxed.downcast_ref().unwrap()
    }
    pub fn read<'a, T: Component>(&'a self) -> RwLockReadGuard<'a, T::Storage> {
        self.lock::<T>().read().unwrap()
    }
    pub fn write<'a, T: Component>(&'a self) -> RwLockWriteGuard<'a, T::Storage> {
        self.lock::<T>().write().unwrap()
    }
    pub fn entities<'a>(&'a self) -> EntityIter<'a> {
        EntityIter {
            guard: self.generations.read().unwrap(),
            index: 0,
        }
    }
}



pub struct WorldArg(Arc<World>, RefCell<Option<Pulse>>);

impl WorldArg {
    pub fn fetch<'a, U, F>(&'a self, f: F) -> U
        where F: FnOnce(&'a World) -> U
    {
        let pulse = self.1.borrow_mut().take().expect("fetch may only be called once.");
        let u = f(&self.0);
        pulse.pulse();
        u
    }
}

pub struct EntityBuilder<'a>(Entity, &'a World);

impl<'a> EntityBuilder<'a> {
    pub fn with<T: Component>(self, value: T) -> EntityBuilder<'a> {
        self.1.write::<T>().add(self.0, value);
        self
    }
    pub fn build(self) -> Entity {
        self.0
    }
}


pub struct Scheduler {
    world: Arc<World>,
    threads: ThreadPool,
    first_free: Index,
}

fn find_free<'a>(mut gens: RwLockWriteGuard<'a, Vec<Generation>>, base: usize) -> Index {
    match gens[base..].iter().position(|g| *g <= 0) {
        Some(pos) => (base + pos) as Index,
        None => {
            gens.push(0);
            (gens.len() - 1) as Index
        },
    }
}

impl Scheduler {
    pub fn new(world: World, num_threads: usize) -> Scheduler {
        let ff = find_free(world.generations.write().unwrap(), 0);
        Scheduler {
            world: Arc::new(world),
            threads: ThreadPool::new(num_threads),
            first_free: ff,
        }
    }
    pub fn get_world(&self) -> &World {
        //println!("{:?}", &*self.world.generations.read().unwrap());
        &self.world
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
    pub fn add_entity<'a>(&'a mut self) -> EntityBuilder<'a> {
        let mut gens = self.world.generations.write().unwrap();
        let ent = {
            let gen = &mut gens[self.first_free as usize];
            assert!(*gen <= 0);
            *gen = 1 - *gen;
            Entity(self.first_free, *gen)
        };
        self.first_free = find_free(gens, (self.first_free + 1) as usize);
        EntityBuilder(ent, &self.world)
    }
    pub fn del_entity(&mut self, entity: Entity) {
        for boxed in self.world.components.values() {
            boxed.del_slice(&[entity]);
        }
        let mut gens = self.world.generations.write().unwrap();
        let mut gen = &mut gens[entity.get_id() as usize];
        assert!(*gen > 0);
        *gen *= -1;
    }
    pub fn wait(&self) {
        while self.threads.active_count() > 0 {} //TODO
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
                let ($(mut $write,)* $($read,)* entities) = warg.fetch(|w|
                    ($(w.write::<$write>(),)*
                     $(w.read::<$read>(),)*
                       w.entities())
                );
                for ent in entities {
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
