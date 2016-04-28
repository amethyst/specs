# specs
[![Build Status](https://travis-ci.org/slide-rs/specs.svg)](https://travis-ci.org/slide-rs/specs)
[![Crates.io](https://img.shields.io/crates/v/specs.svg?maxAge=2592000)](https://crates.io/crates/specs)
[![Gitter](https://badges.gitter.im/slide-rs/specs.svg)](https://gitter.im/slide-rs/specs?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)

Specs is an Entity-Component System written in Rust. It aims for parallel systems execution with high ergonomics and flexibility. The name can be interpret in a number of ways:
- "SPECS Parallel ECS"
- "Super Powerful ECS"
- "Special ECS"


#### Classification
According to [ECS Design Crossroads](https://github.com/amethyst/amethyst/wiki/ECS-Design-Crossroads), `specs` fulfills all the requirements, has _In-place modification_ updates, and _Generational ID_ entities.

#### Features
- Automatic execution of the systems in parallel. Follows Rust ownership rules, where each component storage behaves as a variable. Depends on the order, in which systems are started.
- Component storage is abstract behind the trait. One can use vectors, hashmaps, trees, or whatever else.
- New components can be registered at any point from user modules. They don't have to be POD.
- No virtual calls, low overhead.

#### Why is it fast
- Do you know many other natively parallel ECS in Rust?
- Abstract storage means you can choose the most efficient one according to your needs. You can even roll in your own.
- No virtual calls during systems processing means you work with the data directly.

See [ecs_bench](https://github.com/lschmierer/ecs_bench) for single- and multi-threaded performance comparisons.

#### Why is it cool
- Your system can be as simple as a closure working on some components, no redundant info or boilerplate is needed. At the same time, you can manually fiddle with entities and components, and it would still be safe and convenient.
- Your components can be anything (as long as they implement `Component`)! Neither `Copy` or even `Clone` bounds are needed.
- Your component storages can be anything. Consider crazy stuff like a BSP tree, or a database over the network. Some storages can safely allow sharing their components between entities, some can not - but it's up to you to choose.

## Examples
Entity creation:
```rust
let mut planner = {
    let mut w = specs::World::new();
    // All components types should be registered before working with them
    w.register::<Position>();
    w.register::<Speed>();
    // create_now() of World provides with an EntityBuilder to add components to an Entity
    w.create_now().with(Position(0)).with(Speed(2)).build();
    w.create_now().with(Position(-1)).with(Speed(100)).build();
    w.create_now().with(Position(127)).build();
    // Planner is used to run systems on the specified world with a specified number of threads
    specs::Planner::new(w, 4)
};
```

System run:
```rust
planner.run1w1r(|p: &mut Position, s: &Speed| {
    *p += *s;
});
```
Custom system:
```rust
impl System<u32> for MySystem {
    fn run(&mut self, arg: RunArg, context: u32) {
        let mut numbers = arg.fetch(|w| w.write::<u32>());
        for n in (&numbers).iter() {
            *n += context;
        }
    }
}
```