//! Provides a changeset that can be collected from an iterator.

use std::{iter::FromIterator, ops::AddAssign};

use crate::{
    prelude::*,
    storage::{SharedGetMutOnly, UnprotectedStorage},
    world::Index,
};

/// Change set that can be collected from an iterator, and joined on for easy
/// application to components.
///
/// ### Example
///
/// ```rust
/// # extern crate specs;
/// # use specs::prelude::*;
///
/// pub struct Health(i32);
///
/// impl Component for Health {
///     type Storage = DenseVecStorage<Self>;
/// }
///
/// # fn main() {
/// # let mut world = World::new();
/// # world.register::<Health>();
///
/// let a = world.create_entity().with(Health(100)).build();
/// let b = world.create_entity().with(Health(200)).build();
///
/// let changeset = [(a, 32), (b, 12), (b, 13)]
///     .iter()
///     .cloned()
///     .collect::<ChangeSet<i32>>();
/// for (health, modifier) in (&mut world.write_storage::<Health>(), &changeset).join() {
///     health.0 -= modifier;
/// }
/// # }
/// ```
pub struct ChangeSet<T> {
    mask: BitSet,
    inner: DenseVecStorage<T>,
}

impl<T> Default for ChangeSet<T> {
    fn default() -> Self {
        Self {
            mask: Default::default(),
            inner: Default::default(),
        }
    }
}

impl<T> ChangeSet<T> {
    /// Create a new change set
    pub fn new() -> Self {
        Default::default()
    }

    /// Add a value to the change set. If the entity already have a value in the
    /// change set, the incoming value will be added to that.
    pub fn add(&mut self, entity: Entity, value: T)
    where
        T: AddAssign,
    {
        if self.mask.contains(entity.id()) {
            // SAFETY: We have exclusive access (which ensures no aliasing or
            // concurrent calls from other threads) and we checked the mask,
            // thus it's safe to call.
            unsafe { *self.inner.get_mut(entity.id()) += value };
        } else {
            // SAFETY: We checked the mask, thus it's safe to call.
            unsafe { self.inner.insert(entity.id(), value) };
            self.mask.add(entity.id());
        }
    }

    /// Clear the changeset
    pub fn clear(&mut self) {
        // NOTE: We replace with default empty mask temporarily to protect against
        // unwinding from `Drop` of components.
        let mut mask_temp = core::mem::take(&mut self.mask);
        // SAFETY: `self.mask` is the correct mask as specified. We swap in a
        // temporary empty mask to ensure if this unwinds that the mask will be
        // cleared.
        unsafe { self.inner.clean(&mask_temp) };
        mask_temp.clear();
        self.mask = mask_temp;
    }
}

impl<T> FromIterator<(Entity, T)> for ChangeSet<T>
where
    T: AddAssign,
{
    fn from_iter<I: IntoIterator<Item = (Entity, T)>>(iter: I) -> Self {
        let mut changeset = Self::new();
        for (entity, d) in iter {
            changeset.add(entity, d);
        }
        changeset
    }
}

impl<T> Extend<(Entity, T)> for ChangeSet<T>
where
    T: AddAssign,
{
    fn extend<I: IntoIterator<Item = (Entity, T)>>(&mut self, iter: I) {
        for (entity, d) in iter {
            self.add(entity, d);
        }
    }
}

// TODO: lifetime issues
// SAFETY: `open` returns references to a mask and storage which are contained
// together in the `ChangeSet` and correspond.
/*#[nougat::gat]
unsafe impl<'a, T> LendJoin for &'a mut ChangeSet<T> {
    type Mask = &'a BitSet;
    type Type<'next> = &'next mut T;
    type Value = &'a mut DenseVecStorage<T>;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        (&self.mask, &mut self.inner)
    }

    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type<'_> {
        // SAFETY: Since we require that the mask was checked, an element for
        // `id` must have been inserted without being removed.
        unsafe { value.get_mut(id) }
    }
}*/

// SAFETY: `open` returns references to a mask and storage which are contained
// together in the `ChangeSet` and correspond.
unsafe impl<'a, T> Join for &'a mut ChangeSet<T> {
    type Mask = &'a BitSet;
    type Type = &'a mut T;
    type Value = SharedGetMutOnly<'a, T, DenseVecStorage<T>>;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        (&self.mask, SharedGetMutOnly::new(&mut self.inner))
    }

    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        // SAFETY:
        // * Since we require that the mask was checked, an element for
        //   `id` must have been inserted without being removed.
        // * We also require that the caller drop the value returned before
        //   subsequent calls with the same `id`, so there are no extant
        //   references that were obtained with the same `id`.
        // * Since we have an exclusive reference to `Self::Value`, we know this
        //   isn't being called from multiple threads at once.
        unsafe { value.get(id) }
    }
}

// NOTE: could implement ParJoin for `&'a mut ChangeSet`/`&'a ChangeSet`

// TODO: lifetime issues
// SAFETY: `open` returns references to a mask and storage which are contained
// together in the `ChangeSet` and correspond.
/*#[nougat::gat]
unsafe impl<'a, T> LendJoin for &'a ChangeSet<T> {
    type Mask = &'a BitSet;
    type Type<'next> = &'next T;
    type Value = &'a DenseVecStorage<T>;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        (&self.mask, &self.inner)
    }

    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type<'_> {
        // SAFETY: Since we require that the mask was checked, an element for
        // `i` must have been inserted without being removed.
        unsafe { value.get(id) }
    }
}*/

// SAFETY: `open` returns references to a mask and storage which are contained
// together in the `ChangeSet` and correspond.
unsafe impl<'a, T> Join for &'a ChangeSet<T> {
    type Mask = &'a BitSet;
    type Type = &'a T;
    type Value = &'a DenseVecStorage<T>;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        (&self.mask, &self.inner)
    }

    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        // SAFETY: Since we require that the mask was checked, an element for
        // `i` must have been inserted without being removed.
        unsafe { value.get(id) }
    }
}

// S-TODO: implement LendJoin for ChangeSet
/*
/// A `Join` implementation for `ChangeSet` that simply removes all the entries
/// on a call to `get`.
// SAFETY: `open` returns references to a mask and storage which are contained
// together in the `ChangeSet` and correspond.
unsafe impl<T> Join for ChangeSet<T> {
    type Mask = BitSet;
    type Type = T;
    type Value = DenseVecStorage<T>;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        (self.mask, self.inner)
    }

    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        // S-TODO: Following the safety requirements of `Join::get`, users can get
        // this to be UB by calling `get` dropping the returned value and
        // calling `get` with the same `id`.
        //
        // Note: the current `JoinIter` implementation will never do
        // this. `LendJoinIter` does expose an API to do this, but it is useful
        // to implement `LendJoin` so this can be joined with other types that
        // only implement `LendJoin`.
        // SAFETY: S-TODO
        unsafe { value.remove(id) }
    }
}*/

/// A `Join` implementation for `ChangeSet` that simply removes all the entries
/// on a call to `get`.
// SAFETY: `open` returns references to a mask and storage which are contained
// together in the `ChangeSet` and correspond.
unsafe impl<T> Join for ChangeSet<T> {
    type Mask = BitSet;
    type Type = T;
    type Value = DenseVecStorage<T>;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        (self.mask, self.inner)
    }

    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        // S-TODO this may not actually be safe, see the documentation on `remove` call
        // NOTE: Following the safety requirements of `Join::get`, users can get
        // this to panic by calling `get` dropping the returned value and
        // calling `get` with the same `id`. However, such a panic isn't
        // unsound. Also, the current `JoinIter` implementation will never do
        // this. `LendJoinIter` does expose an API to do this, but it is useful
        // to implement `LendJoin` so this can be joined with other types that
        // only implement `LendJoin`.
        unsafe { value.remove(id) }
    }
}

#[cfg(test)]
mod tests {
    use super::ChangeSet;
    use crate::{
        join::Join,
        storage::DenseVecStorage,
        world::{Builder, Component, WorldExt},
    };
    use shred::World;

    pub struct Health(i32);

    impl Component for Health {
        type Storage = DenseVecStorage<Self>;
    }

    #[test]
    fn test() {
        let mut world = World::new();
        world.register::<Health>();

        let a = world.create_entity().with(Health(100)).build();
        let b = world.create_entity().with(Health(200)).build();
        let c = world.create_entity().with(Health(300)).build();

        let changeset = [(a, 32), (b, 12), (b, 13)]
            .iter()
            .cloned()
            .collect::<ChangeSet<i32>>();
        for (health, modifier) in (&mut world.write_storage::<Health>(), &changeset).join() {
            health.0 -= modifier;
        }
        let healths = world.read_storage::<Health>();
        assert_eq!(68, healths.get(a).unwrap().0);
        assert_eq!(175, healths.get(b).unwrap().0);
        assert_eq!(300, healths.get(c).unwrap().0);
    }
}
