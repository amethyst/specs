use hibitset::BitSetAll;

use join::Join;
use super::*;

impl<'e, T, D> Storage<'e, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Returns an entry to the component associated to the entity.
    ///
    /// Behaves somewhat similarly to `std::collections::HashMap`'s entry api.
    ///
    /// ## Example
    ///
    /// ```rust
    /// # extern crate specs;
    /// # use specs::prelude::*;
    /// # struct Comp {
    /// #    field: u32
    /// # }
    /// # impl Component for Comp {
    /// #    type Storage = DenseVecStorage<Self>;
    /// # }
    /// # fn main() {
    /// # let mut world = World::new();
    /// # world.register::<Comp>();
    /// # let entity = world.create_entity().build();
    /// # let mut storage = world.write_storage::<Comp>();
    ///  if let Ok(entry) = storage.entry(entity) {
    ///      entry.or_insert(Comp { field: 55 });
    ///  }
    /// # }
    /// ```
    pub fn entry<'a>(&'a mut self, e: Entity) -> Result<StorageEntry<'a, 'e, T, D>, WrongGeneration>
    where
        'e: 'a,
    {
        if self.entities.is_alive(e) {
            unsafe {
                let entries = self.entries();
                let (_, mut value): (BitSetAll, _) = entries.open();
                Ok(Entries::get(&mut value, e.id()))
            }
        } else {
            let gen = self
                .entities
                .alloc
                .generation(e.id())
                .unwrap_or(Generation::one());
            Err(WrongGeneration {
                action: "attempting to get an entry to a storage",
                actual_gen: gen,
                entity: e,
            })
        }
    }

    /// Returns a `Join`-able structure that yields all indices, returning
    /// `Entry` for all elements
    ///
    /// WARNING: Do not have a join of only `Entries`s. Otherwise the join will
    /// iterate over every single index of the bitset. If you want a join with
    /// all `Entries`s, add an `EntitiesRes` to the join as well to bound the
    /// join to all entities that are alive.
    /// 
    /// ## Example
    /// 
    /// ```rust
    /// # extern crate specs;
    /// # use specs::prelude::*;
    /// #
    /// # #[derive(Default)]
    /// # struct Counter(u32);
    /// #
    /// # impl Counter {
    /// #     fn increase(&mut self) {
    /// #         self.0 += 1
    /// #     }
    /// #     fn reached_limit(&self) -> bool {
    /// #         return self.0 >= 100;
    /// #     }
    /// #     fn reset(&mut self) {
    /// #         return self.0 = 0;
    /// #     }
    /// # }
    /// #
    /// # impl Component for Counter {
    /// #     type Storage = VecStorage<Self>;
    /// # }
    /// #
    /// # #[derive(Default)]
    /// # struct AllowCounter;
    /// #
    /// # impl Component for AllowCounter {
    /// #     type Storage = NullStorage<Self>;
    /// # }
    /// #
    /// # let mut world = World::new();
    /// # world.register::<Counter>();
    /// # for _ in 0..15 {
    /// #     world.create_entity().build();
    /// # }
    /// # 
    /// # world.exec(|(mut counters, marker): (WriteStorage<Counter>, ReadStorage<AllowCounter>)| {
    /// for (mut counter, _) in (counters.entries(), &marker).join() {
    ///     let counter = counter.or_insert_with(Default::default);
    ///     counter.increase();
    ///         
    ///     if counter.reached_limit() {
    ///         counter.reset();
    ///         // Do something
    ///     }
    /// }
    /// # });
    /// ```
    pub fn entries<'a>(&'a mut self) -> Entries<'a, 'e, T, D> {
        Entries(self)
    }
}

/// `Join`-able structure that yields all indices, returning `Entry` for all elements
pub struct Entries<'a, 'b: 'a, T: 'a, D: 'a>(&'a mut Storage<'b, T, D>);

impl<'a, 'b: 'a, T: 'a, D: 'a> Join for Entries<'a, 'b, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
{
    type Type = StorageEntry<'a, 'b, T, D>;
    type Value = &'a mut Storage<'b, T, D>;
    type Mask = BitSetAll;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        (BitSetAll, self.0)
    }

    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        // This is HACK. See implementation of Join for &'a mut Storage<'e, T, D> for
        // details why it is necessary.
        let storage: *mut Storage<'b, T, D> = *value as *mut Storage<'b, T, D>;
        if (*storage).data.mask.contains(id) {
            StorageEntry::Occupied(OccupiedEntry {
                id,
                storage: &mut *storage,
            })
        } else {
            StorageEntry::Vacant(VacantEntry {
                id,
                storage: &mut *storage,
            })
        }
    }

    #[inline]
    fn is_unconstrained() -> bool {
        true
    }
}

/// An entry to a storage which has a component associated to the entity.
pub struct OccupiedEntry<'a, 'b: 'a, T: 'a, D: 'a> {
    id: Index,
    storage: &'a mut Storage<'b, T, D>,
}

impl<'a, 'b, T, D> OccupiedEntry<'a, 'b, T, D>
where
    T: Component,
    D: Deref<Target = MaskedStorage<T>>,
{
    /// Get a reference to the component associated with the entity.
    pub fn get(&self) -> &T {
        unsafe { self.storage.data.inner.get(self.id) }
    }
}

impl<'a, 'b, T, D> OccupiedEntry<'a, 'b, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Get a mutable reference to the component associated with the entity.
    pub fn get_mut(&mut self) -> &mut T {
        unsafe { self.storage.data.inner.get_mut(self.id) }
    }

    /// Converts the `OccupiedEntry` into a mutable reference bounded by
    /// the storage's lifetime.
    pub fn into_mut(self) -> &'a mut T {
        unsafe { self.storage.data.inner.get_mut(self.id) }
    }

    /// Inserts a value into the storage and returns the old one.
    pub fn insert(&mut self, mut component: T) -> T {
        std::mem::swap(&mut component, self.get_mut());
        component
    }

    /// Removes the component from the storage and returns it.
    pub fn remove(self) -> T {
        self.storage.data.remove(self.id).unwrap()
    }
}

/// An entry to a storage which does not have a component associated to the entity.
pub struct VacantEntry<'a, 'b: 'a, T: 'a, D: 'a> {
    id: Index,
    storage: &'a mut Storage<'b, T, D>,
}

impl<'a, 'b, T, D> VacantEntry<'a, 'b, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Inserts a value into the storage.
    pub fn insert(self, component: T) -> &'a mut T {
        self.storage.data.mask.add(self.id);
        unsafe {
            self.storage.data.inner.insert(self.id, component);
            self.storage.data.inner.get_mut(self.id)
        }
    }
}

/// Entry to a storage for convenient filling of components or removal based on whether
/// the entity has a component.
pub enum StorageEntry<'a, 'b: 'a, T: 'a, D: 'a> {
    /// Entry variant that is returned if the entity has a component.
    Occupied(OccupiedEntry<'a, 'b, T, D>),
    /// Entry variant that is returned if the entity does not have a component.
    Vacant(VacantEntry<'a, 'b, T, D>),
}

impl<'a, 'b, T, D> StorageEntry<'a, 'b, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Inserts a component if the entity does not have it already.
    pub fn or_insert(self, component: T) -> &'a mut T {
        self.or_insert_with(|| component)
    }

    /// Inserts a component using a lazily called function that is only called
    /// when inserting the component. Ensures this entry has a value and if not, 
    /// inserts one using the result of the passed closure. Returns a reference 
    /// to the value afterwards.
    pub fn or_insert_with<F>(self, default: F) -> &'a mut T
    where
        F: FnOnce() -> T,
    {
        match self {
            StorageEntry::Occupied(occupied) => occupied.into_mut(),
            StorageEntry::Vacant(vacant) => vacant.insert(default()),
        }
    }
}
