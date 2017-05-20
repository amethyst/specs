extern crate specs;

use specs::{Component, VecStorage, World};

struct Vel(f32);
struct Pos(f32);

impl Component for Vel {
    type Storage = VecStorage<Vel>;
}

impl Component for Pos {
    type Storage = VecStorage<Pos>;
}

#[cfg(not(feature="parallel"))]
fn main() {}

#[cfg(feature="parallel")]
fn main() {
    use specs::{RunArg, Planner, System};

    let mut planner = {
        let mut world = World::new();
        world.register::<Pos>();
        world.register::<Vel>();

        world.create_now().with(Vel(2.0)).with(Pos(0.0)).build();
        world.create_now().with(Vel(4.0)).with(Pos(1.6)).build();
        world.create_now().with(Vel(1.5)).with(Pos(5.4)).build();

        Planner::new(world)
    };

    struct SysA;

    impl System<()> for SysA {
        fn run(&mut self, arg: RunArg, _: ()) {
            use specs::{Gate, Join};

            let (pos, vel) = arg.fetch(|w| (w.write::<Pos>(), w.read::<Vel>()));

            for (pos, vel) in (&mut pos.pass(), &vel.pass()).join() {
                pos.0 += vel.0;
            }
        }
    }

    planner.add_system(SysA, "a", 1);
    planner.dispatch(());
}
