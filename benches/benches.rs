#![feature(test)]
extern crate test;
extern crate specs;

mod world {
    use test;
    use specs::{Component, World};
    use specs::storages::{HashMapStorage, VecStorage};

    #[derive(Clone, Debug)]
    struct CompInt(i32);
    impl Component for CompInt {
        type Storage = VecStorage<CompInt>;
    }

    #[derive(Clone, Debug)]
    struct CompBool(bool);
    impl Component for CompBool {
        type Storage = HashMapStorage<CompBool>;
    }

    fn create_world() -> World {
        let mut w = World::new();

        w.register::<CompInt>();
        w.register::<CompBool>();

        w
    }

    #[bench]
    fn world_build(b: &mut test::Bencher) {
        b.iter(|| World::new());
    }

    #[bench]
    fn create_now(b: &mut test::Bencher) {
        let mut w = World::new();
        b.iter(|| w.create_entity().build());
    }

    #[bench]
    fn create_now_with_storage(b: &mut test::Bencher) {
        let mut w = create_world();
        b.iter(|| w.create_entity().with(CompInt(0)).build());
    }

    #[bench]
    fn create_pure(b: &mut test::Bencher) {
        let w = World::new();
        b.iter(|| w.entities().create());
    }

    #[bench]
    fn delete_now(b: &mut test::Bencher) {
        let mut w = World::new();
        let mut eids: Vec<_> = (0..10_000_000).map(|_| w.create_entity().build()).collect();
        b.iter(|| if let Some(id) = eids.pop() {
                   w.delete_entity(id)
               });
    }

    #[bench]
    fn delete_now_with_storage(b: &mut test::Bencher) {
        let mut w = create_world();
        let mut eids: Vec<_> = (0..10_000_000)
            .map(|_| w.create_entity().with(CompInt(1)).build())
            .collect();
        b.iter(|| if let Some(id) = eids.pop() {
                   w.delete_entity(id)
               });
    }

    #[bench]
    fn delete_later(b: &mut test::Bencher) {
        let mut w = World::new();
        let mut eids: Vec<_> = (0..10_000_000).map(|_| w.create_entity().build()).collect();
        b.iter(|| if let Some(id) = eids.pop() {
                   w.entities().delete(id)
               });
    }

    #[bench]
    fn maintain_noop(b: &mut test::Bencher) {
        let mut w = World::new();
        b.iter(|| { w.maintain(); });
    }

    #[bench]
    fn maintain_add_later(b: &mut test::Bencher) {
        let mut w = World::new();
        b.iter(|| {
                   w.entities().create();
                   w.maintain();
               });
    }

    #[bench]
    fn maintain_delete_later(b: &mut test::Bencher) {
        let mut w = World::new();
        let mut eids: Vec<_> = (0..10_000_000).map(|_| w.create_entity().build()).collect();
        b.iter(|| {
                   if let Some(id) = eids.pop() {
                       w.entities().delete(id);
                   }
                   w.maintain();
               });
    }
}
