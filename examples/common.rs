extern crate futures;
extern crate specs;

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use futures::Future;
use futures::future::Lazy;
use specs::*;
use specs::common::{Errors, FutureComponent, Merge};

struct MyFloat(f32);

impl Component for MyFloat {
    type Storage = VecStorage<Self>;
}

fn main() {
    let mut world = World::new();

    world.register::<MyFloat>();
    world.register::<FutureComponent<MyFloat>>();

    world.add_resource(Errors::new());

    world.create_entity()
        .with(future_sqrt(25.0))
        .build();

    let mut dispatcher = DispatcherBuilder::new()
        .add(Merge::<MyFloat>::new(), "merge_my_float", &[])
        .build();

    dispatcher.dispatch(&mut world.res);

    world.write_resource::<Errors>().print_and_exit();
}

#[derive(Debug)]
enum MyErr {
    _What,
    _Ever,
}

impl Display for MyErr {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.description())
    }
}

impl Error for MyErr {
    fn description(&self) -> &str {
        match *self {
            MyErr::_What => "What",
            MyErr::_Ever => "Ever",
        }
    }
}

fn future_sqrt(num: f32) -> FutureComponent<MyFloat> {
    use futures::lazy;
    use specs::common::BoxedErr;

    let lazy: Lazy<_, Result<_, MyErr>> = lazy(move || Ok(num.sqrt()));
    let future = lazy.map(MyFloat).map_err(BoxedErr::new);

    Box::new(future)
}
