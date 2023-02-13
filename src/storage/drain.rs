use hibitset::BitSet;

#[nougat::gat(Type)]
use crate::join::LendJoin;
use crate::{
    join::{Join, RepeatableLendGet},
    storage::MaskedStorage,
    world::{Component, Index},
};

/// A draining storage wrapper which has a `Join` implementation
/// that removes the components.
pub struct Drain<'a, T: Component> {
    /// The masked storage
    pub data: &'a mut MaskedStorage<T>,
}

// SAFETY: Calling `get` is always safe! Iterating the mask does not repeat
// indices.
#[nougat::gat]
unsafe impl<'a, T> LendJoin for Drain<'a, T>
where
    T: Component,
{
    type Mask = BitSet;
    type Type<'next> = T;
    type Value = &'a mut MaskedStorage<T>;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        // TODO: Cloning the whole bitset here seems expensive, and it is
        // hidden from the user, but there is no obvious way to restructure
        // things to avoid this with the way that bitsets are composed together
        // for iteration.
        let mask = self.data.mask.clone();

        (mask, self.data)
    }

    unsafe fn get<'next>(value: &'next mut Self::Value, id: Index) -> T
    where
        Self: 'next,
    {
        value.remove(id).expect("Tried to access same index twice")
    }
}

// SAFETY: Calling `get` is always safe!
unsafe impl<'a, T> RepeatableLendGet for Drain<'a, T> where T: Component {}

// SAFETY: Calling `get` is always safe! Iterating the mask does not repeat
// indices.
unsafe impl<'a, T> Join for Drain<'a, T>
where
    T: Component,
{
    type Mask = BitSet;
    type Type = T;
    type Value = &'a mut MaskedStorage<T>;

    unsafe fn open(self) -> (Self::Mask, Self::Value) {
        // TODO: Cloning the whole bitset here seems expensive, and it is
        // hidden from the user, but there is no obvious way to restructure
        // things to avoid this with the way that bitsets are composed together
        // for iteration.
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
