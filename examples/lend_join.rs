use specs::prelude::*;
struct Pos(f32);

impl Component for Pos {
    type Storage = VecStorage<Self>;
}

fn main() {
    let mut world = World::new();

    world.register::<Pos>();

    let entity0 = world.create_entity().with(Pos(0.0)).build();
    world.create_entity().with(Pos(1.6)).build();
    world.create_entity().with(Pos(5.4)).build();

    let mut pos = world.write_storage::<Pos>();
    let entities = world.entities();

    // Unlike `join` the type return from `lend_join` does not implement
    // `Iterator`. Instead, a `next` method is provided that only allows one
    // element to be accessed at once.
    let mut lending = (&mut pos).lend_join();

    // We copy the value out here so the borrow of `lending` is released.
    let a = lending.next().unwrap().0;
    // Here we keep the reference from `lending.next()` alive, so `lending`
    // remains exclusively borrowed for the lifetime of `b`.
    let b = lending.next().unwrap();
    // This right fails to compile since `b` is used below:
    // let d = lending.next().unwrap();
    b.0 = a;

    // Items can be iterated with `while let` loop:
    let mut lending = (&mut pos).lend_join();
    while let Some(pos) = lending.next() {
        pos.0 *= 1.5;
    }

    // A `for_each` method is also available:
    (&mut pos).lend_join().for_each(|pos| {
        pos.0 += 1.0;
    });

    // Finally, there is one bonus feature which `.join()` can't soundly provide.
    let mut lending = (&mut pos).lend_join();
    // That is, there is a method to get the joined result for a particular
    // entity:
    if let Some(pos) = lending.get(entity0, &entities) {
        pos.0 += 5.0;
    }
}
