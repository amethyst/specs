# [DRAFT] Parallel Join

As mentioned in the chapter dedicated on how to [dispatch][c3] systems,
Specs automatically parallelizes system execution when there is non-conflicting
component requirements between them (mutability XOR aliasing).

[c3]: ./03_dispatcher.html

What isn't automatically parallelized by Specs are
the joins made within a single system:

```rust,ignore
    fn run(&mut self, (vel, mut pos): Self::SystemData) {
        use specs::Join;
        // This loop runs sequentially on a single thread.
        for (vel, pos) in (&vel, &mut pos).join() {
            pos.x += vel.x * 0.05;
            pos.y += vel.y * 0.05;
        }
    }
```

This means that, if there are hundreds of thousands of entities and only few
systems that actually can be executed in parallel, then the full power
of CPU cores cannot be fully utilized.

To fix this inefficiency and to parallelize the joining, the `join`
method call can be exchanged for `par_join`:

```rust,ignore
    fn run(&mut self, (vel, mut pos): Self::SystemData) {
        use specs::ParJoin;
        use rayon::iter::ParallelIterator;

        // Parallel joining behaves similarly to normal joining
        // with the difference that iteration can potentially be
        // executed in parallel by a thread pool—the same way as
        // the systems are executed.
        (&vel, &mut pos).par_join()
            .for_each(|(vel, pos)| {
                pos.x += vel.x * 0.05;
                pos.y += vel.y * 0.05;
            });
    }
```

The `par_join` method produces a type implementing [`rayon`s `ParallelIterator`][ra]
trait which provides lots of helper methods to manipulate the iteration,
the same way as the normal `Iterator` trait does.

[ra]: https://crates.io/crates/rayon
