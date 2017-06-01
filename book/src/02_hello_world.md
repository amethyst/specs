# Hello, `World`!

## Setting up

First of all, thanks for trying out `specs`. Let's
set it up first. Add the following line to your `Cargo.toml`:

```toml
specs = "0.9"
```

And add this to your crate root (`main.rs` or `lib.rs`):

```rust,ignore
extern crate specs;
```

## Components

Let's start by creating some data:

```rust,ignore
use specs::{Component, VecStorage};

#[derive(Debug)]
struct Position { 
    x: f32, 
    y: f32 
}

impl Component for Position {
    type Storage = VecStorage<Position>;
}

#[derive(Debug)]
struct Velocity {
    x: f32,
    y: f32,
}

impl Component for Velocity {
    type Storage = VecStorage<Velocity>;
}
```

This will be our component, stored in a `VecStorage` 
(see [the storages chapter][sc] for more details), so some data we can associate
with an entity. Before doing that, we need to create a world, because this is
where the storage for all the components is located.

[sc]: ./05_storages.html

## The `World`

```rust,ignore
use specs::World;

let mut world = World::new();
world.register::<Position>();
```

This will create a component storage for `Position`s.

```rust,ignore
let ball = world.create_entity().with(Position { x: 4.0, y: 7.0 }).build();
```

Now you have an `Entity`, associated with a position.

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

This is how a system looks like. That is, a system which
does not do anything (yet). Let's talk about the above dummy
implementation first. The `SystemData` is an associated type
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

> There are many other types you can use as system
  data. Please see the [System Data Chapter][cs] for more
  information.

[cs]: ./06_system_data.html

## Running the system

This just iterates through all the components and prints
them. To execute the system, you can use `run_now` like this:

```rust,ignore
use specs::run_now;

let hello_world = HelloWorld;
run_now(hello_world, &mut world.res);
```

## Full example code

Here the complete example of this chapter:

```rust,ignore
use specs::{Component, ReadStorage, System, VecStorage, World};

#[derive(Debug)]
struct Position { 
    x: f32, 
    y: f32 
}

impl Component for Position {
    type Storage = VecStorage<Position>;
}

#[derive(Debug)]
struct Velocity {
    x: f32,
    y: f32,
}

impl Component for Velocity {
    type Storage = VecStorage<Velocity>;
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
    
    world.create_entity().with(Position { x: 4.0, y: 7.0 }).build();
    
    let hello_world = HelloWorld;
    run_now(hello_world, &mut world.res);
}
```

> **Note:** You can use `use specs::prelude::*;` to include all
  the common types.

---

This was a pretty basic example so far. A key feature we
haven't seen is the `Dispatcher`, which allows to
configure run systems in parallel (and it offers some other
nice features, too).

Let's see how that works in [Chapter 3: Dispatcher][c3].

[c3]: ./03_dispatcher.html
