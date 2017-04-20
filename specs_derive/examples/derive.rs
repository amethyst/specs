
extern crate specs;
#[macro_use]
extern crate specs_derive;

#[derive(ComponentGroup)]
struct SomeGroup {
    #[group(no_serialize)]
    field1: u32,

    #[group(subgroup)]
    field2: f32,
    field3: String,
}

fn main() {

}
