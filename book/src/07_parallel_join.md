# [DRAFT] Parallel Join

As mentioned in chapeter dedicated to [dispatching systems][c3]
Specs automatically parallises systems that have non-conflicting component
requirements (mutability XOR aliasing).

What Specs doesn't automatically parallelize is joins within single system:

```rust,ignore
    fn run(&mut self, (vel, mut pos): Self::SystemData) {
        use specs::Join;
        // This loop is ran sequentially on single thread.
        for (vel, pos) in (&vel, &mut pos).join() {
            pos.x += vel.x * 0.05;
            pos.y += vel.y * 0.05;
        }
    }
```

This means that if there is hundreds of thousands of entities and only few
systems that can be executed in parallel then the full power of CPU is wasted.

This can be fixed by changing `join` to `par_join`
which produces `rayon`s `ParallelIterator`:

```rust,ignore
    fn run(&mut self, (vel, mut pos): Self::SystemData) {
        use specs::ParJoin;
        use rayon::iter::ParallelIterator;

        // When using `par_join` the entities that have all joined components
        // are split into jobs that can be executed in parallel in same way
        // as systems are automatically executed.
        (&vel, &mut pos).par_join()
            .for_each(|(vel, pos)| {
                pos.x += vel.x * 0.05;
                pos.y += vel.y * 0.05;
            });
    }
```
