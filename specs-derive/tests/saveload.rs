#[macro_use]
extern crate serde;
extern crate specs;
#[macro_use]
extern crate specs_derive;

#[derive(ConvertSaveload)]
struct OneFieldNamed {
    e: ::specs::Entity,
}

#[derive(ConvertSaveload)]
struct TwoField {
    a: u32,
    e: ::specs::Entity,
}

// Tests a struct that owns a parent
// that derives Saveload
#[derive(ConvertSaveload)]
struct LevelTwo {
    owner: OneFieldNamed,
}

#[derive(ConvertSaveload)]
struct OneFieldTuple(::specs::Entity);

#[derive(ConvertSaveload)]
struct TwoFieldTuple(::specs::Entity, u32);

#[derive(ConvertSaveload)]
struct LevelTwoTuple(OneFieldNamed);

#[derive(ConvertSaveload)]
enum AnEnum {
    E(::specs::Entity),
    F { e: ::specs::Entity },
    Unit,
}

#[derive(ConvertSaveload)]
struct Generic<E: EntityLike>(E);

trait EntityLike {}

impl EntityLike for ::specs::Entity {}

mod tests {
    use super::*;
    use specs::{Builder, World};
    use specs::saveload::{ConvertSaveload, U64Marker};

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
