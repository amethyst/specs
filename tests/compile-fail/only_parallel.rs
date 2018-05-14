extern crate specs;
#[macro_use]
extern crate specs_derive;

use specs::prelude::*;

#[derive(Component)]
struct Comp1;

#[derive(Component)]
struct Comp2;

fn fail_double_borrow() {
    let mut world = World::new();

    let e = world.create_entity().build();

    world.exec(|(mut s1, mut s2): (WriteStorage<Comp1>, WriteStorage<Comp2>)| {
        (&mut s1)
            .par_join()
            .for_each(|_| {
                s2.get_mut(e);
                //~^ cannot borrow data mutably in a captured outer variable
            });
    });
}
