extern crate specs;

use specs::{Component, RunArg, Planner, System, VecStorage, World};

struct Vel(f32);
struct Pos(f32);

impl Component for Vel {
    type Storage = VecStorage<Vel>;
}

impl Component for Pos {
    type Storage = VecStorage<Pos>;
}

fn main() {
    let mut planner = {
        let mut world = World::new();
        world.register::<Pos>();
        world.register::<Vel>();
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
