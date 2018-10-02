# The `setup` stage

So far for all our component storages and resources, we've been adding
them to the `World` manually. In Specs, this is not required if you use
`setup`. This is a manually invoked stage that goes through `SystemData`
and calls `register`, `add_resource`, etc. for all (with some exceptions)
components and resources found. The `setup` function can be found in
the following locations:

* `ReadStorage`, `WriteStorage`, `Read`, `Write`
* `SystemData`
* `System`
* `RunNow`
* `Dispatcher`
* `ParSeq`

During setup, all components encountered will be registered, and all
resources that have a `Default` implementation or a custom `SetupHandler`
will be added. Note that resources encountered in `ReadExpect` and `WriteExpect`
will not be added to the `World` automatically.

The recommended way to use `setup` is to run it on `Dispatcher` or `ParSeq`
after the system graph is built, but before the first `dispatch`. This will go
through all `System`s in the graph, and call `setup` on each.

Let's say you began by registering Components and Resources first:

```rust,ignore

struct Gravity;

struct Velocity;

impl Component for Position {
    type Storage = VecStorage<Self>;
}

struct SimulationSystem;

impl<'a> System<'a> for SimulationSystem {
    type SystemData = (Read<'a, Gravity>, WriteStorage<'a, Velocity>);

    fn run(_, _) {}
}

fn main() {
    let mut world = World::new();
    world.add_resource(Gravity);
    world.register::<Velocity>();

    for _ in 0..5 {
        world.create_entity().with(Velocity).build();
    }

    let mut dispatcher = DispatcherBuilder::new()
        .with(SimulationSystem, "simulation", &[])
        .build();

    dispatcher.dispatch(&mut world.res);
    world.maintain();
}

```

You could get rid of that phase by calling `setup()` and re-ordering your main function:

```rust,ignore
fn main() {
    let mut world = World::new();
    let mut dispatcher = DispatcherBuilder::new()
        .with(SimulationSystem, "simulation", &[])
        .build();

    dispatcher.setup(&mut world.res);

    for _ in 0..5 {
        world.create_entity().with(Velocity).build();
    }

    dispatcher.dispatch(&mut world.res);
    world.maintain();
}

```


## Custom `setup` functionality

The good qualities of `setup` don't end here however. We can also use `setup`
to create our non-`Default` resources, and also to initialize our `System`s!
We do this by custom implementing the `setup` function in our `System`.

Let's say we have a `System` that process events, using `shrev::EventChannel`:

```rust,ignore
struct Sys {
    reader: ReaderId<Event>,
}

impl<'a> System<'a> for Sys {
    type SystemData = Read<'a, EventChannel<Event>>;

    fn run(&mut self, events: Self::SystemData) {
        for event in events.read(&mut self.reader) {
            [..]
        }
    }
}
```

This looks pretty OK, but there is a problem here if we want to use `setup`.
The issue is that `Sys` needs a `ReaderId` on creation, but to get a `ReaderId`,
we need `EventChannel<Event>` to be initialized. This means the user of `Sys` need
to create the `EventChannel` themselves and add it manually to the `World`.
We can do better!

```rust,ignore
use specs::prelude::Resources;

#[derive(Default)]
struct Sys {
    reader: Option<ReaderId<Event>>,
}

impl<'a> System<'a> for Sys {
    type SystemData = Read<'a, EventChannel<Event>>;

    fn run(&mut self, events: Self::SystemData) {
        for event in events.read(&mut self.reader.as_mut().unwrap()) {
            [..]
        }
    }

    fn setup(&mut self, res: &mut Resources) {
        use specs::prelude::SystemData;
        Self::SystemData::setup(res);
        self.reader = Some(res.fetch_mut::<EventChannel<Event>>().register_reader());
    }
}
```

This is much better; we can now use `setup` to fully initialize `Sys` without
requiring our users to create and add resources manually to `World`!

**If we override the `setup` function on a `System`, it is vitally important that we
remember to add `Self::SystemData::setup(res);`, or setup will not be performed for
the `System`s `SystemData`.** This could cause panics during setup or during
the first dispatch.

## Setting up in bulk

It's generally recommended to provide a standalone function to register multiple
Components/Resources at once, while allowing the user to add individual systems
by themselves.

```rust,ignore
fn add_physics_engine(world: &mut World, config: LibraryConfig) -> Result<(), LibraryError> {
    world.register::<Velocity>();
    // etc
}
```
