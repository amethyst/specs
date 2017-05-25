extern crate specs;

use specs::prelude::*;

// A component contains data
// which is associated with an entity.

#[derive(Debug)]
struct Vel(f32);
#[derive(Debug)]
struct Pos(f32);

impl Component for Vel {
    type Storage = VecStorage<Vel>;
}

impl Component for Pos {
    type Storage = VecStorage<Pos>;
}

struct SysA;

impl<'a, C> System<'a, C> for SysA {
    // These are the resources required for execution.
    // You can also define a struct and `#[derive(SystemData)]`,
    // see the `full` example.
    type SystemData = (WriteStorage<'a, Pos>, ReadStorage<'a, Vel>);

    fn work(&mut self, data: Self::SystemData, _: C) {
        // The `.join()` combines multiple components,
        // so we only access those entities which have
        // both of them.

        let (mut pos, vel) = data;

        for (pos, vel) in (&mut pos, &vel).join() {
            pos.0 += vel.0;
        }
    }
}

fn main() {
    // The `World` is our
    // container for components
    // and other resources.

    let mut world = World::new();
    world.register::<Pos>();
    world.register::<Vel>();

    // An entity may or may not contain some component.

    world.create_entity().with(Vel(2.0)).with(Pos(0.0)).build();
    world.create_entity().with(Vel(4.0)).with(Pos(1.6)).build();
    world.create_entity().with(Vel(1.5)).with(Pos(5.4)).build();

    // This entity does not have `Vel`, so it won't be dispatched.
    world.create_entity().with(Pos(2.0)).build();

    // This builds a dispatcher.
    // The third parameter of `add` specifies
    // logical dependencies on other systems.
    // Since we only have one, we don't depend on anything.
    // See the `full` example for dependencies.
    let mut dispatcher = DispatcherBuilder::new().add(SysA, "sys_a", &[]).build();

    // We pass the resources and a context (`()`).
    // This dispatches all the systems in parallel (but blocking).
    dispatcher.dispatch(&mut world.res, ());
}
