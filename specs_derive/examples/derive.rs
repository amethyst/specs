extern crate specs;
#[macro_use]
extern crate specs_derive;

#[cfg(feature="serialize")]
extern crate serde;
#[cfg(feature="serialize")]
#[macro_use]
extern crate serde_derive;

#[cfg(feature="serialize")]
fn main() {
    use specs::prelude::*;

    #[derive(Debug, Serialize, Deserialize)]
    struct Comp1(String);
    impl Component for Comp1 {
        type Storage = VecStorage<Comp1>;
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Comp2(f32);
    impl Component for Comp2 {
        type Storage = VecStorage<Comp2>;
    }

    #[derive(Debug, Serialize, Deserialize)]
    struct Comp3(u32);
    impl Component for Comp3 {
        type Storage = VecStorage<Comp3>;
    }

    #[derive(ComponentGroup)]
    #[allow(dead_code)]
    struct SomeGroup {
        #[group(serialize)]
        field1: Comp1,

        #[group(serialize)]
        field2: Comp2,

        field3: Comp3,
    }

    // Running
    let mut world = World::new();
    world.register::<Comp1>();
}

#[cfg(not(feature="serialize"))]
fn main() {
    println!("Requires `serialize` flag to run");
}
