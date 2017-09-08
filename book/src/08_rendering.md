# Rendering

Rendering is often a little bit tricky when you're dealing with a multi-threaded ECS.
That's why we have something called "thread-local systems".

There are two things to keep in mind about thread-local systems:

1) They're always executed at the end of dispatch
2) They cannot have dependencies, you just add them in the order you want them to run

Adding one is a simple line added to the builder code:

```rust,ignore
DispatcherBuilder::new()
    .add_thread_local(RenderSys);
```

## Amethyst

As for Amethyst, it's very easy because Specs is already integrated. So there's no special effort
required, just look at the current examples.

## Piston

Piston has an event loop which looks like this:

```rust,ignore
while let Some(event) = window.poll_event() {
    // Handle event
}
```

Now, we'd like to do as much as possible in the ECS, so we feed in input as a
[resource](./04_resources.html).
This is what your code could look like:

```rust,ignore
struct ResizeEvents(Vec<(u32, u32)>);

world.add_resource(ResizeEvents(Vec::new()));

while let Some(event) = window.poll_event() {
    match event {
        Input::Resize(x, y) => world.write_resource::<ResizeEvents>().0.push((x, y)),
        // ...
    }
}
```

The actual dispatching should happen every time the `Input::Update` event occurs.

---

> If you want a section for your game engine added, feel free to submit a PR!
