# Dispatcher

## When to use a `Dispatcher`

The `Dispatcher` allows you to automatically parallelize
system execution where possible, using the [fork-join model][fj] to split up the
work and merge the result at the end. It requires a bit more planning
and may have a little bit more overhead, but it's pretty convenient,
especially when you're building a big game where you don't
want to do this manually.

[fj]: https://en.wikipedia.org/wiki/Fork–join_model

## Building a dispatcher

First of all, we have to build such a dispatcher.

```rust,ignore
use specs::DispatcherBuilder;

let mut dispatcher = DispatcherBuilder::new()
    .add(hello_world, "hello_world", &[])
    .build();
```

Let's see what this does. After creating the builder,
we add a new

1) system object (`hello_world`)
2) with some name (`"hello_world""`)
3) and no dependencies (`&[]`).

The name can be used to specify that system
as a dependency of another one. But we don't have a second
system yet.

## Creating another system

```rust,ignore
struct UpdatePos;

impl<'a> System<'a> for UpdatePos {
    type SystemData = (ReadStorage<'a, Velocity>,
                       WriteStorage<'a, Position>);
}
```

Let's talk about the system data first. What you see here
is a **tuple**, which we are using as our `SystemData`.
In fact, `SystemData` is implemented for all tuples
with up to 26 other types implementing `SystemData` in it.

> Notice that `ReadStorage` and `WriteStorage` *are* implementors of `SystemData`
  themselves, that's why we could use the first one for our `HelloWorld` system
  without wrapping it in a tuple; for more information see
  [the Chapter about system data][cs].

[cs]: ./06_system_data.html

To complete the implementation block, here's the `run` method:

```rust,ignore
    fn run(&mut self, (vel, mut pos): Self::SystemData) {
        use specs::Join;
        for (vel, pos) in (&vel, &mut pos).join() {
            pos.x += vel.x * 0.05;
            pos.y += vel.y * 0.05;
        }
    }
```

Now the `.join()` method also makes sense: it joins the two component
storages, so that you either get no new element or a new element with
both components, meaning that entities with only a `Position`, only
a `Velocity` or none of them will be skipped. The `0.05` fakes the
so called **delta time** which is the time needed for one frame.
We have to hardcode it right now, because it's not a component (it's the
same for every entity). The solution to this are `Resource`s, see
[the next Chapter][c4].

[c4]: ./04_resources.html

## Adding a system with a dependency

Okay, now you can add another system *after* the `HelloWorld` system:

```rust,ignore
    .add(UpdatePos, "update_pos", &["hello_world"])
```

The `UpdatePos` system now depends on the `HelloWorld` system and will only
be executed after the dependency has finished.

Now to execute all the systems, just do

```rust,ignore
dispatcher.dispatch(&mut world.res);
```

## Full example code

Here the code for this chapter:

```rust,ignore
use specs::{Component, DispatcherBuilder, ReadStorage,
            System, VecStorage, World, WriteStorage};

#[derive(Debug)]
struct Position {
    x: f32,
    y: f32
}

impl Component for Position {
    type Storage = VecStorage<Self>;
}

#[derive(Debug)]
struct Velocity {
    x: f32,
    y: f32,
}

impl Component for Velocity {
    type Storage = VecStorage<Self>;
}

struct HelloWorld;

impl<'a> System<'a> for HelloWorld {
    type SystemData = ReadStorage<'a, Position>;

    fn run(&mut self, position: Self::SystemData) {
        use specs::Join;

        for position in position.join() {
            println!("Hello, {:?}", &position);
        }
    }
}

struct UpdatePos;

impl<'a> System<'a> for UpdatePos {
    type SystemData = (ReadStorage<'a, Velocity>,
                       WriteStorage<'a, Position>);

    fn run(&mut self, (vel, mut pos): Self::SystemData) {
        use specs::Join;
        for (vel, pos) in (&vel, &mut pos).join() {
            pos.x += vel.x * 0.05;
            pos.y += vel.y * 0.05;
        }
    }
}

fn main() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    // Only the second entity will get a position update,
    // because the first one does not have a velocity.
    world.create_entity().with(Position { x: 4.0, y: 7.0 }).build();
    world
        .create_entity()
        .with(Position { x: 2.0, y: 5.0 })
        .with(Velocity { x: 0.1, y: 0.2 })
        .build();

    let mut dispatcher = DispatcherBuilder::new()
        .add(HelloWorld, "hello_world", &[])
        .add(UpdatePos, "update_pos", &["hello_world"])
        .build();

    dispatcher.dispatch(&mut world.res);
}
```

---

[The next chapter][c4] will be a really short chapter about `Resource`s,
a way to share data between systems which do only exist independent of
entities (as opposed to 0..1 times per entity).
