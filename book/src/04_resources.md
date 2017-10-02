# Resources

This (short) chapter will explain the concept of resources, data
which is shared between systems.

First of all, when would you need resources? There's actually a great
example in [chapter 3][c3], where we just faked the delta time when applying
the velocity. Let's see how we can do this the right way.

[c3]: ./03_dispatcher.html

```rust,ignore
struct DeltaTime(f32);
```

> **Note:** In practice you may want to use `std::time::Duration` instead,
  because you shouldn't use `f32`s for durations in an actual game, because
  they're not precise enough.

Adding this resource to our world is pretty easy:

```rust,ignore
world.add_resource(DeltaTime(0.05)); // Let's use some start value
```

To update the delta time, just use

```rust,ignore
let delta = world.write_resource::<DeltaTime>();
*delta = DeltaTime(0.04);
```
## Accessing resources from a system

As you might have guessed, there's a type implementing system data
specifically for resources. It's called `Fetch` (or `FetchMut` for
write access).

So we can now rewrite our system:

```rust,ignore
use specs::{Fetch, ReadStorage, System, WriteStorage};

struct UpdatePos;

impl<'a> System<'a> for UpdatePos {
    type SystemData = (Fetch<'a, DeltaTime>,
                       ReadStorage<'a, Velocity>,
                       WriteStorage<'a, Position>);

    fn run(&mut self, data: Self::SystemData) {
        let (delta, vel, mut pos) = data;

        // `Fetch` implements `Deref`, so it
        // coerces to `&DeltaTime`.
        let delta = delta.0;

        for (vel, pos) in (&vel, &mut pos).join() {
            pos.x += vel.x * delta;
            pos.y += vel.y * delta;
        }
    }
}
```

Note that all resources that a system accesses must be registered with
`world.add_resource(resource)` before that system is run, or you will get a
panic.

For more information on `SystemData`, see [the system data chapter][cs].

[cs]: ./06_system_data.html

---

In [the next chapter][c5], you will learn about the different storages
and when to use which one.

[c5]: 05_storages.html
