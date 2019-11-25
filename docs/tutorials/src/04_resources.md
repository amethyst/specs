# Resources

This (short) chapter will explain the concept of resources, data
which is shared between systems.

First of all, when would you need resources? There's actually a great
example in [chapter 3][c3], where we just faked the delta time when applying
the velocity. Let's see how we can do this the right way.

[c3]: ./03_dispatcher.html

```rust,ignore
#[derive(Default)]
struct DeltaTime(f32);
```

> **Note:** In practice you may want to use `std::time::Duration` instead,
  because you shouldn't use `f32`s for durations in an actual game, because
  they're not precise enough.

Adding this resource to our world is pretty easy:

```rust,ignore
world.insert(DeltaTime(0.05)); // Let's use some start value
```

To update the delta time, just use

```rust,ignore
use specs::WorldExt;

let mut delta = world.write_resource::<DeltaTime>();
*delta = DeltaTime(0.04);
```

## Accessing resources from a system

As you might have guessed, there's a type implementing system data
specifically for resources. It's called `Read` (or `Write` for
write access).

So we can now rewrite our system:

```rust,ignore
use specs::{Read, ReadStorage, System, WriteStorage};

struct UpdatePos;

impl<'a> System<'a> for UpdatePos {
    type SystemData = (Read<'a, DeltaTime>,
                       ReadStorage<'a, Velocity>,
                       WriteStorage<'a, Position>);

    fn run(&mut self, data: Self::SystemData) {
        let (delta, vel, mut pos) = data;

        // `Read` implements `Deref`, so it
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
`world.insert(resource)` before that system is run, or you will get a
panic. If the resource has a `Default` implementation, this step is usually
done during `setup`, but again we will come back to this in a later chapter.

For more information on `SystemData`, see [the system data chapter][cs].

## `Default` for resources

As we have learned in previous chapters, to fetch a `Resource` in our
`SystemData`, we use `Read` or `Write`. However, there is one issue we
have not mentioned yet, and that is the fact that `Read` and `Write` require
`Default` to be implemented on the resource. This is because Specs will
automatically try to add a `Default` version of a resource to the `World`
during `setup` (we will come back to the `setup` stage in the next chapter).
But how do we handle the case when we can't implement `Default` for our resource?

There are actually three ways of doing this:

* Using a custom `SetupHandler` implementation, you can provide this in `SystemData`
  with `Read<'a, Resource, TheSetupHandlerType>`.
* By replacing `Read` and `Write` with `ReadExpect` and `WriteExpect`, which will
  cause the first dispatch of the `System` to panic unless the resource has been
  added manually to `World` first.
* By using `Option<Read<'a, Resource>>`, if the resource really is optional. Note
  that the order here is important, using `Read<'a, Option<Resource>>` will not
  result in the same behavior (it will try to fetch `Option<Resource>` from `World`,
  instead of doing an optional check if `Resource` exists).


[cs]: ./06_system_data.html

---

In [the next chapter][c5], you will learn about the different storages
and when to use which one.

[c5]: 05_storages.html
