extern crate specs;

use specs::prelude::*;

// A component contains data which is associated with an entity.

#[derive(Debug)]
struct Vel(f32);

impl Component for Vel {
    type Storage = DefaultVecStorage<Self>;
}

impl Default for Vel {
    fn default() -> Self {
        Self(0.0)
    }
}

#[derive(Debug)]
struct Pos(f32);

impl Component for Pos {
    type Storage = DefaultVecStorage<Self>;
}

impl Default for Pos {
    fn default() -> Self {
        Self(0.0)
    }
}

struct SysA;

impl<'a> System<'a> for SysA {
    // These are the resources required for execution.
    // You can also define a struct and `#[derive(SystemData)]`,
    // see the `full` example.
    type SystemData = (WriteStorage<'a, Pos>, ReadStorage<'a, Vel>);

    fn run(&mut self, (mut pos, vel): Self::SystemData) {
        // Both the `Pos` and `Vel` components use `DefaultVecStorage`, which supports
        // `as_slice()` and `as_mut_slice()`. This lets us access components without
        // indirection.
        let mut pos_slice = pos.as_mut_slice();
        let vel_slice = vel.as_slice();

        // Note that an entity which has position but not velocity will still have
        // an entry in both slices. `DefaultVecStorage` is sparse, and here is where
        // that matters: the storage has space for many entities, but not all of them
        // contain meaningful values. These slices may be a mix of present and absent
        // (`Default`) data.
        //
        // We could check the `mask()` before reading the velocity and updating the
        // position, and this is what `.join()` normally does. However, because:
        //
        //   1. `Vel` uses `DefaultVecStorage`,
        //   2. `Vel`'s default is 0.0, and
        //   3. `Pos` += 0.0 is a no-op,
        //
        // we can unconditionally add `Vel` to `Pos` without reading the `mask()`!
        // This results in a tight inner loop of known size, which is especially
        // suitable for SIMD, OpenCL, CUDA, and other accelerator technologies.
        //
        // Finally, note that `DefaultVecStorage` and `VecStorage` slice indices
        // always agree. If an entity is at location `i` in one `VecStorage`, it
        // will be at location `i` in every other `VecStorage`. (By contrast,
        // `DenseVecStorage` uses unpredictable indices and cannot be used in
        // this way.) We need only worry about handling slices of different
        // lengths.
        let len = pos_slice.len().min(vel_slice.len());
        for i in 0..len {
            pos_slice[i].0 += vel_slice[i].0;
        }
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
    let mut dispatcher = DispatcherBuilder::new().with(SysA, "sys_a", &[]).build();

    // setup() must be called before creating any entity, it will register
    // all Components and Resources that Systems depend on
    dispatcher.setup(&mut world);

    // An entity may or may not contain some component.

    world.create_entity().with(Vel(2.0)).with(Pos(0.0)).build();
    world.create_entity().with(Vel(4.0)).with(Pos(1.6)).build();
    world.create_entity().with(Vel(1.5)).with(Pos(5.4)).build();

    // This entity does not have `Vel`, so it won't be dispatched.
    world.create_entity().with(Pos(2.0)).build();

    // This dispatches all the systems in parallel (but blocking).
    dispatcher.dispatch(&world);
}
