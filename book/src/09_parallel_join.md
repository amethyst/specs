# Parallel Join

As mentioned in the chapter dedicated to how to [dispatch][c3] systems,
Specs automatically parallelizes system execution when there are non-conflicting
system data requirements (Two `System`s conflict if their `SystemData` needs access
to the same resource where at least one of them needs write access to it).

[c3]: ./03_dispatcher.html

## Basic parallelization

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

This means that, if there are hundreds of thousands of entities and only a few
systems that actually can be executed in parallel, then the full power
of CPU cores cannot be fully utilized.

To fix this potential inefficiency and to parallelize the joining, the `join`
method call can be exchanged for `par_join`:

```rust,ignore
fn run(&mut self, (vel, mut pos): Self::SystemData) {
    use rayon::prelude::*;
    use specs::ParJoin;

    // Parallel joining behaves similarly to normal joining
    // with the difference that iteration can potentially be
    // executed in parallel by a thread pool.
    (&vel, &mut pos)
        .par_join()
        .for_each(|(vel, pos)| {
            pos.x += vel.x * 0.05;
            pos.y += vel.y * 0.05;
        });
}
```

> There is always overhead in parallelization, so you should carefully profile to see if there are benefits in the
  switch. If you have only a few things to iterate over then sequential join is faster.

The `par_join` method produces a type implementing rayon's [`ParallelIterator`][ra]
trait which provides lots of helper methods to manipulate the iteration,
the same way the normal `Iterator` trait does.

[ra]: https://docs.rs/rayon/1.0.0/rayon/iter/trait.ParallelIterator.html
