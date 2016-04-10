# parsec
[![Build Status](https://travis-ci.org/kvark/parsec.svg)](https://travis-ci.org/kvark/parsec)
[![Crates.io](https://img.shields.io/crates/v/parsec.svg?maxAge=2592000)](https://crates.io/crates/parsec)

Features:
- Automatic execution of the systems in parallel. Follows Rust ownership rules. Depends on the order the systems are started.
- Component storage is abstract behind the trait. One can use vectors, hashmaps, trees, or whatever else.
- New components can be registered at any point from user modules. They don't have to be POD.
- No unsafe code! No virtual calls, low overhead.

Why is this fast?
- Do you know many other natively parallel ECS in Rust?
- Abstract storage means you can choose the most efficient one according to your needs. You can even roll in your own.
- No virtual calls during systems processing means you work with the data directly.

Why is this cool?
- Your system can be as simple as a closure working on some components, no redundant info or boilerplate is needed. At the same time, you can manually fiddle with entities and components, and it would still be safe and convenient.
- Your components can be anything (as long as they implement `Component`)! Neither `Copy` or even `Clone` bounds are needed.
- Your component storages can be anything. Consider crazy stuff like a BSP tree, or a database over the network. Some storages can safely allow sharing their components between entities, some can not - but it's up to you to choose.

Example system:
```rust
schedule.run1w1r(|p: &mut Position, s: &Speed| {
    *p += *s;
});
```
