//! Different types of storages you can use for your components.

use std::collections::BTreeMap;

use fnv::FnvHashMap;

use storage::UnprotectedStorage;
use Index;

/// HashMap-based storage. Best suited for rare components.
pub struct HashMapStorage<T>(FnvHashMap<Index, T>);

impl<T> UnprotectedStorage<T> for HashMapStorage<T> {
    fn new() -> Self {
        HashMapStorage(Default::default())
    }

    unsafe fn clean<F>(&mut self, _: F)
        where F: Fn(Index) -> bool
    {
        //nothing to do
    }

    unsafe fn get(&self, id: Index) -> &T {
        self.0.get(&id).unwrap()
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        self.0.get_mut(&id).unwrap()
    }

    unsafe fn insert(&mut self, id: Index, v: T) {
        self.0.insert(id, v);
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        self.0.remove(&id).unwrap()
    }
}

/// BTreeMap-based storage.
pub struct BTreeStorage<T>(BTreeMap<Index, T>);

impl<T> UnprotectedStorage<T> for BTreeStorage<T> {
    fn new() -> Self {
        BTreeStorage(Default::default())
    }

    unsafe fn clean<F>(&mut self, _: F)
        where F: Fn(Index) -> bool
    {
        // nothing to do
    }

    unsafe fn get(&self, id: Index) -> &T {
        self.0.get(&id).unwrap()
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        self.0.get_mut(&id).unwrap()
    }

    unsafe fn insert(&mut self, id: Index, v: T) {
        self.0.insert(id, v);
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        self.0.remove(&id).unwrap()
    }
}

/// Vector storage. Uses a simple `Vec`. Supposed to have maximum
/// performance for the components mostly present in entities.
pub struct VecStorage<T>(Vec<T>);

impl<T> UnprotectedStorage<T> for VecStorage<T> {
    fn new() -> Self {
        VecStorage(Vec::new())
    }

    unsafe fn clean<F>(&mut self, has: F)
        where F: Fn(Index) -> bool
    {
        use std::ptr;
        for (i, v) in self.0.iter_mut().enumerate() {
            if has(i as Index) {
                ptr::drop_in_place(v);
            }
        }
        self.0.set_len(0);
    }

    unsafe fn get(&self, id: Index) -> &T {
        self.0.get_unchecked(id as usize)
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        self.0.get_unchecked_mut(id as usize)
    }

    unsafe fn insert(&mut self, id: Index, v: T) {
        use std::ptr;

        let id = id as usize;
        if self.0.len() <= id {
            let delta = id + 1 - self.0.len();
            self.0.reserve(delta);
            self.0.set_len(id + 1);
        }
        // Write the value without reading or dropping
        // the (currently uninitialized) memory.
        ptr::write(self.0.get_unchecked_mut(id), v);
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        use std::ptr;

        ptr::read(self.get(id))
    }
}

/// Dense vector storage. Has a redirection 2-way table
/// between entities and components, allowing to leave
/// no gaps within the data.
pub struct DenseVecStorage<T> {
    data: Vec<T>,
    entity_id: Vec<Index>,
    data_id: Vec<Index>,
}

impl<T> UnprotectedStorage<T> for DenseVecStorage<T> {
    fn new() -> Self {
        DenseVecStorage {
            data: Vec::new(),
            entity_id: Vec::new(),
            data_id: Vec::new(),
        }
    }

    unsafe fn clean<F>(&mut self, _: F)
        where F: Fn(Index) -> bool
    {
        // nothing to do
    }

    unsafe fn get(&self, id: Index) -> &T {
        let did = *self.data_id.get_unchecked(id as usize);
        self.data.get_unchecked(did as usize)
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        let did = *self.data_id.get_unchecked(id as usize);
        self.data.get_unchecked_mut(did as usize)
    }

    unsafe fn insert(&mut self, id: Index, v: T) {
        let id = id as usize;
        if self.data_id.len() <= id {
            let delta = id + 1 - self.data_id.len();
            self.data_id.reserve(delta);
            self.data_id.set_len(id + 1);
        }
        *self.data_id.get_unchecked_mut(id) = self.data.len() as Index;
        self.entity_id.push(id as Index);
        self.data.push(v);
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        let did = *self.data_id.get_unchecked(id as usize);
        let last = *self.entity_id.last().unwrap();
        *self.data_id.get_unchecked_mut(last as usize) = did;
        self.entity_id.swap_remove(did as usize);
        self.data.swap_remove(did as usize)
    }
}

/// A null storage type, used for cases where the component
/// doesn't contain any data and instead works as a simple flag.
pub struct NullStorage<T>(T);

impl<T: Default> UnprotectedStorage<T> for NullStorage<T> {
    fn new() -> Self {
        NullStorage(Default::default())
    }
    unsafe fn clean<F>(&mut self, _: F) where F: Fn(Index) -> bool {}
    unsafe fn get(&self, _: Index) -> &T {
        &self.0
    }
    unsafe fn get_mut(&mut self, _: Index) -> &mut T {
        panic!("One does not simply modify a NullStorage")
    }
    unsafe fn insert(&mut self, _: Index, _: T) {}
    unsafe fn remove(&mut self, _: Index) -> T {
        Default::default()
    }
}

/// Wrapper storage that stores modifications to components in a bitset.
///
/// **Note: Never use `.iter()` on a mutable component storage that uses this.**
///
/// #Example Usage:
/// 
/// ```rust,ignore
///# extern crate specs;
///# use specs::{Planner, World, RunArg, Component, TrackedStorage, VecStorage, System, Join};
/// 
/// pub struct Comp(u32);
/// impl Component for Comp {
///     // `TrackedStorage` acts as a wrapper around another storage.
///     // You can put any store inside of here (e.g. HashMapStorage, VecStorage, etc.)
///     type Storage = TrackedStorage<Comp, VecStorage<Comp>>;
/// }
/// 
/// pub struct CompSystem;
/// impl System<()> for CompSystem {
///     fn run(&mut self, arg: RunArg, _: ()) {
///         let (entities, mut comps) = arg.fetch(|w| {
///             (w.entities(), w.write::<Comp>()) 
///         });
/// 
///         // Iterates over all components like normal.
///         for (entity, comp) in (&entities, &comps).iter() {
///             // ...
///         }
/// 
///         // **Never do this**
///         // This will flag all components as modified regardless of whether the inner loop
///         // did modify their data.
///         for (entity, comp) in (&entities, &mut comps).iter() {
///             // ...
///         }
/// 
///         // Instead do something like:
///         for (entity, comp) in (&entities, &comps.check()).iter() {
///             if true { // check whether you should modify this component or not.
///                 let mut comp = comps.get_mut(entity);
///                 // ...
///             }
///         }
/// 
///         // To iterate over the flagged/modified components:
///         for (entity, flagged_comp) in (&entities, (&comps).open().1).iter() {
///             // ...
///         }
/// 
///         // Clears the tracked storage every frame with this system.
///         (&mut comps).open().1.clear();
///     }
/// }
///# fn main() { }
/// ```
pub struct TrackedStorage<C: Component, T: UnprotectedStorage<C>> {
    mask: BitSet,
    storage: T,
    phantom: PhantomData<C>,
}

impl<C: Component, T: UnprotectedStorage<C>> UnprotectedStorage<C> for TrackedStorage<C, T> {
    fn new() -> Self {
        TrackedStorage {
            mask: BitSet::new(),
            storage: T::new(),
            phantom: PhantomData,
        }
    }
    unsafe fn clean<F>(&mut self, has: F) where F: Fn(Index) -> bool {
        self.mask.clear();
        self.storage.clean(has);
    }
    unsafe fn get(&self, id: Index) -> &C {
        self.storage.get(id)
    }
    unsafe fn get_mut(&mut self, id: Index) -> &mut C {
        // calling `.iter()` on a mutable reference to the storage will flag everything
        self.mask.add(id);
        self.storage.get_mut(id)
    }
    unsafe fn insert(&mut self, id: Index, comp: C) {
        self.mask.add(id);
        self.storage.insert(id, comp);
    }
    unsafe fn remove(&mut self, id: Index) -> C {
        self.mask.remove(id);
        self.storage.remove(id)
    }
}

impl<C: Component, T: UnprotectedStorage<C>> TrackedStorage<C, T> {
    /// All components will be cleared of being flagged.
    pub fn clear(&mut self) {
        self.mask.clear();
    }
    /// Flags a single component as not flagged.
    pub fn unflag(&mut self, entity: Entity) {
        self.mask.remove(entity.get_id());
    }
    /// Flags a single component as flagged.
    pub fn flag(&mut self, entity: Entity) {
        self.mask.add(entity.get_id());
    }
}

impl<'a, C: Component, T: UnprotectedStorage<C>> Join for &'a TrackedStorage<C, T> {
    type Type = &'a C;
    type Value = &'a T;
    type Mask = &'a BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        (&self.mask, &self.storage)
    }
    unsafe fn get(v: &mut Self::Value, id: Index) -> &'a C {
        v.get(id)
    }
}

impl<'a, C: Component, T: UnprotectedStorage<C>> Join for &'a mut TrackedStorage<C, T> {
    type Type = &'a mut C;
    type Value = &'a mut T;
    type Mask = &'a BitSet;
    fn open(self) -> (Self::Mask, Self::Value) {
        (&self.mask, &mut self.storage)
    }
    unsafe fn get(v: &mut Self::Value, id: Index) -> &'a mut C {
        // similar issue here as the `Storage<T, A, D>` implementation
        use std::mem;
        let value: &'a mut Self::Value = mem::transmute(v);
        value.get_mut(id)
    }
}
