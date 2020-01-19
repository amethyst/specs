//! Provides a changeset that can be collected from an iterator.

use std::{iter::FromIterator, ops::AddAssign};

use crate::{prelude::*, storage::UnprotectedStorage, world::Index};

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
            // SAFETY: we checked the mask, thus it's safe to call
            unsafe {
                *self.inner.get_mut(entity.id()) += value;
            }
        } else {
            // SAFETY: we checked the mask, thus it's safe to call
            unsafe {
                self.inner.insert(entity.id(), value);
            }
            self.mask.add(entity.id());
        }
    }

    /// Clear the changeset
    pub fn clear(&mut self) {
        for id in &self.mask {
            // SAFETY: we checked the mask, thus it's safe to call
            unsafe {
                self.inner.remove(id);
            }
        }
        self.mask.clear();
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

impl<'a, T> Join for &'a mut ChangeSet<T> {
    type Mask = &'a BitSet;
    type Type = &'a mut T;
    type Value = &'a mut DenseVecStorage<T>;

    // SAFETY: No unsafe code and no invariants to meet.
    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        (&self.mask, &mut self.inner)
    }

    // SAFETY: No unsafe code and no invariants to meet.
    // `DistinctStorage` invariants are also met, but no `ParJoin` implementation
    // exists yet.
    unsafe fn get(v: &mut Self::Value, id: Index) -> Self::Type {
        let value: *mut Self::Value = v as *mut Self::Value;
        (*value).get_mut(id)
    }
}

impl<'a, T> Join for &'a ChangeSet<T> {
    type Mask = &'a BitSet;
    type Type = &'a T;
    type Value = &'a DenseVecStorage<T>;

    // SAFETY: No unsafe code and no invariants to meet.
    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        (&self.mask, &self.inner)
    }

    // SAFETY: No unsafe code and no invariants to meet.
    // `DistinctStorage` invariants are also met, but no `ParJoin` implementation
    // exists yet.
    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        value.get(id)
    }
}

/// A `Join` implementation for `ChangeSet` that simply removes all the entries
/// on a call to `get`.
impl<T> Join for ChangeSet<T> {
    type Mask = BitSet;
    type Type = T;
    type Value = DenseVecStorage<T>;

    // SAFETY: No unsafe code and no invariants to meet.
    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        (self.mask, self.inner)
    }

    // SAFETY: No unsafe code and no invariants to meet.
    // `DistinctStorage` invariants are also met, but no `ParJoin` implementation
    // exists yet.
    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        value.remove(id)
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
