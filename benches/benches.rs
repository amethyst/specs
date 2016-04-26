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
        let w = specs::World::new();
        b.iter(|| w.create_now().build());
    }

    #[bench]
    fn create_now_with_storage(b: &mut test::Bencher) {
        let w = create_world();
        b.iter(|| w.create_now().with(CompInt(0)).build());
    }

    #[bench]
    fn create_later(b: &mut test::Bencher) {
        let w = specs::World::new();
        b.iter(|| w.create_later());
    }

    #[bench]
    fn delete_now(b: &mut test::Bencher) {
        let w = specs::World::new();
        let mut eids: Vec<_> = (0..1_000_000).map(|_| w.create_now().build()).collect();
        b.iter(|| w.delete_now(eids.pop().unwrap()));
    }

    #[bench]
    fn delete_now_with_storage(b: &mut test::Bencher) {
        let w = create_world();
        let mut eids: Vec<_> = (0..1_000_000).map(|_| w.create_now().with(CompInt(1)).build()).collect();
        b.iter(|| w.delete_now(eids.pop().unwrap()));
    }

    #[bench]
    fn delete_later(b: &mut test::Bencher) {
        let w = specs::World::new();
        let mut eids: Vec<_> = (0..1_000_000).map(|_| w.create_now().build()).collect();
        b.iter(|| w.delete_later(eids.pop().unwrap()));
    }

    #[bench]
    fn maintain_noop(b: &mut test::Bencher) {
        let w = specs::World::new();
        b.iter(|| {
            w.maintain();
        });
    }

    #[bench]
    fn maintain_add_later(b: &mut test::Bencher) {
        let w = specs::World::new();
        b.iter(|| {
            w.create_later();
            w.maintain();
        });
    }

    #[bench]
    fn maintain_delete_later(b: &mut test::Bencher) {
        let w = specs::World::new();
        let mut eids: Vec<_> = (0..1_000_000).map(|_| w.create_now().build()).collect();
        b.iter(|| {
            w.delete_later(eids.pop().unwrap());
            w.maintain();
        });
    }
}

mod bitset {
    use test;
    use specs::BitSet;

    #[bench]
    fn add(b: &mut test::Bencher) {
        let mut bitset = BitSet::with_capacity(1_000_000);
        let mut range = (0..1_000_000).cycle();
        b.iter(|| range.next().map(|i| bitset.add(i)))
    }

    #[bench]
    fn remove_set(b: &mut test::Bencher) {
        let mut bitset = BitSet::with_capacity(1_000_000);
        let mut range = (0..1_000_000).cycle();
        for i in 0..1_000_000 {
            bitset.add(i);
        }
        b.iter(|| range.next().map(|i| bitset.remove(i)))
    }

    #[bench]
    fn remove_clear(b: &mut test::Bencher) {
        let mut bitset = BitSet::with_capacity(1_000_000);
        let mut range = (0..1_000_000).cycle();
        b.iter(|| range.next().map(|i| bitset.remove(i)))
    }

    #[bench]
    fn contains(b: &mut test::Bencher) {
        let mut bitset = BitSet::with_capacity(1_000_000);
        let mut range = (0..1_000_000).cycle();
        for i in 0..500_000 {
            // events are set, odds are to keep the branch
            // prediction from getting to aggressive
            bitset.add(i * 2);
        }
        b.iter(|| range.next().map(|i| bitset.contains(i)))
    }
}

mod atomic_bitset {
    use test;
    use specs::AtomicBitSet;

    #[bench]
    fn add(b: &mut test::Bencher) {
        let mut bitset = AtomicBitSet::new();
        let mut range = (0..1_000_000).cycle();
        b.iter(|| range.next().map(|i| bitset.add(i)))
    }

    #[bench]
    fn add_atomic(b: &mut test::Bencher) {
        let bitset = AtomicBitSet::new();
        let mut range = (0..1_000_000).cycle();
        b.iter(|| range.next().map(|i| bitset.add_atomic(i)))
    }

    #[bench]
    fn remove_set(b: &mut test::Bencher) {
        let mut bitset = AtomicBitSet::new();
        let mut range = (0..1_000_000).cycle();
        for i in 0..1_000_000 {
            bitset.add(i);
        }
        b.iter(|| range.next().map(|i| bitset.remove(i)))
    }

    #[bench]
    fn remove_clear(b: &mut test::Bencher) {
        let mut bitset = AtomicBitSet::new();
        let mut range = (0..1_000_000).cycle();
        b.iter(|| range.next().map(|i| bitset.remove(i)))
    }

    #[bench]
    fn contains(b: &mut test::Bencher) {
        let mut bitset = AtomicBitSet::new();
        let mut range = (0..1_000_000).cycle();
        for i in 0..500_000 {
            // events are set, odds are to keep the branch
            // prediction from getting to aggressive
            bitset.add(i * 2);
        }
        b.iter(|| range.next().map(|i| bitset.contains(i)))
    }
}