extern crate specs;
#[macro_use]
extern crate specs_derive;

use specs::prelude::*;

// A component contains data which is associated with an entity.

#[derive(Debug)]
struct Vel(f32);

impl Component for Vel {
    type Storage = VecStorage<Self>;
}

#[derive(Debug)]
struct Pos(f32);

impl Component for Pos {
    type Storage = VecStorage<Self>;
}

/// Increments entities' position by their velocity.
///
/// The function parameters are the resources required for execution.
#[specs_system(SysA)]
fn sys_a(mut pos: WriteStorage<Pos>, vel: ReadStorage<Vel>) {
    for (pos, vel) in (&mut pos, &vel).join() {
        pos.0 += vel.0;
    }
}

#[specs_system(SysPrint)]
fn sys_print(entities: Entities, pos: ReadStorage<Pos>, vel: ReadStorage<Vel>) {
    for (e, pos, vel) in (&entities, &pos, &vel).join() {
        println!("{:?}: Pos: {:?}   Vel: {:?}", e, pos, vel);
    }
}

fn main() {
    // The `World` is our
    // container for components
    // and other resources.

    let mut world = World::new();

    // This builds a dispatcher.
    // The third parameter of `add` specifies
    // logical dependencies on other systems.
    // Since we only have one, we don't depend on anything.
    // See the `full` example for dependencies.
    let mut dispatcher = DispatcherBuilder::new()
        .with(SysA, "sys_a", &[])
        .with(SysPrint, "sys_print", &["sys_a"])
        .build();

    // setup() must be called before creating any entity, it will register
    // all Components and Resources that Systems depend on
    dispatcher.setup(&mut world.res);

    // An entity may or may not contain some component.

    world.create_entity().with(Vel(2.0)).with(Pos(0.0)).build();
    world.create_entity().with(Vel(4.0)).with(Pos(1.6)).build();
    world.create_entity().with(Vel(1.5)).with(Pos(5.4)).build();

    // This dispatches all the systems in parallel (but blocking).
    dispatcher.dispatch(&world.res);
    dispatcher.dispatch(&world.res);
}
