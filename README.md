# parsec
[![Build Status](https://travis-ci.org/kvark/parsec.svg)](https://travis-ci.org/kvark/parsec)
[![Crates.io](https://img.shields.io/crates/v/parsec.svg?maxAge=2592000)](https://crates.io/crates/parsec)
[![Gitter](https://badges.gitter.im/kvark/parsec.svg)](https://gitter.im/kvark/parsec?utm_source=badge&utm_medium=badge&utm_campaign=pr-badge)

Features:
- Automatic execution of the systems in parallel. Follows Rust ownership rules. Depends on the order the systems are started.
- Component storage is abstract behind the trait. One can use vectors, hashmaps, trees, or whatever else.
- New components can be registered at any point from user modules. They don't have to be POD.
- No unsafe code! No virtual calls, low overhead.

Why is this fast?
- Do you know many other natively parallel ECS in Rust?
- Abstract storage means you can choose the most efficient one according to your needs. You can even roll in your own.
- No virtual calls during systems processing means you work with the data directly.
- [Benchmark for single- and multi-threaded performance](https://github.com/lschmierer/ecs_bench)

Why is this cool?
- Your system can be as simple as a closure working on some components, no redundant info or boilerplate is needed. At the same time, you can manually fiddle with entities and components, and it would still be safe and convenient.
- Your components can be anything (as long as they implement `Component`)! Neither `Copy` or even `Clone` bounds are needed.
- Your component storages can be anything. Consider crazy stuff like a BSP tree, or a database over the network. Some storages can safely allow sharing their components between entities, some can not - but it's up to you to choose.

Example entity creation:
```rust
let mut scheduler = {
    let mut w = parsec::World::new();
    // All components types should be registered before working with them
    w.register::<Position>();
    w.register::<Speed>();
    // create_now() of World provides with an EntityBuilder to add components to an Entity
    w.create_now().with(Position(0)).with(Speed(2)).build();
    w.create_now().with(Position(-1)).with(Speed(100)).build();
    w.create_now().with(Position(127)).build();
    // Scheduler is used to run systems on the specified world with a specified number of threads
    parsec::Scheduler::new(w, 4)
};
```

Example system:
```rust
schedule.run1w1r(|p: &mut Position, s: &Speed| {
    *p += *s;
});
```
