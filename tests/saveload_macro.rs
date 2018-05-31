#![cfg(feature = "serde")]

#[macro_use]
extern crate specs;
#[macro_use]
extern crate specs_derive;
#[macro_use]
extern crate shred_derive;
#[macro_use]
extern crate serde;
extern crate ron;
extern crate serde_json;
extern crate shred;

use specs::prelude::*;
use specs::saveload::{U64Marker, U64MarkerAllocator};

#[derive(Copy, Clone, Serialize, Deserialize, Component, Debug, PartialEq)]
#[storage(VecStorage)]
struct Pos {
    x: f64,
    y: f64,
}

#[derive(Copy, Clone, Serialize, Deserialize, Component, Debug, PartialEq)]
#[storage(VecStorage)]
struct Vel {
    x: f64,
    y: f64,
}

#[derive(Copy, Clone, Component, Saveload)]
#[storage(VecStorage)]
struct OwnsEntity(Entity);

saveload_components!{
    [Pos, Vel, OwnsEntity], DeData, SerData, Data
}

#[derive(Clone, Eq, PartialEq, Hash, Default, Debug)]
struct Serialized {
    json: String,
}

#[derive(SystemData)]
struct DeSysData<'a> {
    de: saveload_generated::DeData<'a, U64Marker, U64MarkerAllocator>,
    target: Read<'a, Serialized>,
}

struct DeSys;

impl<'a> System<'a> for DeSys {
    type SystemData = DeSysData<'a>;

    fn run(&mut self, mut sys_data: Self::SystemData) {
        use ron::de::Deserializer;
        use specs::error::NoError;
        use specs::saveload::DeserializeComponents;

        if let Ok(mut de) = Deserializer::from_str(&sys_data.target.json) {
            DeserializeComponents::<NoError, U64Marker>::deserialize(
                &mut sys_data.de.components,
                &sys_data.de.entities,
                &mut sys_data.de.markers,
                &mut sys_data.de.alloc,
                &mut de,
            ).unwrap();
        }
    }
}

struct SerSys;

#[derive(SystemData)]
struct SerSysData<'a> {
    ser: saveload_generated::SerData<'a, U64Marker>,
    target: Write<'a, Serialized>,
}

impl<'a> System<'a> for SerSys {
    type SystemData = SerSysData<'a>;

    fn run(&mut self, mut sys_data: Self::SystemData) {
        use specs::error::NoError;
        use specs::saveload::SerializeComponents;

        let mut ser = ron::ser::Serializer::new(Some(Default::default()), true);
        SerializeComponents::<NoError, U64Marker>::serialize(
            &sys_data.ser.components,
            &sys_data.ser.entities,
            &sys_data.ser.markers,
            &mut ser,
        ).unwrap();

        sys_data.target.json = ser.into_output_string();
    }
}

#[test]
fn saveload_macro() {
    use specs::saveload::MarkedBuilder;

    let mut world = setup_world();
    let entity = world
        .create_entity()
        .with(Pos { x: 5.2, y: 9.1 })
        .with(Vel { x: 0.0, y: 1.0 })
        .marked::<U64Marker>()
        .build();

    let mut ser = SerSys;

    ser.run_now(&world.res);

    let mut world2 = setup_world();
    world2.add_resource(world.read_resource::<Serialized>().clone());
    let mut de = DeSys;

    de.run_now(&world2.res);
    world2.maintain();

    let mut len = 0;

    for (pos, vel) in (&world2.read_storage::<Pos>(), &world2.read_storage::<Vel>()).join() {
        len += 1;
        assert!(len < 2);

        assert_eq!(*pos, *world.read_storage().get(entity).unwrap());
        assert_eq!(*vel, *world.read_storage().get(entity).unwrap());
    }
}

fn setup_world() -> World {
    let mut world = World::new();

    world.maintain();
    world.add_resource(Serialized::default());
    world.add_resource(U64MarkerAllocator::new());

    world.register::<Pos>();
    world.register::<Vel>();
    world.register::<OwnsEntity>();
    world.register::<U64Marker>();

    world
}
