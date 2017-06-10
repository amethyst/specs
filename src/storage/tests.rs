use mopa::Any;

use super::*;
use super::storages::*;
use entity::{Component, Entity, Generation};
use {Index, World};

fn create<T: Component>(world: &mut World) -> WriteStorage<T> {
    world.register::<T>();

    world.write()
}

mod map_test {
    use super::*;

    #[derive(Debug)]
    struct Comp<T>(T);
    impl<T: Any + Send + Sync> Component for Comp<T> {
        type Storage = VecStorage<Comp<T>>;
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
        type Storage = VecStorage<Cvec>;
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
        type Storage = FlaggedStorage<FlaggedCvec, VecStorage<FlaggedCvec>>;
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
        type Storage = HashMapStorage<Cmap>;
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
        type Storage = BTreeStorage<CBtree>;
    }

    #[derive(Clone, Debug)]
    struct Cnull(u32);
    impl Default for Cnull {
        fn default() -> Cnull {
            Cnull(0)
        }
    }
    impl From<u32> for Cnull {
        fn from(v: u32) -> Cnull {
            Cnull(v)
        }
    }
    impl Component for Cnull {
        type Storage = NullStorage<Cnull>;
    }

    fn test_add<T: Component + From<u32> + Debug + Eq>() {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation::new(1)), (i + 2718).into());
        }

        for i in 0..1_000 {
            assert_eq!(s.get(Entity::new(i, Generation::new(1))).unwrap(),
                       &(i + 2718).into());
        }
    }

    fn test_sub<T: Component + From<u32> + Debug + Eq>() {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        for i in 0..1_000 {
            s.insert(Entity::new(i, Generation::new(1)), (i + 2718).into());
        }

        for i in 0..1_000 {
            assert_eq!(s.remove(Entity::new(i, Generation::new(1))).unwrap(),
                       (i + 2718).into());
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
            assert_eq!(s.get(Entity::new(i, Generation::new(1))).unwrap(),
                       &(i + 2000).into());
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
            assert_eq!(s.get(Entity::new(i, Generation::new(1))).unwrap(),
                       &(i + 2718).into());
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

        let mut storage = VecStorage::<A>::new();

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

    #[test]
    fn dummy_test_clear() {
        test_clear::<Cnull>();
    }

    // Check storage tests
    #[test]
    fn check_storage() {
        use join::Join;
        let mut w = World::new();
        let mut s1 = create::<Cvec>(&mut w);

        for i in 0..50 {
            s1.insert(Entity::new(i, Generation::new(1)), (i + 10).into());
        }
        for mut entry in (&s1.check()).join() {
            {
                s1.get_unchecked(&entry);
            }

            {
                s1.get_mut_unchecked(&mut entry);
            }
        }
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
        let s2: Storage<Cvec, _> = w.write_with_id(2);

        for i in 0..50 {
            s1.insert(Entity::new(i, Generation::new(1)), (i + 10).into());
        }
        for entry in (&s1.check()).join() {
            s2.get_unchecked(&entry); // verify that the assert fails if the storage is
            // not the original.
        }
    }

    #[test]
    fn flagged() {
        use join::Join;
        use world::EntityIndex;

        let mut w = World::new();
        w.register_with_id::<FlaggedCvec>(1);
        w.register_with_id::<FlaggedCvec>(2);
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
        for c1 in ((&s1).check()).join() {
            assert!(!s1.open().1.flagged(&c1));
        }

        // Modify components to flag.
        for (c1, c2) in (&mut s1, &s2).join() {
            println!("{:?} {:?}", c1, c2);
            c1.0 += c2.0;
        }

        for c1 in (s1.check()).join() {
            // Should only be modified if the entity had both components
            // Which means only half of them should have it.
            if s1.open().1.flagged(&c1) {
                println!("Flagged: {:?}", c1.index());
                // Only every other component was flagged.
                assert!(c1.index() % 2 == 0);
            }
        }

        // Iterate over all flagged entities.
        for (entity, _) in (&*w.entities(), s1.open().1).join() {
            // All entities in here should be flagged.
            assert!(s1.open().1.flagged(&entity));
        }
    }
}

#[cfg(feature="serialize")]
mod serialize_test {
    extern crate serde_json;

    use super::{Join, VecStorage, Component, PackedData};
    use world::World;

    #[derive(PartialEq, Debug, Serialize, Deserialize)]
    struct CompTest {
        field1: u32,
        field2: bool,
    }
    impl Component for CompTest {
        type Storage = VecStorage<CompTest>;
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
        assert_eq!(serialized,
                   r#"{"offsets":[1,4,5],"components":[{"field1":0,"field2":true},{"field1":158123,"field2":false},{"field1":4294967295,"field2":false}]}"#);
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
        assert_eq!(packed.components,
                   vec![CompTest {
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
                        }]);

        storage
            .merge(&entities.as_slice(), packed)
            .expect("Failed to merge into storage");

        assert_eq!((&storage).join().count(), 3);
        assert_eq!((&storage).get(entities[3]),
                   Some(&CompTest {
                            field1: 0,
                            field2: true,
                        }));
        assert_eq!((&storage).get(entities[7]),
                   Some(&CompTest {
                            field1: 158123,
                            field2: false,
                        }));
        assert_eq!((&storage).get(entities[8]),
                   Some(&CompTest {
                            field1: u32::max_value(),
                            field2: false,
                        }));

        let none = vec![0, 1, 2, 4, 5, 6, 9];
        for entity in none {
            assert_eq!((&storage).get(entities[entity]), None);
        }
    }
}
