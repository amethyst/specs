use fnv::FnvHashMap;
use hibitset::BitSet;
use std::iter::FromIterator;
use std::ops::AddAssign;

use join::Join;
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
    inner: FnvHashMap<Index, T>,
}

impl<T> FromIterator<(Entity, T)> for ChangeSet<T>
where
    T: AddAssign<T> + Default,
{
    fn from_iter<I: IntoIterator<Item = (Entity, T)>>(iter: I) -> Self {
        let mut inner = FnvHashMap::default();
        let mut mask = BitSet::default();
        for (entity, d) in iter {
            let v = inner.entry(entity.id()).or_insert_with(T::default);
            *v += d;
            mask.add(entity.id());
        }
        ChangeSet { mask, inner }
    }
}

impl<'a, T> Join for &'a ChangeSet<T> {
    type Type = &'a T;
    type Value = &'a FnvHashMap<Index, T>;
    type Mask = &'a BitSet;

    fn open(self) -> (Self::Mask, Self::Value) {
        (&self.mask, &self.inner)
    }

    unsafe fn get(value: &mut Self::Value, id: Index) -> Self::Type {
        value.get(&id).unwrap()
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
