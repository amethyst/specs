use mopa::Any;

use super::*;
use {Component, Entity, Generation, Index, World};

fn create<T: Component>(world: &mut World) -> WriteStorage<T> {
    world.register::<T>();

    world.write()
}

mod map_test {
    use super::*;

    #[derive(Debug)]
    struct Comp<T>(T);
    impl<T: Any + Send + Sync> Component for Comp<T> {
        type Storage = VecStorage<Self>;
    }

    fn ent(i: Index) -> Entity {
        Entity::new(i, Generation::new(1))
    }

    #[test]
    fn insert() {
        let mut w = World::new();
        let mut c = create(&mut w);

        for i in 0..1_000 {
            c.insert(ent(i), Comp(i));
        }

        for i in 0..1_000 {
            assert_eq!(c.get(ent(i)).unwrap().0, i);
        }
    }

    #[test]
    fn insert_100k() {
        let mut w = World::new();
        let mut c = create(&mut w);

        for i in 0..100_000 {
            c.insert(ent(i), Comp(i));
        }

        for i in 0..100_000 {
            assert_eq!(c.get(ent(i)).unwrap().0, i);
        }
    }

    #[test]
    fn remove() {
        let mut w = World::new();
        let mut c = create(&mut w);

        for i in 0..1_000 {
            c.insert(ent(i), Comp(i));
        }

        for i in 0..1_000 {
            assert_eq!(c.get(ent(i)).unwrap().0, i);
        }

        for i in 0..1_000 {
            c.remove(ent(i));
        }

        for i in 0..1_000 {
            assert!(c.get(ent(i)).is_none());
        }
    }

    #[test]
    fn test_gen() {
        let mut w = World::new();
        let mut c = create(&mut w);

        for i in 0..1_000i32 {
            c.insert(ent(i as u32), Comp(i));
            c.insert(ent(i as u32), Comp(-i));
        }

        for i in 0..1_000i32 {
            assert_eq!(c.get(ent(i as u32)).unwrap().0, -i);
        }
    }

    #[test]
    fn insert_same_key() {
        let mut w = World::new();
        let mut c = create(&mut w);

        for i in 0..10_000 {
            c.insert(ent(i), Comp(i));
            assert_eq!(c.get(ent(i)).unwrap().0, i);
        }
    }

    #[should_panic]
    #[test]
    fn wrap() {
        let mut w = World::new();
        let mut c = create(&mut w);

        c.insert(ent(1 << 25), Comp(7));
    }
}

mod test {
    use std::convert::AsMut;
    use std::fmt::Debug;

    use super::*;

    #[derive(PartialEq, Eq, Debug, Default)]
    struct CMarker;
    impl Component for CMarker {
        type Storage = NullStorage<Self>;
    }

    #[derive(PartialEq, Eq, Debug)]
    struct Cvec(u32);
    impl From<u32> for Cvec {
        fn from(v: u32) -> Cvec {
            Cvec(v)
        }
    }
    impl AsMut<u32> for Cvec {
        fn as_mut(&mut self) -> &mut u32 {
            &mut self.0
        }
    }
    impl Component for Cvec {
        type Storage = VecStorage<Self>;
    }

    #[derive(PartialEq, Eq, Debug)]
    struct FlaggedCvec(u32);
    impl From<u32> for FlaggedCvec {
        fn from(v: u32) -> FlaggedCvec {
            FlaggedCvec(v)
        }
    }
    impl AsMut<u32> for FlaggedCvec {
        fn as_mut(&mut self) -> &mut u32 {
            &mut self.0
        }
    }
    impl Component for FlaggedCvec {
        type Storage = FlaggedStorage<Self, VecStorage<Self>>;
    }

    #[derive(PartialEq, Eq, Debug)]
    struct Cmap(u32);
    impl From<u32> for Cmap {
        fn from(v: u32) -> Cmap {
            Cmap(v)
        }
    }
    impl AsMut<u32> for Cmap {
        fn as_mut(&mut self) -> &mut u32 {
            &mut self.0
        }
    }
    impl Component for Cmap {
        type Storage = HashMapStorage<Self>;
    }

    #[derive(PartialEq, Eq, Debug)]
    struct CBtree(u32);
    impl From<u32> for CBtree {
        fn from(v: u32) -> CBtree {
            CBtree(v)
        }
    }
    impl AsMut<u32> for CBtree {
        fn as_mut(&mut self) -> &mut u32 {
            &mut self.0
        }
    }
    impl Component for CBtree {
        type Storage = BTreeStorage<Self>;
    }

    #[derive(PartialEq, Eq, Debug)]
    #[cfg(feature = "rudy")]
    struct CRudy(u32);
    #[cfg(feature = "rudy")]
    impl From<u32> for CRudy {
        fn from(v: u32) -> CRudy {
            CRudy(v)
        }
    }
    #[cfg(feature = "rudy")]
    impl AsMut<u32> for CRudy {
        fn as_mut(&mut self) -> &mut u32 {
            &mut self.0
        }
    }
    #[cfg(feature = "rudy")]
    impl Component for CRudy {
        type Storage = RudyStorage<Self>;
    }

    #[derive(Debug, Default, PartialEq)]
    struct Cnull;

    impl From<u32> for Cnull {
        fn from(_: u32) -> Self {
            Cnull
        }
    }

    impl Component for Cnull {
        type Storage = NullStorage<Self>;
    }

    fn test_add<T: Component + From<u32> + Debug + Eq>() {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation::new(1)), (i + 2718).into());
        }

        for i in 0..1_000 {
            assert_eq!(
                s.get(Entity::new(i, Generation::new(1))).unwrap(),
                &(i + 2718).into()
            );
        }
    }

    fn test_sub<T: Component + From<u32> + Debug + Eq>() {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation::new(1)), (i + 2718).into());
        }

        for i in 0..1_000 {
            assert_eq!(
                s.remove(Entity::new(i, Generation::new(1))).unwrap(),
                (i + 2718).into()
            );
            assert!(s.remove(Entity::new(i, Generation::new(1))).is_none());
        }
    }

    fn test_get_mut<T: Component + From<u32> + AsMut<u32> + Debug + Eq>() {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation::new(1)), (i + 2718).into());
        }

        for i in 0..1_000 {
            *s.get_mut(Entity::new(i, Generation::new(1)))
                .unwrap()
                .as_mut() -= 718;
        }

        for i in 0..1_000 {
            assert_eq!(
                s.get(Entity::new(i, Generation::new(1))).unwrap(),
                &(i + 2000).into()
            );
        }
    }

    fn test_add_gen<T: Component + From<u32> + Debug + Eq>() {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation::new(1)), (i + 2718).into());
            s.insert(Entity::new(i, Generation::new(2)), (i + 31415).into());
        }

        for i in 0..1_000 {
            assert!(s.get(Entity::new(i, Generation::new(2))).is_none());
            assert_eq!(
                s.get(Entity::new(i, Generation::new(1))).unwrap(),
                &(i + 2718).into()
            );
        }
    }

    fn test_sub_gen<T: Component + From<u32> + Debug + Eq>() {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation::new(2)), (i + 2718).into());
        }

        for i in 0..1_000 {
            assert!(s.remove(Entity::new(i, Generation::new(1))).is_none());
        }
    }

    fn test_clear<T: Component + From<u32>>() {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        for i in 0..10 {
            s.insert(Entity::new(i, Generation::new(1)), (i + 10).into());
        }

        s.clear();

        for i in 0..10 {
            assert!(s.get(Entity::new(i, Generation::new(1))).is_none());
        }
    }

    fn test_anti<T: Component + From<u32> + Debug + Eq>() {
        use join::Join;

        let mut w = World::new();
        let mut s: Storage<T, _> = create::<T>(&mut w);

        for i in 0..10 {
            s.insert(Entity::new(i, Generation::new(1)), (i + 10).into());
        }

        for (i, (a, _)) in (&s, !&s).join().take(10).enumerate() {
            assert_eq!(a, &(i as u32).into());
        }
    }

    #[test]
    fn vec_test_add() {
        test_add::<Cvec>();
    }
    #[test]
    fn vec_test_sub() {
        test_sub::<Cvec>();
    }
    #[test]
    fn vec_test_get_mut() {
        test_get_mut::<Cvec>();
    }
    #[test]
    fn vec_test_add_gen() {
        test_add_gen::<Cvec>();
    }
    #[test]
    fn vec_test_sub_gen() {
        test_sub_gen::<Cvec>();
    }
    #[test]
    fn vec_test_clear() {
        test_clear::<Cvec>();
    }
    #[test]
    fn vec_test_anti() {
        test_anti::<Cvec>();
    }

    #[test]
    fn vec_arc() {
        use std::sync::Arc;

        #[derive(Debug)]
        struct A(Arc<()>);

        let mut storage = VecStorage::<A>::default();

        unsafe {
            for i in (0..200).filter(|i| i % 2 != 0) {
                storage.insert(i, A(Arc::new(())));
            }
            storage.clean(|i| i % 2 != 0);
        }
    }

    #[test]
    fn hash_test_add() {
        test_add::<Cmap>();
    }
    #[test]
    fn hash_test_sub() {
        test_sub::<Cmap>();
    }
    #[test]
    fn hash_test_get_mut() {
        test_get_mut::<Cmap>();
    }
    #[test]
    fn hash_test_add_gen() {
        test_add_gen::<Cmap>();
    }
    #[test]
    fn hash_test_sub_gen() {
        test_sub_gen::<Cmap>();
    }
    #[test]
    fn hash_test_clear() {
        test_clear::<Cmap>();
    }

    #[test]
    fn btree_test_add() {
        test_add::<CBtree>();
    }
    #[test]
    fn btree_test_sub() {
        test_sub::<CBtree>();
    }
    #[test]
    fn btree_test_get_mut() {
        test_get_mut::<CBtree>();
    }
    #[test]
    fn btree_test_add_gen() {
        test_add_gen::<CBtree>();
    }
    #[test]
    fn btree_test_sub_gen() {
        test_sub_gen::<CBtree>();
    }
    #[test]
    fn btree_test_clear() {
        test_clear::<CBtree>();
    }

    #[cfg(feature = "rudy")]
    #[test]
    fn rudy_test_add() {
        test_add::<CRudy>();
    }
    #[cfg(feature = "rudy")]
    #[test]
    fn rudy_test_sub() {
        test_sub::<CRudy>();
    }
    #[cfg(feature = "rudy")]
    #[test]
    fn rudy_test_get_mut() {
        test_get_mut::<CRudy>();
    }
    #[cfg(feature = "rudy")]
    #[test]
    fn rudy_test_add_gen() {
        test_add_gen::<CRudy>();
    }
    #[cfg(feature = "rudy")]
    #[test]
    fn rudy_test_sub_gen() {
        test_sub_gen::<CRudy>();
    }
    #[cfg(feature = "rudy")]
    #[test]
    fn rudy_test_clear() {
        test_clear::<CRudy>();
    }

    #[test]
    fn dummy_test_clear() {
        test_clear::<Cnull>();
    }

    #[test]
    fn test_null_insert_twice() {
        let mut w = World::new();

        w.register::<Cnull>();
        let e = w.create_entity().build();

        let mut null = w.write::<Cnull>();

        assert_eq!(null.get(e), None);
        assert_eq!(null.insert(e, Cnull), InsertResult::Inserted);
        assert_eq!(null.insert(e, Cnull), InsertResult::Updated(Cnull));
    }

    #[test]
    fn restricted_storage() {
        use join::Join;
        use std::collections::HashSet;

        let mut w = World::new();
        w.register::<Cvec>();
        let mut s1: Storage<Cvec, _> = w.write();
        let mut components = HashSet::new();

        for i in 0..50 {
            let c = i + 10;
            s1.insert(Entity::new(i, Generation::new(1)), c.into());
            components.insert(c);
        }

        for (entry, restricted) in (&mut s1.restrict()).join() {
            let c1 = { restricted.get_unchecked(&entry).0 };

            let c2 = { restricted.get_mut_unchecked(&entry).0 };

            assert_eq!(
                c1,
                c2,
                "Mutable and immutable gets returned different components."
            );
            assert!(
                components.remove(&c1),
                "Same component was iterated twice in join."
            );
        }
        assert!(
            components.is_empty(),
            "Some components weren't iterated in join."
        );
    }

    #[test]
    fn par_restricted_storage() {
        use join::ParJoin;
        use std::sync::Mutex;
        use std::collections::HashSet;
        use rayon::iter::ParallelIterator;

        let mut w = World::new();
        w.register::<Cvec>();
        let mut s1: Storage<Cvec, _> = w.write();
        let mut components = HashSet::new();

        for i in 0..50 {
            let c = i + 10;
            s1.insert(Entity::new(i, Generation::new(1)), c.into());
            components.insert(c);
        }

        let components2 = Mutex::new(Vec::new());
        let components2_mut = Mutex::new(Vec::new());

        (&mut s1.par_restrict())
            .par_join()
            .for_each(|(entry, restricted)| {
                let (mut components2, mut components2_mut) =
                    (components2.lock().unwrap(), components2_mut.lock().unwrap());
                components2.push(restricted.get_unchecked(&entry).0);
                components2_mut.push(restricted.get_mut_unchecked(&entry).0);
            });
        let components2 = components2.into_inner().unwrap();
        assert_eq!(
            components2,
            components2_mut.into_inner().unwrap(),
            "Mutable and immutable gets returned different components."
        );
        assert_eq!(
            components,
            components2.into_iter().collect(),
            "Components iterated weren't as should've been."
        );
    }

    #[test]
    #[should_panic]
    fn wrong_storage() {
        use join::Join;
        let mut w = World::new();
        w.register_with_id::<Cvec>(1);
        w.register_with_id::<Cvec>(2);
        let mut s1: Storage<Cvec, _> = w.write_with_id(1);
        // Possibility if the world uses dynamic components.
        let mut s2: Storage<Cvec, _> = w.write_with_id(2);

        for i in 0..50 {
            s1.insert(Entity::new(i, Generation::new(1)), (i + 10).into());
            s2.insert(Entity::new(i, Generation::new(1)), (i + 10).into());
        }
        for ((s1_entry, _), (_, s2_restricted)) in (&mut s1.restrict(), &mut s2.restrict()).join() {
            // verify that the assert fails if the storage is not the original.
            s2_restricted.get_unchecked(&s1_entry);
        }
    }

    #[test]
    #[should_panic]
    fn par_wrong_storage() {
        use join::ParJoin;
        use rayon::iter::ParallelIterator;

        let mut w = World::new();
        w.register_with_id::<Cvec>(1);
        w.register_with_id::<Cvec>(2);
        let mut s1: Storage<Cvec, _> = w.write_with_id(1);
        // Possibility if the world uses dynamic components.
        let mut s2: Storage<Cvec, _> = w.write_with_id(2);

        for i in 0..50 {
            s1.insert(Entity::new(i, Generation::new(1)), (i + 10).into());
            s2.insert(Entity::new(i, Generation::new(1)), (i + 10).into());
        }
        (&mut s1.par_restrict(), &mut s2.par_restrict())
            .par_join()
            .for_each(|((s1_entry, _), (_, s2_restricted))| {
                // verify that the assert fails if the storage is not the original.
                s2_restricted.get_unchecked(&s1_entry);
            });
    }

    #[test]
    fn check_storage() {
        use join::Join;

        let mut w = World::new();
        w.register::<CMarker>();
        let mut s1: Storage<CMarker, _> = w.write();

        for i in 0..50 {
            s1.insert(Entity::new(i, Generation::new(1)), CMarker);
        }

        for (entity, id) in (&*w.entities(), &s1.check()).join() {
            if id % 3 == 0 {
                let _ = s1.get_mut(entity);
            } else {
                let _ = s1.get(entity);
            }
        }

        assert_eq!((&s1.check()).join().count(), 50);
    }

    #[test]
    fn par_check_storage() {
        use join::ParJoin;
        use rayon::iter::ParallelIterator;

        let mut w = World::new();
        w.register::<CMarker>();
        let mut s1: Storage<CMarker, _> = w.write();

        for i in 0..50 {
            s1.insert(Entity::new(i, Generation::new(1)), CMarker);
        }

        assert_eq!((&s1.check()).par_join().count(), 50);
    }

    #[test]
    fn flagged() {
        use join::Join;
        use world::EntityIndex;

        let mut w = World::new();
        w.register_with_id::<FlaggedCvec>(1);
        w.register_with_id::<FlaggedCvec>(2);

        let entities = &*w.entities();
        let mut s1: Storage<FlaggedCvec, _> = w.write_with_id(1);
        let mut s2: Storage<FlaggedCvec, _> = w.write_with_id(2);

        for i in 0..15 {
            // Test insertion flagging
            s1.insert(Entity::new(i, Generation::new(1)), i.into());
            assert!(s1.open().1.flagged(Entity::new(i, Generation::new(1))));

            if i % 2 == 0 {
                s2.insert(Entity::new(i, Generation::new(1)), i.into());
                assert!(s2.open().1.flagged(Entity::new(i, Generation::new(1))));
            }
        }

        (&mut s1).open().1.clear_flags();

        // Cleared flags
        for (entity, _) in (entities, &s1.check()).join() {
            assert!(!s1.open().1.flagged(&entity));
        }

        // Modify components to flag.
        for (c1, c2) in (&mut s1, &s2).join() {
            println!("{:?} {:?}", c1, c2);
            c1.0 += c2.0;
        }

        for (entity, _) in (entities, &s1.check()).join() {
            // Should only be modified if the entity had both components
            // Which means only half of them should have it.
            if s1.open().1.flagged(&entity) {
                println!("Flagged: {:?}", entity.index());
                // Only every other component was flagged.
                assert!(entity.index() % 2 == 0);
            }
        }

        // Iterate over all flagged entities.
        for (entity, _) in (&*w.entities(), s1.open().1).join() {
            // All entities in here should be flagged.
            assert!(s1.open().1.flagged(&entity));
        }
    }
}

#[cfg(feature = "serde")]
mod serialize_test {
    extern crate serde_json;

    use super::{Component, Join, PackedData, VecStorage};
    use world::World;

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct CompTest {
        field1: u32,
        field2: bool,
    }
    impl Component for CompTest {
        type Storage = VecStorage<Self>;
    }

    #[test]
    fn serialize_storage() {
        // set up
        let mut world = World::new();
        world.register::<CompTest>();
        world.create_entity().build();
        world
            .create_entity()
            .with(CompTest {
                field1: 0,
                field2: true,
            })
            .build();
        world.create_entity().build();
        world.create_entity().build();
        world
            .create_entity()
            .with(CompTest {
                field1: 158123,
                field2: false,
            })
            .build();
        world
            .create_entity()
            .with(CompTest {
                field1: u32::max_value(),
                field2: false,
            })
            .build();
        world.create_entity().build();

        let storage = world.read::<CompTest>();
        let serialized = serde_json::to_string(&storage).unwrap();
        assert_eq!(
            serialized,
            r#"{"offsets":[1,4,5],"components":[{"field1":0,"field2":true},"#.to_owned() +
                r#"{"field1":158123,"field2":false},{"field1":4294967295,"field2":false}]}"#
        );
    }

    #[test]
    fn deserialize_storage() {
        // set up

        let mut world = World::new();
        world.register::<CompTest>();
        let entities: Vec<_> = world.entities().create_iter().take(10).collect();

        let data = r#"
            {
                "offsets":[3,7,8],
                "components": [
                    {
                        "field1":0,
                        "field2":true
                    },
                    {
                        "field1":158123,
                        "field2":false
                    },
                    {
                        "field1":4294967295,
                        "field2":false
                    }
                ]
            }
        "#;

        let mut storage = world.write::<CompTest>();
        let packed: PackedData<CompTest> = serde_json::from_str(&data).unwrap();
        assert_eq!(packed.offsets, vec![3, 7, 8]);
        assert_eq!(
            packed.components,
            vec![
                CompTest {
                    field1: 0,
                    field2: true,
                },
                CompTest {
                    field1: 158123,
                    field2: false,
                },
                CompTest {
                    field1: u32::max_value(),
                    field2: false,
                },
            ]
        );

        storage
            .merge(&entities.as_slice(), packed)
            .expect("Failed to merge into storage");

        assert_eq!((&storage).join().count(), 3);
        assert_eq!(
            (&storage).get(entities[3]),
            Some(&CompTest {
                field1: 0,
                field2: true,
            })
        );
        assert_eq!(
            (&storage).get(entities[7]),
            Some(&CompTest {
                field1: 158123,
                field2: false,
            })
        );
        assert_eq!(
            (&storage).get(entities[8]),
            Some(&CompTest {
                field1: u32::max_value(),
                field2: false,
            })
        );

        let none = vec![0, 1, 2, 4, 5, 6, 9];
        for entity in none {
            assert_eq!((&storage).get(entities[entity]), None);
        }
    }
}
