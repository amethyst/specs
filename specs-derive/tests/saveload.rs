// These are mostly just compile tests
#![allow(dead_code)]

#[macro_use]
extern crate serde;
extern crate specs;
#[macro_use]
extern crate specs_derive;

use specs::prelude::*;

#[derive(Saveload)]
struct OneFieldNamed {
    e: Entity,
}

#[derive(Saveload)]
struct TwoField {
    a: u32,
    e: Entity,
}

// Tests a struct that owns a parent
// that derives Saveload
#[derive(Saveload)]
struct LevelTwo {
    owner: OneFieldNamed,
}

#[derive(Saveload)]
struct OneFieldTuple(Entity);

#[derive(Saveload)]
struct TwoFieldTuple(Entity, u32);

#[derive(Saveload)]
struct LevelTwoTuple(OneFieldNamed);

#[derive(Saveload)]
enum AnEnum {
    E(Entity),
    F { e: Entity },
    Unit,
}

mod tests {
    use super::*;
    use specs::saveload::{FromDeserialize, IntoSerialize, U64Marker};

    /* Just a compile test to verify that we meet the proper bounds.
    Does not need to be #[test] since it's a compile test */

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
    }

    fn black_box<M, T: IntoSerialize<M> + FromDeserialize<M>>(_item: T) {}
}
