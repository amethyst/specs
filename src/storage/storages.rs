//! Different types of storages you can use for your components.

use std::collections::BTreeMap;

use fnv::FnvHashMap;

use {DistinctStorage, Index, UnprotectedStorage};

#[cfg(feature = "rudy")]
use rudy::rudymap::RudyMap;

/// BTreeMap-based storage.
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct BTreeStorage<T>(BTreeMap<Index, T>);

impl<T> UnprotectedStorage<T> for BTreeStorage<T> {
    unsafe fn clean<F>(&mut self, _: F)
    where
        F: Fn(Index) -> bool,
    {
        // nothing to do
    }

    unsafe fn get(&self, id: Index) -> &T {
        &self.0[&id]
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

unsafe impl<T> DistinctStorage for BTreeStorage<T> {}

/// HashMap-based storage. Best suited for rare components.
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct HashMapStorage<T>(FnvHashMap<Index, T>);

impl<T> UnprotectedStorage<T> for HashMapStorage<T> {
    unsafe fn clean<F>(&mut self, _: F)
    where
        F: Fn(Index) -> bool,
    {
        //nothing to do
    }

    unsafe fn get(&self, id: Index) -> &T {
        &self.0[&id]
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

unsafe impl<T> DistinctStorage for HashMapStorage<T> {}

/// Dense vector storage. Has a redirection 2-way table
/// between entities and components, allowing to leave
/// no gaps within the data.
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct DenseVecStorage<T> {
    data: Vec<T>,
    entity_id: Vec<Index>,
    data_id: Vec<Index>,
}

impl<T> UnprotectedStorage<T> for DenseVecStorage<T> {
    unsafe fn clean<F>(&mut self, _: F)
    where
        F: Fn(Index) -> bool,
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

unsafe impl<T> DistinctStorage for DenseVecStorage<T> {}

/// A null storage type, used for cases where the component
/// doesn't contain any data and instead works as a simple flag.
pub struct NullStorage<T>(T);

impl<T: Default> UnprotectedStorage<T> for NullStorage<T> {
    unsafe fn clean<F>(&mut self, _: F)
    where
        F: Fn(Index) -> bool,
    {
    }

    unsafe fn get(&self, _: Index) -> &T {
        &self.0
    }

    unsafe fn get_mut(&mut self, _: Index) -> &mut T {
        &mut self.0
    }

    unsafe fn insert(&mut self, _: Index, _: T) {}

    unsafe fn remove(&mut self, _: Index) -> T {
        Default::default()
    }
}

impl<T> Default for NullStorage<T>
where
    T: Default,
{
    fn default() -> Self {
        use std::mem::size_of;

        assert_eq!(size_of::<T>(), 0, "NullStorage can only be used with ZST");

        NullStorage(Default::default())
    }
}

/// This is safe because you cannot mutate ZSTs.
unsafe impl<T> DistinctStorage for NullStorage<T> {}

/// Vector storage. Uses a simple `Vec`. Supposed to have maximum
/// performance for the components mostly present in entities.
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct VecStorage<T>(Vec<T>);

impl<T> UnprotectedStorage<T> for VecStorage<T> {
    unsafe fn clean<F>(&mut self, has: F)
    where
        F: Fn(Index) -> bool,
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

unsafe impl<T> DistinctStorage for VecStorage<T> {}

/// Rudy-based storage.
#[cfg(feature = "rudy")]
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct RudyStorage<T>(RudyMap<Index, T>);

#[cfg(feature = "rudy")]
impl<T> UnprotectedStorage<T> for RudyStorage<T> {
    unsafe fn clean<F>(&mut self, _: F)
    where
        F: Fn(Index) -> bool,
    {
    }

    unsafe fn get(&self, id: Index) -> &T {
        self.0.get(id).unwrap()
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        self.0.get_mut(id).unwrap()
    }

    unsafe fn insert(&mut self, id: Index, v: T) {
        self.0.insert(id, v);
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        self.0.remove(id).unwrap()
    }
}

// Rudy does satisfy the DistinctStorage guarantee:
//   https://github.com/adevore/rudy/issues/12
#[cfg(feature = "rudy")]
unsafe impl<T> DistinctStorage for RudyStorage<T> {}
