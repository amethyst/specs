use std::any::Any;

use super::*;
use crate::world::{Component, Entity, Generation, Index, WorldExt};
use shred::World;
use std::mem::MaybeUninit;

fn create<T: Component>(world: &mut World) -> WriteStorage<T>
where
    T::Storage: Default,
{
    world.register::<T>();

    world.write_storage()
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
            if let Err(err) = c.insert(ent(i), Comp(i)) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
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
            if let Err(err) = c.insert(ent(i), Comp(i)) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
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
            if let Err(err) = c.insert(ent(i), Comp(i)) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
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
            if let Err(err) = c.insert(ent(i as u32), Comp(i)) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
            if let Err(err) = c.insert(ent(i as u32), Comp(-i)) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
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
            if let Err(err) = c.insert(ent(i), Comp(i)) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
            assert_eq!(c.get(ent(i)).unwrap().0, i);
        }
    }

    #[should_panic]
    #[test]
    fn wrap() {
        let mut w = World::new();
        let mut c = create(&mut w);

        let _ = c.insert(ent(1 << 25), Comp(7));
    }
}

mod test {
    use std::{convert::AsMut, fmt::Debug};

    use super::*;
    use crate::{world::Builder, World};

    #[derive(PartialEq, Eq, Debug, Default)]
    struct CMarker;
    impl Component for CMarker {
        type Storage = NullStorage<Self>;
    }

    #[derive(PartialEq, Eq, Debug, Default)]
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

    struct CEntries(u32);

    impl From<u32> for CEntries {
        fn from(v: u32) -> CEntries {
            CEntries(v)
        }
    }

    impl Component for CEntries {
        type Storage = VecStorage<Self>;
    }

    #[derive(PartialEq, Eq, Debug, Default)]
    struct CdefaultVec(u32);
    impl From<u32> for CdefaultVec {
        fn from(v: u32) -> CdefaultVec {
            CdefaultVec(v)
        }
    }
    impl AsMut<u32> for CdefaultVec {
        fn as_mut(&mut self) -> &mut u32 {
            &mut self.0
        }
    }
    impl Component for CdefaultVec {
        type Storage = DefaultVecStorage<Self>;
    }

    fn test_add<T: Component + From<u32> + Debug + Eq>()
    where
        T::Storage: Default,
    {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        for i in 0..1_000 {
            if let Err(err) = s.insert(Entity::new(i, Generation::new(1)), (i + 2718).into()) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
        }

        for i in 0..1_000 {
            assert_eq!(
                s.get(Entity::new(i, Generation::new(1))).unwrap(),
                &(i + 2718).into()
            );
        }
    }

    fn test_sub<T: Component + From<u32> + Debug + Eq>()
    where
        T::Storage: Default,
    {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        for i in 0..1_000 {
            if let Err(err) = s.insert(Entity::new(i, Generation::new(1)), (i + 2718).into()) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
        }

        for i in 0..1_000 {
            assert_eq!(
                s.remove(Entity::new(i, Generation::new(1))).unwrap(),
                (i + 2718).into()
            );
            assert!(s.remove(Entity::new(i, Generation::new(1))).is_none());
        }
    }

    fn test_get_mut<T: Component + From<u32> + AsMut<u32> + Debug + Eq>()
    where
        T::Storage: Default,
    {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        for i in 0..1_000 {
            if let Err(err) = s.insert(Entity::new(i, Generation::new(1)), (i + 2718).into()) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
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

    fn test_get_mut_or_default<T: Component + Default + From<u32> + AsMut<u32> + Debug + Eq>()
    where
        T::Storage: Default,
    {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        // Insert the first 500 components manually, leaving indices 500..1000
        // unoccupied.
        for i in 0..500 {
            if let Err(err) = s.insert(Entity::new(i, Generation::new(1)), (i).into()) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
        }

        for i in 0..1_000 {
            *s.get_mut_or_default(Entity::new(i, Generation::new(1)))
                .unwrap()
                .as_mut() += i;
        }

        // The first 500 were initialized, and should be i*2.
        for i in 0..500 {
            assert_eq!(
                s.get(Entity::new(i, Generation::new(1))).unwrap(),
                &(i + i).into()
            );
        }

        // The rest were Default-initialized, and should equal i.
        for i in 500..1_000 {
            assert_eq!(
                s.get(Entity::new(i, Generation::new(1))).unwrap(),
                &(i).into()
            );
        }
    }

    fn test_add_gen<T: Component + From<u32> + Debug + Eq>()
    where
        T::Storage: Default,
    {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        for i in 0..1_000 {
            if let Err(err) = s.insert(Entity::new(i, Generation::new(1)), (i + 2718).into()) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
            if s.insert(Entity::new(i, Generation::new(2)), (i + 31415).into()).is_ok() {
                panic!("Overwrote entity generation!  I shouldn't have been allowed to do this!");
            }
        }

        for i in 0..1_000 {
            assert!(s.get(Entity::new(i, Generation::new(2))).is_none());
            assert_eq!(
                s.get(Entity::new(i, Generation::new(1))).unwrap(),
                &(i + 2718).into()
            );
        }
    }

    fn test_sub_gen<T: Component + From<u32> + Debug + Eq>()
    where
        T::Storage: Default,
    {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        for i in 0..1_000 {
            if s.insert(Entity::new(i, Generation::new(2)), (i + 2718).into()).is_ok() {
                panic!("Overwrote entity generation!  I shouldn't have been allowed to do this!");
            }
        }

        for i in 0..1_000 {
            assert!(s.remove(Entity::new(i, Generation::new(1))).is_none());
        }
    }

    fn test_clear<T: Component + From<u32>>()
    where
        T::Storage: Default,
    {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        for i in 0..10 {
            if let Err(err) = s.insert(Entity::new(i, Generation::new(1)), (i + 10).into()) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
        }

        s.clear();

        for i in 0..10 {
            assert!(s.get(Entity::new(i, Generation::new(1))).is_none());
        }
    }

    fn test_anti<T: Component + From<u32> + Debug + Eq>()
    where
        T::Storage: Default,
    {
        let mut w = World::new();
        let mut s: Storage<T, _> = create::<T>(&mut w);

        for i in 0..10 {
            if let Err(err) = s.insert(Entity::new(i, Generation::new(1)), (i + 10).into()) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
        }

        for (i, (a, _)) in (&s, !&s).join().take(10).enumerate() {
            assert_eq!(a, &(i as u32).into());
        }
    }

    fn test_slice_access<T: Component + From<u32> + Debug + Eq>()
    where
        T::Storage: Default + SliceAccess<T, Element = T>,
    {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        for i in 0..1_000 {
            if let Err(err) = s.insert(Entity::new(i, Generation::new(1)), (i + 2718).into()) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
        }

        let slice = s.as_slice();
        assert_eq!(slice.len(), 1_000);
        for (i, v) in slice.iter().enumerate() {
            assert_eq!(v, &(i as u32 + 2718).into());
        }
    }

    fn test_maybeuninit_slice<T: Component + From<u32> + Debug + Eq>()
    where
        T::Storage: Default + SliceAccess<T, Element = MaybeUninit<T>>,
    {
        let mut w = World::new();
        let mut s: Storage<T, _> = create(&mut w);

        for i in 0..1_000 {
            if let Err(err) = s.insert(Entity::new(i, Generation::new(1)), (i + 2718).into()) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
        }

        let slice = s.as_slice();
        assert_eq!(slice.len(), 1_000);
        for (i, v) in slice.iter().enumerate() {
            let v = unsafe { &*v.as_ptr() };
            assert_eq!(v, &(i as u32 + 2718).into());
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
    fn vec_test_get_mut_or_default() {
        test_get_mut_or_default::<Cvec>();
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
    fn vec_test_maybeuninit_slice() {
        test_maybeuninit_slice::<Cvec>();
    }

    #[test]
    fn vec_arc() {
        use std::sync::Arc;

        #[derive(Debug)]
        struct A(Arc<()>);

        let mut storage = VecStorage::<A>::default();
        let mut bitset = BitSet::new();

        unsafe {
            for i in (0..200).filter(|i| i % 2 != 0) {
                storage.insert(i, A(Arc::new(())));
                bitset.add(i);
            }
            storage.clean(&bitset);
        }
    }

    #[test]
    fn default_vec_test_add() {
        test_add::<CdefaultVec>();
    }
    #[test]
    fn default_vec_test_sub() {
        test_sub::<CdefaultVec>();
    }
    #[test]
    fn default_vec_test_get_mut() {
        test_get_mut::<CdefaultVec>();
    }
    #[test]
    fn default_vec_test_get_mut_or_default() {
        test_get_mut_or_default::<CdefaultVec>();
    }
    #[test]
    fn default_vec_test_add_gen() {
        test_add_gen::<CdefaultVec>();
    }
    #[test]
    fn default_vec_test_sub_gen() {
        test_sub_gen::<CdefaultVec>();
    }
    #[test]
    fn default_vec_test_clear() {
        test_clear::<CdefaultVec>();
    }
    #[test]
    fn default_vec_test_anti() {
        test_anti::<CdefaultVec>();
    }
    #[test]
    fn default_vec_test_slice_access() {
        test_slice_access::<CdefaultVec>();
    }

    #[test]
    fn default_vec_test_defaults() {
        let mut w = World::new();
        let mut s: Storage<CdefaultVec, _> = create(&mut w);

        // insert 1 and 3 at 1 and 3
        s.insert(Entity::new(1, Generation::new(1)), 1.into())
            .unwrap();
        s.insert(Entity::new(3, Generation::new(1)), 3.into())
            .unwrap();

        // should contain default values at other locations
        assert_eq!(
            s.as_slice(),
            &[
                CdefaultVec(0),
                CdefaultVec(1),
                CdefaultVec(0),
                CdefaultVec(3),
            ]
        );

        // deleting the record 3 should swap in the default but not shrink
        s.remove(Entity::new(3, Generation::new(1)));
        assert_eq!(
            s.as_slice(),
            &[
                CdefaultVec(0),
                CdefaultVec(1),
                CdefaultVec(0),
                CdefaultVec(0),
            ]
        );
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

    #[test]
    fn test_null_insert_twice() {
        let mut w = World::new();

        w.register::<Cnull>();
        let e = w.create_entity().build();

        let mut null = w.write_storage::<Cnull>();

        assert_eq!(null.get(e), None);
        match null.insert(e, Cnull) {
            Ok(None) => {}
            r => panic!("Expected Ok(None) got {:?}", r),
        }
        match null.insert(e, Cnull) {
            Ok(Some(Cnull)) => {}
            r => panic!("Expected Ok(Some(Cnull)) got {:?}", r),
        }
    }

    #[test]
    fn restricted_storage() {
        use crate::join::Join;
        use std::collections::HashSet;

        let mut w = World::new();
        w.register::<Cvec>();
        let mut s1: Storage<Cvec, _> = w.write_storage();
        let mut components = HashSet::new();

        for i in 0..50 {
            let c = i + 10;
            if let Err(err) = s1.insert(Entity::new(i, Generation::new(1)), c.into()) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
            components.insert(c);
        }

        for mut comps in (&mut s1.restrict_mut()).join() {
            let c1 = { comps.get_unchecked().0 };

            let c2 = { comps.get_mut_unchecked().0 };

            assert_eq!(
                c1, c2,
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
    #[cfg(feature = "parallel")]
    fn par_restricted_storage() {
        use crate::join::ParJoin;
        use rayon::iter::ParallelIterator;
        use std::{collections::HashSet, sync::Mutex};

        let mut w = World::new();
        w.register::<Cvec>();
        let mut s1: Storage<Cvec, _> = w.write_storage();
        let mut components = HashSet::new();

        for i in 0..50 {
            let c = i + 10;
            if let Err(err) = s1.insert(Entity::new(i, Generation::new(1)), c.into()) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
            components.insert(c);
        }

        let components2 = Mutex::new(Vec::new());
        let components2_mut = Mutex::new(Vec::new());

        (&mut s1.par_restrict_mut())
            .par_join()
            .for_each(|mut comps| {
                let (mut components2, mut components2_mut) =
                    (components2.lock().unwrap(), components2_mut.lock().unwrap());
                components2.push(comps.get_unchecked().0);
                components2_mut.push(comps.get_mut_unchecked().0);
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
    fn storage_entry() {
        let mut w = World::new();
        w.register::<Cvec>();

        let e1 = w.create_entity().build();
        let e2 = w.create_entity().with(Cvec(10)).build();

        let e3 = w.create_entity().build();
        let e4 = w.create_entity().with(Cvec(10)).build();

        let e5 = w.create_entity().build();
        let e6 = w.create_entity().with(Cvec(10)).build();

        let e7 = w.create_entity().build();
        let e8 = w.create_entity().with(Cvec(10)).build();

        let mut s1 = w.write_storage::<Cvec>();

        // Basic entry usage.
        if let Ok(entry) = s1.entry(e1) {
            entry.or_insert(Cvec(5));
        }

        if let Ok(entry) = s1.entry(e2) {
            entry.or_insert(Cvec(5));
        }

        // Verify that lazy closures are called only when inserted.
        {
            let mut increment = 0;
            let mut lazy_increment = |entity: Entity, valid: u32| {
                if let Ok(entry) = s1.entry(entity) {
                    entry.or_insert_with(|| {
                        increment += 1;
                        Cvec(5)
                    });

                    assert_eq!(increment, valid);
                }
            };

            lazy_increment(e3, 1);
            lazy_increment(e4, 1);
        }

        // Sanity checks that the entry is occupied after insertions.
        {
            let mut occupied = |entity, value| {
                assert_eq!(*s1.get(entity).unwrap(), value);

                match s1.entry(entity) {
                    Ok(StorageEntry::Occupied(mut occupied)) => {
                        assert_eq!(*occupied.get_mut(), value)
                    }
                    _ => panic!("Entity not occupied {:?}", entity),
                }
            };

            occupied(e1, Cvec(5));
            occupied(e2, Cvec(10));
            occupied(e3, Cvec(5));
            occupied(e4, Cvec(10));
        }

        // Swap between occupied and vacant depending on the type of entry.
        {
            let mut toggle = |entity: Entity| match s1.entry(entity) {
                Ok(StorageEntry::Occupied(occupied)) => {
                    occupied.remove();
                }
                Ok(StorageEntry::Vacant(vacant)) => {
                    vacant.insert(Cvec(15));
                }
                Err(_) => {}
            };

            toggle(e5);
            toggle(e6);

            assert_eq!(s1.get(e5), Some(&Cvec(15)));
            assert_eq!(s1.get(e6), None);
        }

        // Check that `StorageEntry::replace` works as expected
        {
            assert_eq!(s1.entry(e7).unwrap().replace(Cvec(1337)), None);
            assert_eq!(s1.entry(e8).unwrap().replace(Cvec(11)), Some(Cvec(10)));
            assert_eq!(s1.entry(e7).unwrap().replace(Cvec(9001)), Some(Cvec(1337)));
            assert_eq!(s1.get(e7), Some(&Cvec(9001)));
            assert_eq!(s1.get(e8), Some(&Cvec(11)));
        }
    }

    #[test]
    fn storage_mask() {
        use crate::join::Join;

        let mut w = World::new();
        w.register::<CMarker>();
        let mut s1: Storage<CMarker, _> = w.write_storage();

        for i in 0..50 {
            if let Err(err) = s1.insert(Entity::new(i, Generation::new(1)), CMarker) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
        }

        for (entity, id) in (&w.entities(), s1.mask().clone()).join() {
            if id % 3 == 0 {
                let _ = s1.get_mut(entity);
            } else {
                let _ = s1.get(entity);
            }
        }

        assert_eq!((s1.mask()).join().count(), 50);
    }

    #[test]
    #[cfg(feature = "parallel")]
    fn par_storage_mask() {
        use crate::join::ParJoin;
        use rayon::iter::ParallelIterator;

        let mut w = World::new();
        w.register::<CMarker>();
        let mut s1: Storage<CMarker, _> = w.write_storage();

        for i in 0..50 {
            if let Err(err) = s1.insert(Entity::new(i, Generation::new(1)), CMarker) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
        }

        assert_eq!((s1.mask()).par_join().count(), 50);
    }

    #[test]
    fn flagged() {
        use crate::join::Join;

        let mut w = World::new();
        w.register::<FlaggedCvec>();

        let mut s1: Storage<FlaggedCvec, _> = w.write_storage();

        let mut inserted = BitSet::new();
        let mut modified = BitSet::new();
        let mut removed = BitSet::new();
        let mut reader_id = s1.register_reader();

        for i in 0..15 {
            let entity = w.entities().create();
            if let Err(err) = s1.insert(entity, i.into()) {
                panic!("Failed to insert component into entity! {:?}", err);
            }
        }

        {
            inserted.clear();
            modified.clear();
            removed.clear();

            let events = s1.channel().read(&mut reader_id);
            for event in events {
                match event {
                    ComponentEvent::Modified(id) => modified.add(*id),
                    ComponentEvent::Inserted(id) => inserted.add(*id),
                    ComponentEvent::Removed(id) => removed.add(*id),
                };
            }
        }

        for (entity, _) in (&w.entities(), &s1).join() {
            assert!(inserted.contains(entity.id()));
            assert!(!modified.contains(entity.id()));
            assert!(!removed.contains(entity.id()));
        }

        for (_, mut comp) in (&w.entities(), &mut s1).join() {
            comp.0 += 1;
        }

        {
            inserted.clear();
            modified.clear();
            removed.clear();

            let events = s1.channel().read(&mut reader_id);
            for event in events {
                match event {
                    ComponentEvent::Modified(id) => modified.add(*id),
                    ComponentEvent::Inserted(id) => inserted.add(*id),
                    ComponentEvent::Removed(id) => removed.add(*id),
                };
            }
        }

        for (entity, _) in (&w.entities(), &s1).join() {
            assert!(!inserted.contains(entity.id()));
            assert!(modified.contains(entity.id()));
            assert!(!removed.contains(entity.id()));
        }

        for (entity, _) in (&w.entities(), s1.mask().clone()).join() {
            s1.remove(entity);
        }

        {
            inserted.clear();
            modified.clear();
            removed.clear();

            let events = s1.channel().read(&mut reader_id);
            for event in events {
                match event {
                    ComponentEvent::Modified(id) => modified.add(*id),
                    ComponentEvent::Inserted(id) => inserted.add(*id),
                    ComponentEvent::Removed(id) => removed.add(*id),
                };
            }
        }

        for (entity, _) in (&w.entities(), &s1).join() {
            assert!(!inserted.contains(entity.id()));
            assert!(!modified.contains(entity.id()));
            assert!(removed.contains(entity.id()));
        }
    }

    #[test]
    fn entries() {
        use crate::{join::Join, storage::WriteStorage, world::Entities};

        let mut w = World::new();

        w.register::<CEntries>();

        {
            let mut s: Storage<CEntries, _> = w.write_storage();

            for i in 0..15 {
                let entity = w.entities().create();
                if let Err(err) = s.insert(entity, i.into()) {
                    panic!("Failed to insert component into entity! {:?}", err);
                }
            }

            for _ in 0..15 {
                w.entities().create();
            }
        }

        let mut sum = 0;

        w.exec(|(e, mut s): (Entities, WriteStorage<CEntries>)| {
            sum = (&e, s.entries()).join().fold(0, |acc, (_, value)| {
                let v = value.or_insert(2.into());
                acc + v.0
            });
        });

        assert_eq!(sum, 135);
    }
}
