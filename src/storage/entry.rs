use hibitset::BitSetAll;

use super::*;
use crate::join::LendJoin;
use crate::world::Generation;

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
    /// if let Ok(entry) = storage.entry(entity) {
    ///     entry.or_insert(Comp { field: 55 });
    /// }
    /// # }
    /// ```
    pub fn entry<'a>(&'a mut self, e: Entity) -> Result<StorageEntry<'a, 'e, T, D>, WrongGeneration>
    where
        'e: 'a,
    {
        if self.entities.is_alive(e) {
            Ok(self.entry_inner(e.id()))
        } else {
            let gen = self
                .entities
                .alloc
                .generation(e.id())
                .unwrap_or_else(Generation::one);
            Err(WrongGeneration {
                action: "attempting to get an entry to a storage",
                actual_gen: gen,
                entity: e,
            })
        }
    }

    /// Returns a [`LendJoin`]-able structure that yields all indices, returning
    /// [`StorageEntry`] for all elements
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
    /// let mut join = (counters.entries(), &marker).lend_join();
    /// while let Some((mut counter, _)) = join.next() {
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

    /// Returns an entry to the component associated with the provided index.
    ///
    /// Does not check whether an entity is alive!
    pub fn entry_inner<'a>(&'a mut self, id: Index) -> StorageEntry<'a, 'e, T, D> {
        if self.data.mask.contains(id) {
            StorageEntry::Occupied(OccupiedEntry { id, storage: self })
        } else {
            StorageEntry::Vacant(VacantEntry { id, storage: self })
        }
    }
}

/// [`LendJoin`]-able structure that yields all indices,
/// returning [`StorageEntry`] for all elements.
///
/// This can be constructed via [`Storage::entries`].
pub struct Entries<'a, 'b: 'a, T: 'a, D: 'a>(&'a mut Storage<'b, T, D>);

// SAFETY: We return a mask containing all items, but check the original mask in
// the `get` implementation. Iterating the mask does not repeat indices.
#[nougat::gat]
unsafe impl<'a, 'b: 'a, T: 'a, D: 'a> LendJoin for Entries<'a, 'b, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    type Mask = BitSetAll;
    type Type<'next> = StorageEntry<'next, 'b, T, D>;
    type Value = &'a mut Storage<'b, T, D>;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        (BitSetAll, self.0)
    }

    unsafe fn get<'next>(value: &'next mut Self::Value, id: Index) -> Self::Type<'next> {
        value.entry_inner(id)
    }

    #[inline]
    fn is_unconstrained() -> bool {
        true
    }
}

// SAFETY: LendJoin::get impl for this type is safe to call multiple times with
// the same ID.
unsafe impl<'a, 'b: 'a, T: 'a, D: 'a> RepeatableLendGet for Entries<'a, 'b, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
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
        // SAFETY: This is safe since `OccupiedEntry` is only constructed
        // after checking the mask.
        unsafe { self.storage.data.inner.get(self.id) }
    }
}

impl<'a, 'b, T, D> OccupiedEntry<'a, 'b, T, D>
where
    T: Component,
    D: DerefMut<Target = MaskedStorage<T>>,
{
    /// Get a mutable reference to the component associated with the entity.
    pub fn get_mut(&mut self) -> AccessMutReturn<'_, T> {
        // SAFETY: This is safe since `OccupiedEntry` is only constructed
        // after checking the mask.
        unsafe { self.storage.data.inner.get_mut(self.id) }
    }

    /// Converts the `OccupiedEntry` into a mutable reference bounded by
    /// the storage's lifetime.
    pub fn into_mut(self) -> AccessMutReturn<'a, T> {
        // SAFETY: This is safe since `OccupiedEntry` is only constructed
        // after checking the mask.
        unsafe { self.storage.data.inner.get_mut(self.id) }
    }

    /// Inserts a value into the storage and returns the old one.
    pub fn insert(&mut self, mut component: T) -> T {
        core::mem::swap(&mut component, self.get_mut().access_mut());
        component
    }

    /// Removes the component from the storage and returns it.
    pub fn remove(self) -> T {
        self.storage.data.remove(self.id).unwrap()
    }
}

/// An entry to a storage which does not have a component associated to the
/// entity.
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
    pub fn insert(self, component: T) -> AccessMutReturn<'a, T> {
        // Note, this method adds `id` to the mask.
        // SAFETY: `VacantEntry` is only constructed after checking that `id` is
        // not present in the mask and we consume `VacantEntry` here.
        unsafe { self.storage.not_present_insert(self.id, component) };
        // TODO (perf): We could potentially have an insert method that directly
        // produces a reference to the just inserted value.
        // SAFETY: We just inserted the component above.
        unsafe { self.storage.data.inner.get_mut(self.id) }
    }
}

/// Entry to a storage for convenient filling of components or removal based on
/// whether the entity has a component.
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
    /// Inserts a component and returns the old value in case this entry was
    /// already occupied.
    pub fn replace(self, component: T) -> Option<T> {
        match self {
            StorageEntry::Occupied(mut occupied) => Some(occupied.insert(component)),
            StorageEntry::Vacant(vacant) => {
                vacant.insert(component);
                None
            }
        }
    }

    /// Inserts a component if the entity does not have it already.
    pub fn or_insert(self, component: T) -> AccessMutReturn<'a, T> {
        self.or_insert_with(|| component)
    }

    /// Inserts a component using a lazily called function that is only called
    /// when inserting the component. Ensures this entry has a value and if not,
    /// inserts one using the result of the passed closure. Returns a reference
    /// to the value afterwards.
    pub fn or_insert_with<F>(self, default: F) -> AccessMutReturn<'a, T>
    where
        F: FnOnce() -> T,
    {
        match self {
            StorageEntry::Occupied(occupied) => occupied.into_mut(),
            StorageEntry::Vacant(vacant) => vacant.insert(default()),
        }
    }
}
