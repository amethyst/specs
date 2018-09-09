extern crate specs;

use specs::prelude::*;

struct MyComp;

impl Component for MyComp {
    type Storage = FlaggedStorage<Self>;
}

#[test]
fn test_remove_insert() {
    let mut world = World::new();
    world.register::<MyComp>();

    struct Sys1;

    impl<'a> System<'a> for Sys1 {
        type SystemData = (Entities<'a>, WriteStorage<'a, MyComp>);

        fn run(&mut self, (ent, mut my_comp): Self::SystemData) {
            my_comp.insert(ent.create(), MyComp).unwrap();
        }
    }

    Sys1.run_now(&mut world.res);

    let mut insertions = world.write_storage::<MyComp>().track_inserted();
    let mut removals = world.write_storage::<MyComp>().track_removed();

    struct Sys2;

    impl<'a> System<'a> for Sys2 {
        type SystemData = (Entities<'a>, WriteStorage<'a, MyComp>);

        fn run(&mut self, (ent, mut my_comp): Self::SystemData) {
            let a = (&*ent).join().next().unwrap();

            my_comp.remove(a).unwrap();
            my_comp.insert(a, MyComp).unwrap();
        }
    }

    Sys2.run_now(&mut world.res);

    let s = world.write_storage::<MyComp>();

    s.inserted().read(&mut insertions).for_each(|x| println!("Insert: {:?}", x));
    s.removed().read(&mut removals).for_each(|x| println!("Remove: {:?}", x));

    panic!()
}

#[test]
fn test_insert_remove() {
    let mut world = World::new();
    world.register::<MyComp>();

    let mut insertions = world.write_storage::<MyComp>().track_inserted();
    let mut removals = world.write_storage::<MyComp>().track_removed();

    struct Sys1;

    impl<'a> System<'a> for Sys1 {
        type SystemData = (Entities<'a>, WriteStorage<'a, MyComp>);

        fn run(&mut self, (ent, mut my_comp): Self::SystemData) {
            let e = ent.create();
            my_comp.insert(e, MyComp).unwrap();
            my_comp.remove(e).unwrap();
        }
    }

    Sys1.run_now(&mut world.res);

    let s = world.write_storage::<MyComp>();

    s.inserted().read(&mut insertions).for_each(|x| println!("Insert: {:?}", x));
    s.removed().read(&mut removals).for_each(|x| println!("Remove: {:?}", x));

    panic!()
}