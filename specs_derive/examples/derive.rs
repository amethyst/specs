
extern crate specs;
#[macro_use]
extern crate specs_derive;
extern crate serde;
#[macro_use]
extern crate serde_derive;

use specs::{VecStorage, Gate, Component};

#[derive(Serialize, Deserialize)]
struct Comp1(String);
impl Component for Comp1 {
    type Storage = VecStorage<Comp1>;
}

#[derive(Serialize, Deserialize)]
struct Comp2(f32);
impl Component for Comp2 {
    type Storage = VecStorage<Comp2>;
}

#[derive(Serialize, Deserialize)]
struct Comp3(u32);
impl Component for Comp3 {
    type Storage = VecStorage<Comp3>;
}

#[derive(ComponentGroup)]
#[allow(dead_code)]
struct SomeGroup {
    #[group(no_serialize)]
    field1: Comp1,
    field2: Comp2,
    field3: Comp3,
}

fn main() {

}
