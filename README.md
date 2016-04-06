# parsec
Parallel Systems for Entity Components

Features:
- Automatic execution of the systems in parallel. Follows Rust ownership rules. Depends on the order the systems are started.
- Component storage is abstract behind the trait. One can use vectors, hashmaps, trees, or whatever else.
- New components can be registered at any point from user modules. They don't have to be POD.
- No unsafe code! No virtual calls, low overhead.

Example system:
```rust
schedule.run1w1r(|p: &mut Position, s: &Speed| {
    *p += *s;
});
```
