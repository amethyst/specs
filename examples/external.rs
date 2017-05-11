extern crate specs;


#[cfg(not(feature="parallel"))]
fn main() {
}

#[cfg(feature="parallel")]
fn main() {
    parallel::main()
}

#[cfg(feature = "parallel")]
mod parallel {
    use specs::{Component, ExternalSystem, Planner, VecStorage, World};

    struct SingleThreadedStuff {
        counter: u32,
        #[allow(unused)]
        something_unsafe: *const u8, // This makes the struct !Send
    }

    #[derive(Clone, Debug)]
    struct CompInt(u32);

    impl Component for CompInt {
        type Storage = VecStorage<CompInt>;
    }

    pub fn main() {
        let mut world = World::new();

        {
            world.register::<CompInt>();
            world.create_now().with(CompInt(4)).build();
            world.create_now().with(CompInt(7)).build();
        }

        let mut planner = Planner::<()>::new(world);


        let (ext, work) = ExternalSystem::new();

        planner.add_system(ext, "external_sys", 1);

        planner.dispatch(());

        let mut single_threaded = SingleThreadedStuff {
            counter: 0,
            something_unsafe: ::std::ptr::null(),
        };

        work.do_work(|arg, ()| {
            use specs::Join;

            let comp_int = arg.fetch(|w| w.read::<CompInt>());

            for i in (comp_int).join() {
                single_threaded.counter += i.0;
            }
        });

        assert_eq!(single_threaded.counter, 4 + 7);

        planner.wait();
    }
}
