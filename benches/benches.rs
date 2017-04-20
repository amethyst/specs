#![feature(test)]
extern crate test;
extern crate specs;

mod world {
    use test;
    use specs;

    #[derive(Clone, Debug)]
    struct CompInt(i32);
    impl specs::Component for CompInt {
        type Storage = specs::VecStorage<CompInt>;
    }
    #[derive(Clone, Debug)]
    struct CompBool(bool);
    impl specs::Component for CompBool {
        type Storage = specs::HashMapStorage<CompBool>;
    }

    fn create_world() -> specs::World {
        let mut w = specs::World::new();
        w.register::<CompInt>();
        w.register::<CompBool>();
        w
    }

    #[bench]
    fn world_build(b: &mut test::Bencher) {
        b.iter(|| specs::World::new());
    }

    #[bench]
    fn create_now(b: &mut test::Bencher) {
        let mut w = specs::World::new();
        b.iter(|| w.create_now().build());
    }

    #[bench]
    fn create_now_with_storage(b: &mut test::Bencher) {
        let mut w = create_world();
        b.iter(|| w.create_now().with(CompInt(0)).build());
    }

    #[bench]
    fn create_pure(b: &mut test::Bencher) {
        let w = specs::World::new();
        b.iter(|| w.create_pure());
    }

    #[bench]
    fn delete_now(b: &mut test::Bencher) {
        let mut w = specs::World::new();
        let mut eids: Vec<_> = (0..10_000_000).map(|_| w.create_now().build()).collect();
        b.iter(|| {
            if let Some(id) = eids.pop() {
                w.delete_now(id)
            }
        });
    }

    #[bench]
    fn delete_now_with_storage(b: &mut test::Bencher) {
        let mut w = create_world();
        let mut eids: Vec<_> = (0..10_000_000).map(|_| w.create_now().with(CompInt(1)).build()).collect();
        b.iter(|| {
            if let Some(id) = eids.pop() {
                w.delete_now(id)
            }
        });
    }

    #[bench]
    fn delete_later(b: &mut test::Bencher) {
        let mut w = specs::World::new();
        let mut eids: Vec<_> = (0..10_000_000).map(|_| w.create_now().build()).collect();
        b.iter(|| {
            if let Some(id) = eids.pop() {
                w.delete_later(id)
            }
        });
    }

    #[bench]
    fn maintain_noop(b: &mut test::Bencher) {
        let mut w = specs::World::new();
        b.iter(|| {
            w.maintain();
        });
    }

    #[bench]
    fn maintain_add_later(b: &mut test::Bencher) {
        let mut w = specs::World::new();
        b.iter(|| {
            w.create_pure();
            w.maintain();
        });
    }

    #[bench]
    fn maintain_delete_later(b: &mut test::Bencher) {
        let mut w = specs::World::new();
        let mut eids: Vec<_> = (0..10_000_000).map(|_| w.create_now().build()).collect();
        b.iter(|| {
            if let Some(id) = eids.pop() {
                w.delete_later(id);
            }
            w.maintain();
        });
    }
}
