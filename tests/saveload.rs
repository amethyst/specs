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
    #[cfg(feature = "uuid_entity")]
    use spocs::saveload::UuidMarker;
    use spocs::{
        saveload::{ConvertSaveload, Marker, SimpleMarker},
        Builder, Entity, World, WorldExt,
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

    struct NetworkSync;

    #[test]
    fn type_check() {
        let mut world = World::new();
        let entity = world.create_entity().build();
        type_check_internal::<SimpleMarker<NetworkSync>>(entity);
        #[cfg(feature = "uuid_entity")]
        type_check_internal::<UuidMarker>(entity);
    }

    fn type_check_internal<M: Marker>(entity: Entity) {
        black_box::<M, _>(OneFieldNamed { e: entity });
        black_box::<M, _>(TwoField { a: 5, e: entity });
        black_box::<M, _>(LevelTwo {
            owner: OneFieldNamed { e: entity },
        });
        black_box::<M, _>(OneFieldTuple(entity));
        black_box::<M, _>(TwoFieldTuple(entity, 5));
        // The derive will work for all variants
        // so no need to test anything but unit
        black_box::<M, _>(AnEnum::Unit);
        black_box::<M, _>(Generic(entity));
    }

    fn black_box<M, T: ConvertSaveload<M>>(_item: T) {}
}
