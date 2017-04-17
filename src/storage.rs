use std;
use std::collections::HashMap;
use std::collections::BTreeMap;
use std::hash::BuildHasherDefault;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Not};

use fnv::FnvHasher;

use bitset::{BitSet, BitSetNot};
use gate::Gate;
use join::Join;
use world::{Allocator, Component};
use {Entity, Index};

#[cfg(feature="serialize")]
use serde;


/// The `UnprotectedStorage` together with the `BitSet` that knows
/// about which elements are stored, and which are not.
pub struct MaskedStorage<T: Component> {
    mask: BitSet,
    inner: T::Storage,
}

impl<T: Component> MaskedStorage<T> {
    /// Creates a new `MaskedStorage`. This is called when you register
    /// a new component type within the world.
    pub fn new() -> MaskedStorage<T> {
        MaskedStorage {
            mask: BitSet::new(),
            inner: UnprotectedStorage::new(),
        }
    }

    fn open_mut(&mut self) -> (&BitSet, &mut T::Storage) {
        (&self.mask, &mut self.inner)
    }

    /// Clear the contents of this storage.
    pub fn clear(&mut self) {
        let mask = &mut self.mask;
        unsafe {
            self.inner.clean(|i| mask.contains(i));
        }
        mask.clear();
    }

    /// Remove an element by a given index.
    pub fn remove(&mut self, id: Index) -> Option<T> {
        if self.mask.remove(id) {
            Some(unsafe { self.inner.remove(id) })
        } else {
            None
        }
    }
}

impl<T: Component> Drop for MaskedStorage<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

/// An inverted storage type, only useful to iterate entities
/// that do not have a particular component type.
pub struct AntiStorage<'a>(&'a BitSet);

impl<'a> Join for AntiStorage<'a> {
    type Type = ();
    type Value = ();
    type Mask = BitSetNot<&'a BitSet>;

    fn open(self) -> (Self::Mask, ()) {
        (BitSetNot(self.0), ())
    }

    unsafe fn get(_: &mut (), _: Index) -> () {
        ()
    }
}

/// A storage type that iterates entities that have
/// a particular component type, but does not return the
/// component.
pub struct CheckStorage(BitSet);

impl<'a> Join for &'a CheckStorage {
    type Type = ();
    type Value = ();
    type Mask = &'a BitSet;

    fn open(self) -> (Self::Mask, ()) {
        (&self.0, ())
    }

    unsafe fn get(_: &mut (), _: Index) -> () {
        ()
    }
}

/// A wrapper around the masked storage and the generations vector.
/// Can be used for safe lookup of components, insertions and removes.
/// This is what `World::read/write` locks for the user.
pub struct Storage<T, A, D> {
    phantom: PhantomData<T>,
    alloc: A,
    data: D,
}

impl<'a, T, A, D> Not for &'a Storage<T, A, D>
    where T: Component,
          D: Deref<Target = MaskedStorage<T>>
{
    type Output = AntiStorage<'a>;

    fn not(self) -> Self::Output {
        AntiStorage(&self.data.mask)
    }
}

impl<T, A, D> Storage<T, A, D> {
    /// Create a new `Storage`
    pub fn new(alloc: A, data: D) -> Storage<T, A, D> {
        Storage {
            phantom: PhantomData,
            alloc: alloc,
            data: data,
        }
    }
}

impl<T, A, D> Storage<T, A, D>
    where T: Component,
          A: Deref<Target = Allocator>,
          D: Deref<Target = MaskedStorage<T>>
{
    /// Tries to read the data associated with an `Entity`.
    pub fn get(&self, e: Entity) -> Option<&T> {
        if self.data.mask.contains(e.get_id()) && self.alloc.is_alive(e) {
            Some(unsafe { self.data.inner.get(e.get_id()) })
        } else {
            None
        }
    }

    /// Returns a struct that can iterate over the entities that have it
    /// but does not return the contents of the storage.
    ///
    /// Useful if you want to check if an entity has a component
    /// and then possibly get the component later on in the loop.
    pub fn check(&self) -> CheckStorage {
        CheckStorage(self.data.mask.clone())
    }
}

impl<T, A, D> Gate for Storage<T, A, D> {
    type Target = Self;

    fn pass(self) -> Self {
        self
    }
}


/// the status of an insert operation
pub enum InsertResult<T> {
    /// The value was inserted and there was no value before
    Inserted,
    /// The value was updated an already inserted value
    /// the value returned is the old value
    Updated(T),
    /// The value failed to insert because the entity
    /// was invalid
    EntityIsDead(T),
}

impl<T, A, D> Storage<T, A, D>
    where T: Component,
          A: Deref<Target = Allocator>,
          D: DerefMut<Target = MaskedStorage<T>>
{
    /// Tries to mutate the data associated with an `Entity`.
    pub fn get_mut(&mut self, e: Entity) -> Option<&mut T> {
        if self.data.mask.contains(e.get_id()) && self.alloc.is_alive(e) {
            Some(unsafe { self.data.inner.get_mut(e.get_id()) })
        } else {
            None
        }
    }

    /// Inserts new data for a given `Entity`.
    /// Returns the result of the operation as a `InsertResult<T>`
    pub fn insert(&mut self, e: Entity, mut v: T) -> InsertResult<T> {
        if self.alloc.is_alive(e) {
            let id = e.get_id();
            if self.data.mask.contains(id) {
                std::mem::swap(&mut v, unsafe { self.data.inner.get_mut(id) });
                InsertResult::Updated(v)
            } else {
                self.data.mask.add(id);
                unsafe { self.data.inner.insert(id, v) };
                InsertResult::Inserted
            }
        } else {
            InsertResult::EntityIsDead(v)
        }
    }

    /// Removes the data associated with an `Entity`.
    pub fn remove(&mut self, e: Entity) -> Option<T> {
        if self.alloc.is_alive(e) {
            self.data.remove(e.get_id())
        } else {
            None
        }
    }

    /// Clears the contents of the storage.
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

#[cfg(feature="serialize")]
impl<T, A, D> Storage<T, A, D> where
    T: Component + serde::Deserialize,
    A: Deref<Target = Allocator>,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Merges a list of components into the storage.
    ///
    /// The list of entities will be used as the base for the offsets of the packed data.
    ///
    /// e.g.
    /// ```rust,ignore
    ///let list = vec![Entity(0, 1), Entity(1, 1), Entity(2, 1)];
    ///let packed = PackedData { offsets: [0, 2], components: [ ... ] };
    ///storage.merge(&list, packed);
    /// ```
    /// Would merge the components at offset 0 and 2, which would be `Entity(0, 1)` and `Entity(2, 1)` while ignoring
    /// `Entity(1, 1)`.
    pub fn merge<'a>(&'a mut self, entities: &'a Vec<Entity>, mut packed: PackedData<T>) {
        for (component, offset) in packed.components.drain(..).zip(packed.offsets.iter()) {
            match entities.get(*offset as usize) {
                Some(entity) => { self.insert(*entity, component); },
                None => { println!("No entity at offset {:?}", *offset as usize) }
            }
        }
    }
}

impl<'a, T, A, D> Join for &'a Storage<T, A, D> where
    T: Component,
    A: Deref<Target = Allocator>,
    D: Deref<Target = MaskedStorage<T>>,
{
    type Type = &'a T;
    type Value = &'a T::Storage;
    type Mask = &'a BitSet;

    fn open(self) -> (Self::Mask, Self::Value) {
        (&self.data.mask, &self.data.inner)
    }

    unsafe fn get(v: &mut Self::Value, i: Index) -> &'a T {
        v.get(i)
    }
}

impl<'a, T, A, D> Join for &'a mut Storage<T, A, D>
    where T: Component,
          A: Deref<Target = Allocator>,
          D: DerefMut<Target = MaskedStorage<T>>
{
    type Type = &'a mut T;
    type Value = &'a mut T::Storage;
    type Mask = &'a BitSet;

    fn open(self) -> (Self::Mask, Self::Value) {
        self.data.open_mut()
    }

    unsafe fn get(v: &mut Self::Value, i: Index) -> &'a mut T {
        use std::mem;

        // This is horribly unsafe. Unfortunately, Rust doesn't provide a way
        // to abstract mutable/immutable state at the moment, so we have to hack
        // our way through it.
        let value: &'a mut Self::Value = mem::transmute(v);
        value.get_mut(i)
    }
}

#[cfg(feature="serialize")]
impl<T, A, D> serde::Serialize for Storage<T, A, D> where
    T: Component + serde::Serialize,
    A: Deref<Target = Allocator>,
    D: Deref<Target = MaskedStorage<T>>,
{
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use bitset::BitSetLike;
        use serde::ser::SerializeStruct;

        // Serializes the storage in a format of PackedData<T>
        let (bitset, storage) = self.open();
        let mut structure = serializer.serialize_struct("PackedData", 2)?;
        let mut components: Vec<&T> = Vec::new();
        let mut offsets: Vec<u32> = Vec::new();
        for index in bitset.iter() {
            offsets.push(index);
            unsafe {
                components.push(storage.get(index));
            }
        }

        structure.serialize_field("offsets", &offsets)?;
        structure.serialize_field("components", &components)?;
        structure.end()
    }
}

#[cfg(feature="serialize")]
#[derive(Debug, Serialize, Deserialize)]
/// Structure of packed components with offsets of which entities they belong to.
/// Offsets define which entities the components correspond to, based on a list of entities
/// the packed data is sent in with.
///
/// If the list of entities is all entities in the world, then the offsets in the 
/// packed data are the indices of the entities.
pub struct PackedData<T> {
    /// List of components.
    pub components: Vec<T>,
    /// Offsets used to get entities which correspond to the components.
    pub offsets: Vec<u32>,
}

#[cfg(feature="serialize")]
impl<T> PackedData<T> {
    /// Shifts all offsets in the packed data to match a new base.
    ///
    /// Useful if you want to merge some components ontop of the current world:
    /// `packed.rebase( ... /* amount of entities in world */ );`
    ///
    /// If the base is higher than the offset then the function will return the index of that offset.
    pub fn rebase(&mut self, base: u32) -> Option<usize> {
        for index in 0..self.offsets.len() {
            if self.offsets[index] < base {
                return Some(index);
            }
            self.offsets[index] -= base;
        }
        None
    }
}

/// 
pub struct GatedStorage<T, A, G> {
    marker: PhantomData<T>,
    alloc: A,
    gate: G,
}

impl<T, A, G> GatedStorage<T, A, G> {
    /// Creates a new `GatedStorage`.
    pub fn new(alloc: A, gate: G) -> Self {
        GatedStorage {
            marker: PhantomData,
            alloc: alloc,
            gate: gate,
        }
    }
}

impl<T, A, G: Gate> Gate for GatedStorage<T, A, G> {
    type Target = Storage<T, A, G::Target>;

    fn pass(self) -> Self::Target {
        Storage::new(self.alloc, self.gate.pass())
    }
}


/// Used by the framework to quickly join componets
pub trait UnprotectedStorage<T>: Sized {
    /// Creates a new `Storage<T>`. This is called when you register a new
    /// component type within the world.
    fn new() -> Self;

    /// Clean the storage given a check to figure out if an index
    /// is valid or not. Allows us to safely drop the storage.
    unsafe fn clean<F>(&mut self, F) where F: Fn(Index) -> bool;

    /// Tries reading the data associated with an `Index`.
    /// This is unsafe because the external set used
    /// to protect this storage is absent.
    unsafe fn get(&self, id: Index) -> &T;

    /// Tries mutating the data associated with an `Index`.
    /// This is unsafe because the external set used
    /// to protect this storage is absent.
    unsafe fn get_mut(&mut self, id: Index) -> &mut T;

    /// Inserts new data for a given `Index`.
    unsafe fn insert(&mut self, Index, T);

    /// Removes the data associated with an `Index`.
    unsafe fn remove(&mut self, Index) -> T;
}

/// HashMap-based storage. Best suited for rare components.
pub struct HashMapStorage<T>(HashMap<Index, T, BuildHasherDefault<FnvHasher>>);

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


#[cfg(test)]
mod map_test {
    use mopa::Any;
    use super::{Storage, MaskedStorage, VecStorage};
    use world::Allocator;
    use {Component, Entity, Index, Generation};

    struct Comp<T>(T);
    impl<T: Any + Send + Sync> Component for Comp<T> {
        type Storage = VecStorage<Comp<T>>;
    }

    fn ent(i: Index) -> Entity {
        Entity::new(i, Generation(1))
    }

    #[test]
    fn insert() {
        let mut c = Storage::new(Box::new(Allocator::new()), Box::new(MaskedStorage::new()));

        for i in 0..1_000 {
            c.insert(ent(i), Comp(i));
        }

        for i in 0..1_000 {
            assert_eq!(c.get(ent(i)).unwrap().0, i);
        }
    }

    #[test]
    fn insert_100k() {
        let mut c = Storage::new(Box::new(Allocator::new()), Box::new(MaskedStorage::new()));

        for i in 0..100_000 {
            c.insert(ent(i), Comp(i));
        }

        for i in 0..100_000 {
            assert_eq!(c.get(ent(i)).unwrap().0, i);
        }
    }

    #[test]
    fn remove() {
        let mut c = Storage::new(Box::new(Allocator::new()), Box::new(MaskedStorage::new()));

        for i in 0..1_000 {
            c.insert(ent(i), Comp(i));
        }

        for i in 0..1_000 {
            assert_eq!(c.get(ent(i)).unwrap().0, i);
        }

        for i in 0..1_000 {
            c.remove(ent(i));
        }

        for i in 0..1_000 {
            assert!(c.get(ent(i)).is_none());
        }
    }

    #[test]
    fn test_gen() {
        let mut c = Storage::new(Box::new(Allocator::new()), Box::new(MaskedStorage::new()));

        for i in 0..1_000i32 {
            c.insert(ent(i as u32), Comp(i));
            c.insert(ent(i as u32), Comp(-i));
        }

        for i in 0..1_000i32 {
            assert_eq!(c.get(ent(i as u32)).unwrap().0, -i);
        }
    }

    #[test]
    fn insert_same_key() {
        let mut c = Storage::new(Box::new(Allocator::new()), Box::new(MaskedStorage::new()));

        for i in 0..10_000 {
            c.insert(ent(i), Comp(i));
            assert_eq!(c.get(ent(i)).unwrap().0, i);
        }
    }

    #[should_panic]
    #[test]
    fn wrap() {
        let mut c = Storage::new(Box::new(Allocator::new()), Box::new(MaskedStorage::new()));

        c.insert(ent(1 << 25), Comp(7));
    }
}

#[test]
fn test_vec_arc() {
    use std::sync::Arc;

    struct A(Arc<()>);

    let mut storage = VecStorage::<A>::new();

    unsafe {
        for i in (0..200).filter(|i| i % 2 != 0) {
            storage.insert(i, A(Arc::new(())));
        }
        storage.clean(|i| i % 2 != 0);
    }
}

#[cfg(test)]
mod test {
    use std::convert::AsMut;
    use std::fmt::Debug;
    use super::{Storage, MaskedStorage, VecStorage, HashMapStorage, BTreeStorage, NullStorage};
    use world::Allocator;
    use {Component, Entity, Generation};

    #[derive(PartialEq, Eq, Debug)]
    struct Cvec(u32);
    impl From<u32> for Cvec {
        fn from(v: u32) -> Cvec {
            Cvec(v)
        }
    }
    impl AsMut<u32> for Cvec {
        fn as_mut(&mut self) -> &mut u32 {
            &mut self.0
        }
    }
    impl Component for Cvec {
        type Storage = VecStorage<Cvec>;
    }

    #[derive(PartialEq, Eq, Debug)]
    struct Cmap(u32);
    impl From<u32> for Cmap {
        fn from(v: u32) -> Cmap {
            Cmap(v)
        }
    }
    impl AsMut<u32> for Cmap {
        fn as_mut(&mut self) -> &mut u32 {
            &mut self.0
        }
    }
    impl Component for Cmap {
        type Storage = HashMapStorage<Cmap>;
    }

    #[derive(PartialEq, Eq, Debug)]
    struct CBtree(u32);
    impl From<u32> for CBtree {
        fn from(v: u32) -> CBtree {
            CBtree(v)
        }
    }
    impl AsMut<u32> for CBtree {
        fn as_mut(&mut self) -> &mut u32 {
            &mut self.0
        }
    }
    impl Component for CBtree {
        type Storage = BTreeStorage<CBtree>;
    }

    #[derive(Clone)]
    struct Cnull(u32);
    impl Default for Cnull {
        fn default() -> Cnull {
            Cnull(0)
        }
    }
    impl From<u32> for Cnull {
        fn from(v: u32) -> Cnull {
            Cnull(v)
        }
    }
    impl Component for Cnull {
        type Storage = NullStorage<Cnull>;
    }

    fn create<T: Component>() -> Storage<T, Box<Allocator>, Box<MaskedStorage<T>>> {
        Storage::new(Box::new(Allocator::new()),
                     Box::new(MaskedStorage::<T>::new()))
    }

    fn test_add<T: Component + From<u32> + Debug + Eq>() {
        let mut s = Storage::new(Box::new(Allocator::new()),
                                 Box::new(MaskedStorage::<T>::new()));

        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation(1)), (i + 2718).into());
        }

        for i in 0..1_000 {
            assert_eq!(s.get(Entity::new(i, Generation(1))).unwrap(),
                       &(i + 2718).into());
        }
    }

    fn test_sub<T: Component + From<u32> + Debug + Eq>() {
        let mut s = Storage::new(Box::new(Allocator::new()),
                                 Box::new(MaskedStorage::<T>::new()));

        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation(1)), (i + 2718).into());
        }

        for i in 0..1_000 {
            assert_eq!(s.remove(Entity::new(i, Generation(1))).unwrap(),
                       (i + 2718).into());
            assert!(s.remove(Entity::new(i, Generation(1))).is_none());
        }
    }

    fn test_get_mut<T: Component + From<u32> + AsMut<u32> + Debug + Eq>() {
        let mut s = Storage::new(Box::new(Allocator::new()),
                                 Box::new(MaskedStorage::<T>::new()));

        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation(1)), (i + 2718).into());
        }

        for i in 0..1_000 {
            *s.get_mut(Entity::new(i, Generation(1)))
                 .unwrap()
                 .as_mut() -= 718;
        }

        for i in 0..1_000 {
            assert_eq!(s.get(Entity::new(i, Generation(1))).unwrap(),
                       &(i + 2000).into());
        }
    }

    fn test_add_gen<T: Component + From<u32> + Debug + Eq>() {
        let mut s = Storage::new(Box::new(Allocator::new()),
                                 Box::new(MaskedStorage::<T>::new()));

        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation(1)), (i + 2718).into());
            s.insert(Entity::new(i, Generation(2)), (i + 31415).into());
        }

        for i in 0..1_000 {
            assert!(s.get(Entity::new(i, Generation(2))).is_none());
            assert_eq!(s.get(Entity::new(i, Generation(1))).unwrap(),
                       &(i + 2718).into());
        }
    }

    fn test_sub_gen<T: Component + From<u32> + Debug + Eq>() {
        let mut s = Storage::new(Box::new(Allocator::new()),
                                 Box::new(MaskedStorage::<T>::new()));

        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation(2)), (i + 2718).into());
        }

        for i in 0..1_000 {
            assert!(s.remove(Entity::new(i, Generation(1))).is_none());
        }
    }

    fn test_clear<T: Component + From<u32>>() {
        let mut s = Storage::new(Box::new(Allocator::new()),
                                 Box::new(MaskedStorage::<T>::new()));

        for i in 0..10 {
            s.insert(Entity::new(i, Generation(1)), (i + 10).into());
        }

        s.clear();

        for i in 0..10 {
            assert!(s.get(Entity::new(i, Generation(1))).is_none());
        }
    }

    fn test_anti<T: Component + From<u32> + Debug + Eq>() {
        use join::Join;
        let mut s = create::<T>();

        for i in 0..10 {
            s.insert(Entity::new(i, Generation(1)), (i + 10).into());
        }

        for (i, (a, _)) in (&s, !&s).join().take(10).enumerate() {
            assert_eq!(a, &(i as u32).into());
        }
    }


    #[test]
    fn vec_test_add() {
        test_add::<Cvec>();
    }
    #[test]
    fn vec_test_sub() {
        test_sub::<Cvec>();
    }
    #[test]
    fn vec_test_get_mut() {
        test_get_mut::<Cvec>();
    }
    #[test]
    fn vec_test_add_gen() {
        test_add_gen::<Cvec>();
    }
    #[test]
    fn vec_test_sub_gen() {
        test_sub_gen::<Cvec>();
    }
    #[test]
    fn vec_test_clear() {
        test_clear::<Cvec>();
    }
    #[test]
    fn vec_test_anti() {
        test_anti::<Cvec>();
    }

    #[test]
    fn hash_test_add() {
        test_add::<Cmap>();
    }
    #[test]
    fn hash_test_sub() {
        test_sub::<Cmap>();
    }
    #[test]
    fn hash_test_get_mut() {
        test_get_mut::<Cmap>();
    }
    #[test]
    fn hash_test_add_gen() {
        test_add_gen::<Cmap>();
    }
    #[test]
    fn hash_test_sub_gen() {
        test_sub_gen::<Cmap>();
    }
    #[test]
    fn hash_test_clear() {
        test_clear::<Cmap>();
    }

    #[test]
    fn btree_test_add() {
        test_add::<CBtree>();
    }
    #[test]
    fn btree_test_sub() {
        test_sub::<CBtree>();
    }
    #[test]
    fn btree_test_get_mut() {
        test_get_mut::<CBtree>();
    }
    #[test]
    fn btree_test_add_gen() {
        test_add_gen::<CBtree>();
    }
    #[test]
    fn btree_test_sub_gen() {
        test_sub_gen::<CBtree>();
    }
    #[test]
    fn btree_test_clear() {
        test_clear::<CBtree>();
    }

    #[test]
    fn dummy_test_clear() {
        test_clear::<Cnull>();
    }
}

#[cfg(feature="serialize")]
#[cfg(test)]
mod serialize_test {
    extern crate serde_json;

    use super::{Entity, Join, VecStorage, Component, Gate, PackedData};
    use world::World;

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct CompTest {
        field1: u32,
        field2: bool,
    }
    impl Component for CompTest {
        type Storage = VecStorage<CompTest>;
    }

    #[test]
    fn serialize_storage() {
        // set up
        let mut world = {
            let mut world = World::<()>::new();
            world.register::<CompTest>();
            world.create_now().with(CompTest { field1: 0, field2: true }).build();
            world.create_now().with(CompTest { field1: 158123, field2: false }).build();
            world.create_now().with(CompTest { field1: u32::max_value(), field2: false }).build();
            world
        };

        let storage = world.read::<CompTest>().pass();
        let serialized = serde_json::to_string(&storage).unwrap();
        assert_eq!(serialized, r#"{"offsets":[0,1,2],"components":[{"field1":0,"field2":true},{"field1":158123,"field2":false},{"field1":4294967295,"field2":false}]}"#);
    }

    #[test]
    fn deserialize_storage() {
        // set up
        let (mut world, entities) = {
            let mut world = World::<()>::new();
            world.register::<CompTest>();
            let entities = world.create_iter().take(10).collect::<Vec<Entity>>();
            (world, entities)
        };

        let data = r#"
            {
                "offsets":[3,7,8],
                "components": [
                    {
                        "field1":0,
                        "field2":true
                    },
                    {
                        "field1":158123,
                        "field2":false
                    },
                    {
                        "field1":4294967295,
                        "field2":false
                    }
                ]
            }
        "#;

        let mut storage = world.write::<CompTest>().pass();
        let packed: PackedData<CompTest> = serde_json::from_str(&data).unwrap();
        assert_eq!(packed.offsets, vec![3, 7, 8]);
        assert_eq!(packed.components, vec![
            CompTest { field1: 0, field2: true, },
            CompTest { field1: 158123, field2: false, },
            CompTest { field1: u32::max_value(), field2: false, },
        ]);

        storage.merge(&entities, packed);

        assert_eq!((&storage).join().count(), 3);
        assert_eq!((&storage).get(entities[3]), Some(&CompTest { field1: 0, field2: true }));
        assert_eq!((&storage).get(entities[7]), Some(&CompTest { field1: 158123, field2: false }));
        assert_eq!((&storage).get(entities[8]), Some(&CompTest { field1: u32::max_value(), field2: false }));

        let none = vec![0, 1, 2, 4, 5, 6, 9];
        for entity in none {
            assert_eq!((&storage).get(entities[entity]), None);
        }
    }
}
