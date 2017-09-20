# Specs

> **S**pecs **P**arallel **ECS**

[![Build Status][bi]][bl] [![Crates.io][ci]][cl] [![Gitter][gi]][gl] ![MIT/Apache][li] [![Docs.rs][di]][dl] [![Coverage][cci]][ccl]

[bi]: https://travis-ci.org/slide-rs/specs.svg?branch=master
[bl]: https://travis-ci.org/slide-rs/specs

[ci]: https://img.shields.io/crates/v/specs.svg
[cl]: https://crates.io/crates/specs/

[li]: https://img.shields.io/badge/license-Apache%202.0-blue.svg

[di]: https://docs.rs/specs/badge.svg
[dl]: https://docs.rs/specs/

[gi]: https://badges.gitter.im/slide-rs/specs.svg
[gl]: https://gitter.im/slide-rs/specs

[cci]: https://coveralls.io/repos/github/slide-rs/specs/badge.svg?branch=master
[ccl]: https://coveralls.io/github/slide-rs/specs?branch=master

Specs is an Entity-Component System written in Rust.
Unlike most other ECS libraries out there, it provides

* easy parallelism
* high flexibility
    * contains 5 different storages for components, which can be extended by the user
    * its types are mostly not coupled, so you can easily write some part yourself and
      still use Specs
    * `System`s may read from and write to components and resources, can depend on each
      other and you can use barriers to force several stages in system execution
* high performance for real-world applications

## Example

```rust
// A component contains data
// which is associated with an entity.

#[derive(Debug)]
struct Vel(f32);

impl Component for Vel {
    type Storage = VecStorage<Self>;
}

#[derive(Debug)]
struct Pos(f32);

impl Component for Pos {
    type Storage = VecStorage<Self>;
}

struct SysA;

impl<'a> System<'a> for SysA {
    // These are the resources required for execution.
    // You can also define a struct and `#[derive(SystemData)]`,
    // see the `full` example.
    type SystemData = (WriteStorage<'a, Pos>, ReadStorage<'a, Vel>);

    fn run(&mut self, (mut pos, vel): Self::SystemData) {
        // The `.join()` combines multiple components,
        // so we only access those entities which have
        // both of them.
        for (pos, vel) in (&mut pos, &vel).join() {
            pos.0 += vel.0;
        }
    }
}

fn main() {
    // The `World` is our
    // container for components
    // and other resources.
    let mut world = World::new();
    world.register::<Pos>();
    world.register::<Vel>();

    // An entity may or may not contain some component.

    world.create_entity().with(Vel(2.0)).with(Pos(0.0)).build();
    world.create_entity().with(Vel(4.0)).with(Pos(1.6)).build();
    world.create_entity().with(Vel(1.5)).with(Pos(5.4)).build();

    // This entity does not have `Vel`, so it won't be dispatched.
    world.create_entity().with(Pos(2.0)).build();

    // This builds a dispatcher.
    // The third parameter of `add` specifies
    // logical dependencies on other systems.
    // Since we only have one, we don't depend on anything.
    // See the `full` example for dependencies.
    let mut dispatcher = DispatcherBuilder::new().add(SysA, "sys_a", &[]).build();

    // This dispatches all the systems in parallel (but blocking).
    dispatcher.dispatch(&mut world.res);
}
```

Please look into [the examples directory](examples) for more.

## Contribution

Contribution is very welcome! If you didn't contribute before, just
filter for issues with "easy" label. Please note that your contributions
are assumed to be dual-licensed under Apache-2.0/MIT.
