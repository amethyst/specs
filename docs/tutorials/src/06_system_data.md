# System Data

Every system can request data which it needs to run. This data can be specified
using the `System::SystemData` type. Typical implementors of the `SystemData` trait
are `ReadStorage`, `WriteStorage`, `Read`, `Write`, `ReadExpect`, `WriteExpect` and `Entities`.
A tuple of types implementing `SystemData` automatically also implements `SystemData`.
This means you can specify your `System::SystemData` as follows:

```rust
struct Sys;

impl<'a> System<'a> for Sys {
    type SystemData = (WriteStorage<'a, Pos>, ReadStorage<'a, Vel>);

    fn run(&mut self, (pos, vel): Self::SystemData) {
        /* ... */
    }
}
```

It is very important that you don't request both a `ReadStorage` and a `WriteStorage`
for the same component or a `Read` and a `Write` for the same resource.
This is just like the borrowing rules of Rust, where you can't borrow something
mutably and immutably at the same time. In Specs, we have to check this at
runtime, thus you'll get a panic if you don't follow this rule.

## Accessing Entities

You want to create/delete entities from a system? There is
good news for you. You can use `Entities` to do that.
It implements `SystemData` so just put it in your `SystemData` tuple.

> Don't confuse `specs::Entities` with `specs::EntitiesRes`.
  While the latter one is the actual resource, the former one is a type
  definition for `Read<Entities>`.

Please note that you may never write to these `Entities`, so only
use `Read`. Even though it's immutable, you can atomically create
and delete entities with it. Just use the `.create()` and `.delete()`
methods, respectively. After dynamic entity deletion,
a call to `World::maintain` is necessary in order to make the changes
persistent and delete associated components.

## Adding and removing components

Adding or removing components can be done by modifying
either the component storage directly with a `WriteStorage`
or lazily using the `LazyUpdate` resource.

```rust,ignore
use specs::{Component, Read, LazyUpdate, NullStorage, System, Entities, WriteStorage};

struct Stone;
impl Component for Stone {
    type Storage = NullStorage<Self>;
}

struct StoneCreator;
impl<'a> System<'a> for StoneCreator {
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Stone>,
        Read<'a, LazyUpdate>,
    );

    fn run(&mut self, (entities, mut stones, updater): Self::SystemData) {
        let stone = entities.create();

        // 1) Either we insert the component by writing to its storage
        stones.insert(stone, Stone);

        // 2) or we can lazily insert it with `LazyUpdate`
        updater.insert(stone, Stone);
    }
}
```

> **Note:** After using `LazyUpdate` a call to `World::maintain`
  is necessary to actually execute the changes.

## `SetupHandler` / `Default` for resources

Please refer to [the resources chapter for automatic creation of resources][c4].

[c4]: ./04_resources.html

## Specifying `SystemData`

As mentioned earlier, `SystemData` is implemented for tuples up to 26 elements. Should you ever need
more, you could even nest these tuples. However, at some point it becomes hard to keep track of all the elements.
That's why you can also create your own `SystemData` bundle using a struct:

```rust,ignore
extern crate specs;

use specs::prelude::*;

#[derive(SystemData)]
pub struct MySystemData<'a> {
    positions: ReadStorage<'a, Position>,
    velocities: ReadStorage<'a, Velocity>,
    forces: ReadStorage<'a, Force>,

    delta: Read<'a, DeltaTime>,
    game_state: Write<'a, GameState>,
}
```

Make sure to enable the `shred-derive` feature in your `Cargo.toml`:

```toml
specs = { version = "*", features = ["shred-derive"] }
```
