#[cfg(feature = "serde")]
#[macro_use]
extern crate serde;
// Specs is renamed here so that the custom derive cannot refer specs directly
#[cfg(feature = "serde")]
extern crate specs as spocs;

#[cfg(feature = "serde")]
#[macro_use]
extern crate specs_derive;

#[cfg(feature = "serde")]
mod tests {
    use spocs::{
        Builder, Entity,
        saveload::{ConvertSaveload, Marker, U64Marker, U64MarkerAllocator, MarkedBuilder, SerializeComponents},
        error::{Error, NoError},
        World, ReadStorage, DenseVecStorage, Component
    };

    #[derive(ConvertSaveload)]
    struct OneFieldNamed {
        e: Entity,
    }

    #[derive(ConvertSaveload)]
    struct TwoField {
        a: u32,
        e: Entity,
    }

    // Tests a struct that owns a parent
    // that derives Saveload
    #[derive(ConvertSaveload)]
    struct LevelTwo {
        owner: OneFieldNamed,
    }

    #[derive(ConvertSaveload)]
    struct OneFieldTuple(Entity);

    #[derive(ConvertSaveload)]
    struct TwoFieldTuple(Entity, u32);

    #[derive(ConvertSaveload)]
    struct LevelTwoTuple(OneFieldNamed);

    #[derive(ConvertSaveload)]
    enum AnEnum {
        E(Entity),
        F { e: Entity },
        Unit,
    }

    #[derive(ConvertSaveload)]
    struct Generic<E: EntityLike>(E);

    trait EntityLike {}

    impl EntityLike for Entity {}

    #[test]
    fn type_check() {
        let mut world = World::new();
        let entity = world.create_entity().build();

        black_box::<U64Marker, _>(OneFieldNamed { e: entity });
        black_box::<U64Marker, _>(TwoField { a: 5, e: entity });
        black_box::<U64Marker, _>(LevelTwo {
            owner: OneFieldNamed { e: entity },
        });
        black_box::<U64Marker, _>(OneFieldTuple(entity));
        black_box::<U64Marker, _>(TwoFieldTuple(entity, 5));
        // The derive will work for all variants
        // so no need to test anything but unit
        black_box::<U64Marker, _>(AnEnum::Unit);
        //black_box::<U64Marker, _>(Generic(entity));
    }

    #[test]
    fn test_entity_reference_error() {
        #[derive(Component)]
        struct Parent(Entity);

        impl ConvertSaveload<U64Marker> for Parent {
            type Data = U64Marker;
            type Error = Error;

            fn convert_into<F: FnMut(Entity) -> Option<U64Marker>>(&self, ids: F) -> Result<Self::Data, Self::Error> {
                self.0.convert_into(ids)
            }

            fn convert_from<F: FnMut(U64Marker) -> Option<Entity>>(marker: Self::Data, ids: F) -> Result<Self, Self::Error> {
                Entity::convert_from(marker,  ids).map(|entity| Parent(entity))
            }
        }

        let mut world = World::new();

        world.register::<Parent>();
        world.register::<U64Marker>();
        world.add_resource(U64MarkerAllocator::new());

        let parent = world.create_entity()
            .marked::<U64Marker>()
            .build();

        let child = world.create_entity()
            .with(Parent(parent))
            .marked::<U64Marker>()
            .build();

        world.delete_entity(parent).unwrap();

        let (parents, markers): (ReadStorage<Parent>, ReadStorage<U64Marker>) = world.system_data();

        let ids = |entity| markers.get(entity).cloned();

        assert_eq!(
            (&parents,).serialize_entity(child, ids),
            Err(Error::NoMarker)
        );
    }

    fn black_box<M, T: ConvertSaveload<M>>(_item: T) {}
}
