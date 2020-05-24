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

    #[derive(Serialize, Deserialize, Clone)]
    struct TupleSerdeType(u32);

    #[derive(Clone)]
    struct UnserializableType {
        inner: u32
    }

    impl Default for UnserializableType {
        fn default() -> Self {
            Self {
                inner: 0
            }
        }
    }

    #[derive(Serialize, Deserialize, Clone)]
    struct ComplexSerdeType {
        #[serde(skip, default)]
        opaque: UnserializableType
    }

    #[derive(ConvertSaveload)]
    struct ComplexSerdeMixedType {
        #[convert_save_load_skip_convert]
        #[convert_save_load_attr(serde(skip, default))]
        opaque: UnserializableType,
        other: u32
    }

    #[derive(ConvertSaveload)]
    pub struct NamedContainsSerdeType {
        e: Entity,
        a: ComplexSerdeType,
        b: ComplexSerdeMixedType,
    }

    #[derive(ConvertSaveload)]
    struct TupleContainsSerdeType(Entity, ComplexSerdeMixedType);

    #[derive(ConvertSaveload)]
    enum NamedEnumContainsSerdeType {
        A(Entity),
        B { inner_baz: NamedContainsSerdeType }
    }

    #[derive(ConvertSaveload)]
    enum UnnamedEnumContainsSerdeType {
        A(Entity),
        B(NamedContainsSerdeType)
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
        black_box::<M, _>(TupleSerdeType(5));
        black_box::<M, _>(NamedContainsSerdeType{ e: entity, a: ComplexSerdeType { opaque: UnserializableType::default() }, b: ComplexSerdeMixedType { opaque: UnserializableType::default(), other: 5 } });
        black_box::<M, _>(TupleContainsSerdeType(entity, ComplexSerdeMixedType { opaque: UnserializableType::default(), other: 5 } ));
        black_box::<M, _>(NamedEnumContainsSerdeType::A(entity));
        black_box::<M, _>(UnnamedEnumContainsSerdeType::A(entity));
        // The derive will work for all variants
        // so no need to test anything but unit
        black_box::<M, _>(AnEnum::Unit);
        black_box::<M, _>(Generic(entity));
    }

    fn black_box<M, T: ConvertSaveload<M>>(_item: T) {}
}
