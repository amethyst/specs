extern crate futures;
extern crate specs;

use std::error::Error;
use std::fmt::{Display, Formatter, Result as FmtResult};

use futures::{Future, Poll};
use futures::future::Lazy;
use specs::*;
use specs::common::{BoxedFuture, Errors, Merge};
use specs::error::BoxedErr;

struct MyFloat(f32);

struct MyFuture(BoxedFuture<MyFloat>);

impl Future for MyFuture {
    type Item = MyFloat;
    type Error = BoxedErr;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}

impl Component for MyFloat {
    type Storage = VecStorage<Self>;
}

impl Component for MyFuture {
    type Storage = DenseVecStorage<Self>;
}

fn main() {
    let mut world = World::new();

    world.register::<MyFloat>();
    world.register::<MyFuture>();

    world.add_resource(Errors::new());

    world.create_entity().with(future_sqrt(25.0)).build();

    let mut dispatcher = DispatcherBuilder::new()
        .add(Merge::<MyFuture>::new(), "merge_my_float", &[])
        .build();

    dispatcher.dispatch(&world.res);

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

fn future_sqrt(num: f32) -> MyFuture {
    use futures::lazy;

    let lazy: Lazy<_, Result<_, MyErr>> = lazy(move || Ok(num.sqrt()));
    let future = lazy.map(MyFloat).map_err(BoxedErr::new);

    MyFuture(Box::new(future))
}
