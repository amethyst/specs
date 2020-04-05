# Hello, `World`!

## Setting up

First of all, thanks for trying out `specs`.
Before setting up the project, please make sure you're using the latest Rust version:

```bash
rustup update
```

Okay, now let's set up the project!

```bash
cargo new --bin my_game
```

Add the following line to your `Cargo.toml`:

```toml
[dependencies]
specs = "0.16.1"
```

## Components

Let's start by creating some data:

```rust,ignore
use specs::{Component, VecStorage};

#[derive(Debug)]
struct Position {
    x: f32,
    y: f32,
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
```

These will be our two component types. Optionally, the `specs-derive` crate
provides a convenient custom `#[derive]` you can use to define component types
more succinctly.

But first, you will need to enable the `derive` feature:

```toml
[dependencies]
specs = { version = "0.16.1", features = ["specs-derive"] }
```

Now you can do this:

```rust,ignore
use specs::{Component, VecStorage};

#[derive(Component, Debug)]
#[storage(VecStorage)]
struct Position {
    x: f32,
    y: f32,
}

#[derive(Component, Debug)]
#[storage(VecStorage)]
struct Velocity {
    x: f32,
    y: f32,
}
```

If the `#[storage(...)]` attribute is omitted, the given component will be
stored in a `DenseVecStorage` by default. But for this example, we are
explicitly asking for these components to be kept in a `VecStorage` instead (see
the later [storages chapter][sc] for more details). But before we move on, we
need to create a world in which to store all of our components.

[sc]: ./05_storages.html

## The `World`

```rust,ignore
use specs::{World, WorldExt, Builder};

let mut world = World::new();
world.register::<Position>();
world.register::<Velocity>();
```

This will create component storages for `Position`s and `Velocity`s.

```rust,ignore
let ball = world.create_entity().with(Position { x: 4.0, y: 7.0 }).build();
```

Now you have an `Entity`, associated with a position.

> **Note:** `World` is a struct coming from `shred`, an important dependency of Specs.
> Whenever you call functions specific to Specs, you will need to import the `WorldExt`
> trait.

So far this is pretty boring. We just have some data,
but we don't do anything with it. Let's change that!

## The system

```rust,ignore
use specs::System;

struct HelloWorld;

impl<'a> System<'a> for HelloWorld {
    type SystemData = ();

    fn run(&mut self, data: Self::SystemData) {}
}
```

This is what a system looks like. Though it doesn't do anything (yet).
Let's talk about this dummy implementation first.
The `SystemData` is an associated type
which specifies which components we need in order to run
the system.

Let's see how we can read our `Position` components:

```rust,ignore
use specs::{ReadStorage, System};

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
```

Note that all components that a system accesses must be registered with
`world.register::<Component>()` before that system is run, or you will get a
panic. This will usually be done automatically during `setup`, but we'll
come back to that in a later [chapter][se].

> There are many other types you can use as system data. Please see the
> [System Data Chapter][cs] for more information.

[cs]: ./06_system_data.html
[se]: ./07_setup.html

## Running the system

This just iterates through all the components and prints
them. To execute the system, you can use `RunNow` like this:

```rust,ignore
use specs::RunNow;

let mut hello_world = HelloWorld;
hello_world.run_now(&world);
world.maintain();
```

The `world.maintain()` is not completely necessary here. Calling maintain should be done in general, however.
If entities are created or deleted while a system is running, calling `maintain`
will record the changes in its internal data structure.

## Full example code

Here the complete example of this chapter:

```rust,ignore
use specs::{Builder, Component, ReadStorage, System, VecStorage, World, WorldExt, RunNow};

#[derive(Debug)]
struct Position {
    x: f32,
    y: f32,
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

fn main() {
    let mut world = World::new();
    world.register::<Position>();
    world.register::<Velocity>();

    world.create_entity().with(Position { x: 4.0, y: 7.0 }).build();

    let mut hello_world = HelloWorld;
    hello_world.run_now(&world);
    world.maintain();
}
```

---

This was a pretty basic example so far. A key feature we haven't seen is the
`Dispatcher`, which allows us to configure systems to run in parallel (and it offers
some other nice features, too).

Let's see how that works in [Chapter 3: Dispatcher][c3].

[c3]: ./03_dispatcher.html
