use specs::prelude::*;
struct Pos(f32);

impl Component for Pos {
    type Storage = VecStorage<Self>;
}

fn main() {
    let mut world = World::new();

    world.register::<Pos>();

    world.create_entity().with(Pos(0.0)).build();
    world.create_entity().with(Pos(1.6)).build();
    world.create_entity().with(Pos(5.4)).build();

    let mut pos = world.write_storage::<Pos>();

    let mut lending = (&mut pos).lend_join();

    let a = lending.next().unwrap().0;
    let b = lending.next().unwrap();
    // let d = lending.next().unwrap(); (this rightly fails to compile)
    let _c = a + b.0;
}
