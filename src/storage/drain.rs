use hibitset::BitSet;

use crate::{
    join::Join,
    storage::MaskedStorage,
    world::{Component, Index},
};

#[cfg(feature = "nightly")]
use crate::world::{EntitiesRes, HasIndex};

/// A draining storage wrapper which has a `Join` implementation
/// that removes the components.
pub struct Drain<'a, T: Component> {
    /// The masked storage
    pub data: &'a mut MaskedStorage<T>,
    /// Entities to get the generation for component events
    #[cfg(feature = "nightly")]
    pub entities: &'a EntitiesRes,
}

impl<'a, T> Join for Drain<'a, T>
where
    T: Component,
{
    type Mask = BitSet;
    type Type = T;
    #[cfg(feature = "nightly")]
    type Value = (&'a mut MaskedStorage<T>, &'a EntitiesRes);
    #[cfg(not(feature = "nightly"))]
    type Value = &'a mut MaskedStorage<T>;

    // SAFETY: No invariants to meet and no unsafe code.
    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        let mask = self.data.mask.clone();

        #[cfg(feature = "nightly")]
        let t = (mask, (self.data, self.entities));
        #[cfg(not(feature = "nightly"))]
        let t = (mask, self.data);

        t
    }

    // SAFETY: No invariants to meet and no unsafe code.
    #[cfg(feature = "nightly")]
    unsafe fn get((storage, entities): &mut Self::Value, id: Index) -> T {
        storage
            .remove(HasIndex::from_index(id, entities))
            .expect("Tried to access same index twice")
    }

    #[cfg(not(feature = "nightly"))]
    // SAFETY: No invariants to meet and no unsafe code.
    unsafe fn get(value: &mut Self::Value, id: Index) -> T {
        value.remove(id).expect("Tried to access same index twice")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn basic_drain() {
        use crate::{
            join::Join,
            storage::DenseVecStorage,
            world::{Builder, Component, World, WorldExt},
        };

        #[derive(Debug, PartialEq)]
        struct Comp;

        impl Component for Comp {
            type Storage = DenseVecStorage<Self>;
        }

        let mut world = World::new();
        world.register::<Comp>();

        world.create_entity().build();
        let b = world.create_entity().with(Comp).build();
        let c = world.create_entity().with(Comp).build();
        world.create_entity().build();
        let e = world.create_entity().with(Comp).build();

        let mut comps = world.write_storage::<Comp>();
        let entities = world.entities();

        {
            let mut iter = (comps.drain(), &entities).join();

            assert_eq!(iter.next().unwrap(), (Comp, b));
            assert_eq!(iter.next().unwrap(), (Comp, c));
            assert_eq!(iter.next().unwrap(), (Comp, e));
        }

        assert_eq!((&comps).join().count(), 0);
    }
}
