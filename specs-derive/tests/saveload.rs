#[macro_use]
extern crate serde;
// Specs is renamed here so that the custom derive cannot refer specs directly
extern crate specs as spocs;
#[macro_use]
extern crate specs_derive;

use spocs::{
    error::NoError,
    saveload::{ConvertSaveload, Marker},
    Entity,
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

mod tests {
    use super::*;
    use spocs::saveload::{ConvertSaveload, U64Marker};
    use spocs::{Builder, World};

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
        black_box::<U64Marker, _>(Generic(entity));
    }

    fn black_box<M, T: ConvertSaveload<M>>(_item: T) {}
}
