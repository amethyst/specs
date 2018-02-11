use hibitset::BitSet;
use std::iter::FromIterator;
use std::ops::AddAssign;

use join::Join;
use storage::{DenseVecStorage, UnprotectedStorage};
use world::{Entity, Index};

/// Change set that can be collected from an iterator, and joined on for easy application to
/// components.
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
/// for (health, modifier) in (&mut world.write::<Health>(), &changeset).join() {
///     health.0 -= modifier;
/// }
/// # }
/// ```
pub struct ChangeSet<T> {
    mask: BitSet,
    inner: DenseVecStorage<T>,
}

impl<T> ChangeSet<T> {
    /// Create a new change set
    pub fn new() -> Self {
        ChangeSet {
            mask: BitSet::default(),
            inner: DenseVecStorage::default(),
        }
    }

    /// Add a value to the change set. If the entity already have a value in the change set, the
    /// incoming value will be added to that.
    pub fn add(&mut self, entity: Entity, value: T)
    where
        T: AddAssign + Default,
    {
        if self.mask.contains(entity.id()) {
            unsafe {
                *self.inner.get_mut(entity.id()) += value;
            }
        } else {
            unsafe {
                self.inner.insert(entity.id(), value);
            }
            self.mask.add(entity.id());
        }
    }
}

impl<T> FromIterator<(Entity, T)> for ChangeSet<T>
where
    T: AddAssign + Default,
{
    fn from_iter<I: IntoIterator<Item = (Entity, T)>>(iter: I) -> Self {
        let mut changeset = ChangeSet::new();
        for (entity, d) in iter {
            changeset.add(entity, d);
        }
        changeset
    }
}

impl<'a, T> Join for &'a ChangeSet<T> {
    type Type = &'a T;
    type Value = &'a DenseVecStorage<T>;
    type Mask = &'a BitSet;

    fn open(self) -> (Self::Mask, Self::Value) {
        (&self.mask, &self.inner)
    }

    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        value.get(id)
    }
}

#[cfg(test)]
mod tests {
    use super::ChangeSet;
    use join::Join;
    use storage::DenseVecStorage;
    use world::{Component, World};

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
        for (health, modifier) in (&mut world.write::<Health>(), &changeset).join() {
            health.0 -= modifier;
        }
        let healths = world.read::<Health>();
        assert_eq!(68, healths.get(a).unwrap().0);
        assert_eq!(175, healths.get(b).unwrap().0);
        assert_eq!(300, healths.get(c).unwrap().0);
    }
}
