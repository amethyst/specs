#![cfg(not(feature = "parallel"))]

use std::rc::Rc;

use specs::{storage::VecStorage, Builder, Component, World, WorldExt};

#[derive(PartialEq)]
struct CompNonSend(Rc<u32>);

impl Component for CompNonSend {
    type Storage = VecStorage<Self>;
}

#[test]
fn non_send_component_is_accepted() {
    let mut world = World::new();
    world.register::<CompNonSend>();

    let entity = world
        .create_entity()
        .with(CompNonSend(Rc::new(123)))
        .build();

    let comp_non_sends = world.read_storage::<CompNonSend>();
    let comp_non_send = comp_non_sends
        .get(entity)
        .expect("Expected component to exist.");
    assert_eq!(123, *comp_non_send.0);
}
