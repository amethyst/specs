
extern crate specs;
#[macro_use]
extern crate specs_derive;

use specs::*;

#[derive(Default, Metadata)]
struct Other {
    flagged: Flagged,
}

#[derive(Default, Metadata)]
struct TestMetadata {
    flagged: Flagged,
    other: Other,
}

#[derive(Default, Metadata)]
struct TupleMetadata(Flagged, Other);

#[derive(Debug, Default)]
struct TestComp;
impl Component for TestComp {
    type Storage = DenseVecStorage<Self>;
    type Metadata = TupleMetadata;
}

fn main() {
    let mut world = World::new();
    world.register::<TestComp>();

    let e1 = world.create_entity()
        .with(TestComp)
        .build();
    let _e2 = world.create_entity()
        .with(TestComp)
        .build();
    let _e3 = world.create_entity()
        .with(TestComp)
        .build();
    let e4 = world.create_entity()
        .with(TestComp)
        .build();
    {
        let mut storage = world.write::<TestComp>();
        storage.find_mut::<Flagged>().clear_flags();
        storage.get_mut(e1);
        storage.get_mut(e4);
    }

    {
        let storage = world.read::<TestComp>();

        // Grab the metadata.
        let flagged = storage.find::<Flagged>();

        for (entity, comp, _) in (&*world.entities(), &storage, flagged).join() {
            println!("({:?}, {:?}): {:?}", entity.id(), entity.gen().id(), comp);
        }
    }

    {
        let mut storage = world.write::<TestComp>();
        storage.find::<Flagged>();
        let _inner_flagged: &mut Flagged = storage.find_mut::<Other>().find_mut();
    }

    {
        let mut storage = world.write::<TestComp>();

        {
            // Grab the metadata and clone it to avoid borrow issues
            let flagged = storage.find::<Flagged>().clone();

            // Iterate over all flagged components
            for (entity, comp, _) in (&*world.entities(), &mut storage, flagged).join() {
                println!("({:?}, {:?}): {:?}", entity.id(), entity.gen().id(), comp);
            }
        }
    }
}
