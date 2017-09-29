use hibitset::BitSet;

use join::Join;
use storage::MaskedStorage;
use world::Component;
use Index;

/// A draining storage wrapper which has a `Join` implementation
/// that removes the components.
pub struct Drain<'a, T: Component> {
    /// The masked storage
    pub data: &'a mut MaskedStorage<T>,
}

impl<'a, T> Join for Drain<'a, T>
where
    T: Component,
{
    type Type = T;
    type Value = &'a mut MaskedStorage<T>;
    type Mask = BitSet;

    fn open(self) -> (Self::Mask, Self::Value) {
        let mask = self.data.mask.clone();

        (mask, self.data)
    }

    unsafe fn get(value: &mut Self::Value, id: Index) -> T {
        value.remove(id).expect("Tried to access same index twice")
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn basic_drain() {
        use join::Join;
        use storage::DenseVecStorage;
        use world::{Component, World};

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

        let mut comps = world.write::<Comp>();
        let entities = world.entities();

        {
            let mut iter = (comps.drain(), &*entities).join();

            assert_eq!(iter.next().unwrap(), (Comp, b));
            assert_eq!(iter.next().unwrap(), (Comp, c));
            assert_eq!(iter.next().unwrap(), (Comp, e));
        }

        assert_eq!((&comps).join().count(), 0);
    }
}
