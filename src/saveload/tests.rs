extern crate ron;

use std::{convert::Infallible, hash::Hash};

use super::*;
use crate::{error::Error, prelude::*};

mod marker_test {
    use super::*;

    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    struct A(i32);

    impl Component for A {
        type Storage = VecStorage<Self>;
    }

    #[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
    struct B(bool);

    impl Component for B {
        type Storage = VecStorage<Self>;
    }

    struct NetworkSync;

    /// Ensure that the marker correctly allocates IDs for entities that come
    /// from mixed sources: normal entity creation, lazy creation, and
    /// deserialization.
    #[test]
    fn bumps_index_after_reload() {
        bumps_index_after_reload_internal::<SimpleMarker<NetworkSync>>(SimpleMarkerAllocator::new());
        #[cfg(feature = "uuid_entity")]
        bumps_index_after_reload_internal::<UuidMarker>(UuidMarkerAllocator::new());
    }

    fn bumps_index_after_reload_internal<M>(allocator: M::Allocator)
    where
        M: Marker + Component + Clone,
        <M as Component>::Storage: Default,
        <M as Marker>::Identifier: Clone + Hash + Eq,
        <M as Marker>::Allocator: Default + Clone,
    {
        let mut world = World::new();

        world.insert(allocator.clone());
        world.register::<A>();
        world.register::<B>();
        world.register::<M>();

        world
            .create_entity()
            .with(A(32))
            .with(B(true))
            .marked::<M>()
            .build();
        world
            .create_entity()
            .with(A(64))
            .with(B(false))
            .marked::<M>()
            .build();

        // Serialize all entities
        let mut ser = ron::ser::Serializer::new(Some(Default::default()), true);

        world.exec(
            |(ents, comp_a, comp_b, markers, _alloc): (
                Entities,
                ReadStorage<A>,
                ReadStorage<B>,
                ReadStorage<M>,
                Read<M::Allocator>,
            )| {
                SerializeComponents::<Infallible, M>::serialize(
                    &(&comp_a, &comp_b),
                    &ents,
                    &markers,
                    &mut ser,
                )
                .unwrap();
            },
        );

        let serial = ser.into_output_string();

        let mut de = ron::de::Deserializer::from_str(&serial).unwrap();

        // Throw the old world away and deserialize into a new world
        let mut world = World::new();

        world.insert(allocator);
        world.register::<A>();
        world.register::<B>();
        world.register::<M>();

        world.exec(
            |(ents, comp_a, comp_b, mut markers, mut alloc): (
                Entities,
                WriteStorage<A>,
                WriteStorage<B>,
                WriteStorage<M>,
                Write<M::Allocator>,
            )| {
                DeserializeComponents::<Error, _>::deserialize(
                    &mut (comp_a, comp_b),
                    &ents,
                    &mut markers,
                    &mut alloc,
                    &mut de,
                )
                .unwrap();
            },
        );

        // Two marked entities should be deserialized
        assert_marked_entity_count::<M>(&mut world, 2);

        // Queue lazy creation of 2 more entities
        world.exec(|(ents, lazy): (Entities, Read<LazyUpdate>)| {
            lazy.create_entity(&ents)
                .with(A(128))
                .with(B(false))
                .marked::<M>()
                .build();
            lazy.create_entity(&ents)
                .with(A(256))
                .with(B(true))
                .marked::<M>()
                .build();
        });

        // Create 2 new entities besides the deserialized ones
        world
            .create_entity()
            .with(A(512))
            .with(B(false))
            .marked::<M>()
            .build();
        world
            .create_entity()
            .with(A(1024))
            .with(B(true))
            .marked::<M>()
            .build();

        // Check that markers of deserialized entities and newly created entities are
        // unique
        assert_marked_entity_count::<M>(&mut world, 4);
        assert_markers_are_unique::<M>(&mut world);

        // Check that markers of lazily created entities are unique
        world.maintain();
        assert_marked_entity_count::<M>(&mut world, 6);
        assert_markers_are_unique::<M>(&mut world);
    }

    /// Assert that the number of entities marked with `SimpleMarker` is equal
    /// to `count`
    fn assert_marked_entity_count<M: Marker>(world: &mut World, count: usize) {
        world.exec(|(ents, markers): (Entities, ReadStorage<M>)| {
            let marked_entity_count = (&ents, &markers).join().count();

            assert_eq!(marked_entity_count, count);
        });
    }

    /// Ensure there are no duplicate marker .ids() in the world
    fn assert_markers_are_unique<M: Marker>(world: &mut World)
    where
        <M as Marker>::Identifier: Clone + Eq + Hash,
    {
        world.exec(|(ents, markers): (Entities, ReadStorage<M>)| {
            use std::collections::HashSet;

            let marker_ids: Vec<_> = (&ents, &markers)
                .join()
                .map(|(_entity, marker)| marker.id())
                .collect();

            let marker_id_set: HashSet<_> = marker_ids.iter().cloned().collect();

            assert_eq!(marker_ids.len(), marker_id_set.len());
        });
    }
}
