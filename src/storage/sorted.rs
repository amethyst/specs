use {Index, UnprotectedStorage};

/// A specs storage which keeps a copied version of the components
/// sorted.
pub struct SortedStorage<S, T> {
    inner: S,
    sorted: Vec<T>,
}

impl<S, T> SortedStorage<S, T> {
    /// Iterate over the sorted vector.
    pub fn iter_sorted(&self) -> ::std::slice::Iter<T> {
        self.sorted.iter()
    }

    /// Iterate over the sorted vector mutably.
    pub fn iter_sorted_mut(&mut self) -> ::std::slice::IterMut<T> {
        self.sorted.iter_mut()
    }
}

impl<S, T> UnprotectedStorage<T> for SortedStorage<S, T>
    where T: Clone + Ord,
          S: UnprotectedStorage<T>,
{
    fn new() -> Self {
        SortedStorage {
            inner: UnprotectedStorage::new(),
            sorted: Vec::new(),
        }
    }

    unsafe fn clean<F>(&mut self, f: F)
        where F: Fn(Index) -> bool
    {
        self.inner.clean(f);
    }

    unsafe fn get(&self, id: Index) -> &T {
        self.inner.get(id)
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut T {
        self.inner.get_mut(id)
    }

    unsafe fn insert(&mut self, id: Index, value: T) {
        self.inner.insert(id, value.clone());

        let sorted_id = self.sorted
            .iter()
            .position(|x| value.le(x))
            .unwrap_or(0);
        self.sorted.insert(sorted_id, value);
    }

    unsafe fn remove(&mut self, id: Index) -> T {
        let val = self.inner.remove(id);

        let sorted_id = self.sorted
            .iter()
            .position(|x| x == &val)
            .expect("Sorted vec does not contain element");
        self.sorted.remove(sorted_id);

        val
    }
}
