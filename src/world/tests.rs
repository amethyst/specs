use super::{WorldExt, *};
use crate::{join::Join, storage::VecStorage};

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
    assert!(world.read_storage::<Pos>().get(b).is_none());
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
    assert!(world.read_storage::<Pos>().get(e1).is_some());
    assert!(world.read_storage::<Vel>().get(e1).is_some());
    assert!(world.read_storage::<Vel>().get(e2).is_some());
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
    assert!(world.read_storage::<Pos>().get(e).is_none());
}

#[test]
fn super_lazy_execution() {
    let mut world = World::new();
    world.register::<Pos>();

    let e = {
        let entity_res = world.read_resource::<EntitiesRes>();
        entity_res.create()
    };
    world.read_resource::<LazyUpdate>().exec(move |world| {
        world.read_resource::<LazyUpdate>().exec(move |world| {
            if let Err(err) = world.write_storage::<Pos>().insert(e, Pos) {
                panic!("Unable to lazily insert component! {:?}", err);
            }
        });
        assert!(world.read_storage::<Pos>().get(e).is_none());
    });
    world.maintain();
    assert!(world.read_storage::<Pos>().get(e).is_some());
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
        lazy.exec(move |world| {
            if let Err(err) = world.write_storage::<Pos>().insert(e, Pos) {
                panic!("Unable to lazily insert component! {:?}", err);
            }
        });
    }

    world.maintain();
    assert!(world.read_storage::<Pos>().get(e).is_some());
}

#[test]
fn lazy_execution_order() {
    let mut world = World::new();
    world.insert(Vec::<u32>::new());
    {
        let lazy = world.read_resource::<LazyUpdate>();
        lazy.exec(move |world| {
            let mut v = world.write_resource::<Vec<u32>>();
            v.push(1);
        });
        lazy.exec(move |world| {
            let mut v = world.write_resource::<Vec<u32>>();
            v.push(2);
        });
    }
    world.maintain();
    let v = world.read_resource::<Vec<u32>>();
    assert_eq!(&**v, &[1, 2]);
}

#[test]
fn delete_twice() {
    let mut world = World::new();

    let e = world.create_entity().build();

    world.delete_entity(e).unwrap();
    assert!(world.entities().delete(e).is_err());
}

#[test]
fn delete_and_lazy() {
    let mut world = World::new();
    {
        let lazy_update = world.write_resource::<crate::LazyUpdate>();
        lazy_update.exec(|world| {
            world.entities().create();
        })
    }

    world.maintain();
    {
        let lazy_update = world.write_resource::<crate::LazyUpdate>();
        lazy_update.exec(|world| {
            world.entities().create();
        })
    }

    world.delete_all();
}
