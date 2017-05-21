extern crate shred;
#[macro_use]
extern crate shred_derive;
extern crate specs;

use shred::{DispatcherBuilder, System};

use specs::{ReadStorage, WriteStorage, World};
use specs::entity::Component;
use specs::storages::VecStorage;

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

#[derive(SystemData)]
struct Data<'a> {
    vel: ReadStorage<'a, Vel>,
    pos: WriteStorage<'a, Pos>,
}

struct SysA;

impl<'a, C> System<'a, C> for SysA {
    type SystemData = Data<'a>;

    fn work(&mut self, mut data: Data, _: C) {
        use specs::Join;

        for (pos, vel) in (&mut data.pos, &data.vel).join() {
            pos.0 += vel.0;
        }
    }
}

fn main() {
    let mut world = World::new();
    world.register::<Pos>();
    world.register::<Vel>();

    world.create_entity().with(Vel(2.0)).with(Pos(0.0)).build();
    world.create_entity().with(Vel(4.0)).with(Pos(1.6)).build();
    world.create_entity().with(Vel(1.5)).with(Pos(5.4)).build();

    let mut dispatcher = DispatcherBuilder::new().add(SysA, "sys_a", &[]).finish();

    dispatcher.dispatch(&mut world.res, ());
}
