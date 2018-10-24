extern crate specs;
#[macro_use]
extern crate specs_derive;

use specs::prelude::*;

#[specs_system(SysPrint)]
fn sys_print() {
    println!("Yo! I'm SysPrint");
}

fn main() {
    let mut world = World::new();

    // Create a dispatcher with the generated system.
    let mut dispatcher = DispatcherBuilder::new().with(SysPrint, "sys_print", &[]).build();

    dispatcher.setup(&mut world.res);

    // This dispatches all the systems in parallel (but blocking).
    dispatcher.dispatch(&world.res);
}
