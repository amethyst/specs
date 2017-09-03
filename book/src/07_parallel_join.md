# [DRAFT] Parallel Join

As mentioned in chapter dedicated on how to [dispatch][c3] systems
Specs automatically parallises execution of systems that have
non-conflicting component requirements (mutability XOR aliasing).

What isn't automatically parallised are the joins made within single system:

```rust,ignore
    fn run(&mut self, (vel, mut pos): Self::SystemData) {
        use specs::Join;
        // This loop is ran sequentially on a single thread.
        for (vel, pos) in (&vel, &mut pos).join() {
            pos.x += vel.x * 0.05;
            pos.y += vel.y * 0.05;
        }
    }
```

This means that if there are hundreds of thousands of entities and only few
systems that can be executed in parallel then the full power
of CPU cores will get wasted.

To fix this inefficiency and to parallelize joining the `join`
method call can be changed to `par_join` one:

```rust,ignore
    fn run(&mut self, (vel, mut pos): Self::SystemData) {
        use specs::ParJoin;
        use rayon::iter::ParallelIterator;

        // Parallel joining behaves similarly to normal joining
        // but now each iteration can potentially executed in parallel
        // by a thread pool same way as the systems are executed.
        (&vel, &mut pos).par_join()
            .for_each(|(vel, pos)| {
                pos.x += vel.x * 0.05;
                pos.y += vel.y * 0.05;
            });
    }
```

The `par_join` produces `rayon`s `ParallelIterator` which provides lots of
helper methods to manipulate iteration the same way as
the normal `Iterator` trait does.
