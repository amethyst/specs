use super::*;
use join::Join;
use storage::VecStorage;

struct Pos;

impl Component for Pos {
    type Storage = VecStorage<Self>;
}

struct Vel;

impl Component for Vel {
    type Storage = VecStorage<Self>;
}

#[test]
fn delete_all() {
    let mut world = World::new();

    world.register::<Pos>();
    world.register::<Vel>();

    world.create_entity().build();
    let b = world.create_entity().with(Pos).with(Vel).build();
    world.create_entity().with(Pos).with(Vel).build();

    assert_eq!(world.entities().join().count(), 3);

    world.delete_all();

    assert_eq!(world.entities().join().count(), 0);
    assert!(world.read::<Pos>().get(b).is_none());
}

#[test]
fn lazy_insertion() {
    let mut world = World::new();
    world.register::<Pos>();
    world.register::<Vel>();

    let e1;
    let e2;
    {
        let entities = world.read_resource::<EntitiesRes>();
        let lazy = world.read_resource::<LazyUpdate>();

        e1 = entities.create();
        e2 = entities.create();
        lazy.insert(e1, Pos);
        lazy.insert_all(vec![(e1, Vel), (e2, Vel)]);
    }

    world.maintain();
    assert!(world.read::<Pos>().get(e1).is_some());
    assert!(world.read::<Vel>().get(e1).is_some());
    assert!(world.read::<Vel>().get(e2).is_some());
}

#[test]
fn lazy_removal() {
    let mut world = World::new();
    world.register::<Pos>();

    let e = world.create_entity().with(Pos).build();
    {
        let lazy = world.read_resource::<LazyUpdate>();
        lazy.remove::<Pos>(e);
    }

    world.maintain();
    assert!(world.read::<Pos>().get(e).is_none());
}

#[test]
fn lazy_execution() {
    let mut world = World::new();
    world.register::<Pos>();

    let e = {
        let entity_res = world.read_resource::<EntitiesRes>();
        entity_res.create()
    };
    {
        let lazy = world.read_resource::<LazyUpdate>();
        lazy.execute(move |world| {
            world.write::<Pos>().insert(e, Pos);
        });
    }

    world.maintain();
    assert!(world.read::<Pos>().get(e).is_some());
}

#[test]
fn delete_twice() {
    let mut world = World::new();

    let e = world.create_entity().build();

    world.delete_entity(e).unwrap();
    assert!(world.entities().delete(e).is_err());
}

#[test]
fn test_bundle() {
    let mut world = World::new();

    pub struct SomeResource {
        pub v: u32,
    }

    pub struct TestBundle;

    impl Bundle for TestBundle {
        fn add_to_world(self, world: &mut World) {
            world.add_resource(SomeResource { v: 12 });
        }
    }

    world.add_bundle(TestBundle);
    assert_eq!(12, world.read_resource::<SomeResource>().v);
}
