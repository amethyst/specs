//! Common functionality you might need when using Specs.
//!
//! At the moment, this module provides two types:
//!
//! * `Errors`: A resource you can use to store errors that occurred outside of
//!             the ECS but were catched inside, therefore should be handled by the user
//!
//! * `Merge`: A system generic over `T` which automatically merges `Ready` futures into
//!            the component storage for `T`.
//!
//! To make use of these features, you need to ask for the `common` feature
//! like this:
//!
//! ```toml
//! [dependencies.specs]
//! # version = "..."
//! features = ["common"]
//! ```

use std::convert::AsRef;
use std::error::Error;
use std::io::Write;
use std::marker::PhantomData;

use crossbeam::sync::MsQueue;
use futures::{Async, Future};
use futures::executor::{spawn, Notify, Spawn};

use {Component, Entities, Entity, Fetch, Join, RunningTime, System, WriteStorage};
use error::BoxedErr;

/// A boxed, thread-safe future with `T` as item and `BoxedErr` as error type.
pub type BoxedFuture<T> = Box<Future<Item = T, Error = BoxedErr> + Send + Sync + 'static>;

/// A draining iterator for `Errors`.
/// This is the return value of `Errors::drain`.
#[derive(Debug)]
pub struct DrainErrors<'a> {
    queue: &'a mut MsQueue<BoxedErr>,
}

impl<'a> Iterator for DrainErrors<'a> {
    type Item = BoxedErr;

    fn next(&mut self) -> Option<Self::Item> {
        self.queue.try_pop()
    }
}

/// A resource you can use to store errors that occurred outside of
/// the ECS but were catched inside, therefore should be handled by the user.
#[derive(Debug)]
pub struct Errors {
    /// The collection of errors.
    errors: MsQueue<BoxedErr>,
}

impl Errors {
    /// Creates a new instance of `Errors`.
    pub fn new() -> Self {
        Errors {
            errors: MsQueue::new(),
        }
    }

    /// Add an error to the error collection.
    pub fn add(&self, error: BoxedErr) {
        self.errors.push(error);
    }

    /// A convenience method which allows nicer error handling.
    ///
    /// ## Example
    ///
    /// ```
    /// # use specs::common::Errors;
    /// # let errors = Errors::new();
    /// # use std::io::{Error, ErrorKind};
    /// # fn do_something() -> Result<i32, Error> { Err(Error::new(ErrorKind::Other, "Other")) }
    /// errors.execute::<Error, _>(|| {
    ///     let y = do_something()?;
    ///     println!("{}", y + 5);
    ///
    ///     Ok(())
    /// });
    /// ```
    ///
    /// So the closure you pass to this method is essentially an environment where you can
    /// use `?` for error handling.
    pub fn execute<E, F>(&self, f: F)
    where
        E: Error + Send + Sync + 'static,
        F: FnOnce() -> Result<(), E>,
    {
        if let Err(e) = f() {
            self.add(BoxedErr::new(e));
        }
    }

    /// Checks if the queue contains at least one error.
    pub fn has_error(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Removes the first error from the queue.
    pub fn pop_err(&mut self) -> Option<BoxedErr> {
        self.errors.try_pop()
    }

    /// Returns a draining iterator, removing elements
    /// on each call to `Iterator::next`.
    pub fn drain(&mut self) -> DrainErrors {
        DrainErrors {
            queue: &mut self.errors,
        }
    }

    /// Collect all errors into a `Vec`.
    pub fn collect(&mut self) -> Vec<BoxedErr> {
        self.drain().collect()
    }

    /// Prints all errors and exits in case there's been an error. Useful for debugging.
    pub fn print_and_exit(&mut self) {
        use std::io::stderr;
        use std::process::exit;

        if self.errors.is_empty() {
            return;
        }

        let mut errors = self.collect();

        let stderr = stderr();
        let mut stderr = stderr.lock();

        writeln!(
            &mut stderr,
            "Exiting program because of {} errors...",
            errors.len()
        ).unwrap();

        for (ind, error) in errors.drain(..).enumerate() {
            let error = error.as_ref();

            writeln!(&mut stderr, "{}: {}", ind, error).unwrap();
        }

        exit(1);
    }
}

/// A system which merges `Ready` futures into the persistent storage.
/// Please note that your `World` has to contain a component storage
/// for `F` and `F::Item`.
///
/// In case of an error, it will be added to the `Errors` resource.
#[derive(Derivative)]
#[derivative(Default(bound = ""))]
pub struct Merge<F> {
    #[derivative(Default(value = "PhantomData"))]
    future_type: PhantomData<F>,
    spawns: Vec<(Entity, Spawn<F>)>,
}

impl<F> Merge<F> {
    /// Creates a new merge system.
    pub fn new() -> Self {
        Default::default()
    }
}

impl<'a, T, F> System<'a> for Merge<F>
where
    T: Component + Send + Sync + 'static,
    F: Future<Item = T, Error = BoxedErr> + Component + Send + Sync,
{
    type SystemData = (
        Entities<'a>,
        Fetch<'a, Errors>,
        WriteStorage<'a, F>,
        WriteStorage<'a, T>,
    );

    fn run(&mut self, (entities, errors, mut futures, mut pers): Self::SystemData) {
        for (e, _) in (&*entities, &futures.check()).join() {
            self.spawns.push((e, spawn(futures.remove(e).unwrap())));
        }

        retain_mut(&mut self.spawns, |spawn| {
            match spawn.1.poll_future_notify(NOTIFY_IGNORE, 0) {
                Ok(Async::NotReady) => true,
                Ok(Async::Ready(value)) => {
                    pers.insert(spawn.0, value);
                    false
                }
                Err(err) => {
                    errors.add(err);
                    false
                }
            }
        });
    }

    fn running_time(&self) -> RunningTime {
        RunningTime::Short
    }
}

struct NotifyIgnore;

impl Notify for NotifyIgnore {
    fn notify(&self, _: usize) {
        // Intentionally ignore
    }
}

static NOTIFY_IGNORE: &&NotifyIgnore = &&NotifyIgnore;


fn retain_mut<T, F>(vec: &mut Vec<T>, mut f: F)
where
    F: FnMut(&mut T) -> bool,
{
    let len = vec.len();
    let mut del = 0;
    {
        let v = &mut **vec;

        for i in 0..len {
            if !f(&mut v[i]) {
                del += 1;
            } else if del > 0 {
                v.swap(i - del, i);
            }
        }
    }
    if del > 0 {
        vec.truncate(len - del);
    }
}

#[cfg(test)]
mod test {
    use std::error::Error;
    use std::fmt::{Display, Formatter, Result as FmtResult};

    use futures::Poll;
    use futures::future::{result, Future, FutureResult};
    use futures::task;

    use Component;
    use DispatcherBuilder;
    use common::{BoxedErr, Errors, Merge};
    use storage::{NullStorage, VecStorage};
    use world::World;

    #[test]
    fn test_merge() {
        #[derive(Default)]
        struct TestComponent;

        impl Component for TestComponent {
            type Storage = NullStorage<Self>;
        }

        struct TestFuture {
            result: FutureResult<TestComponent, BoxedErr>,
        }

        impl Future for TestFuture {
            type Item = TestComponent;
            type Error = BoxedErr;

            fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
                task::current(); // This function called purely to see if we can.
                // Futures will expect to be able to call this function without panicking.
                self.result.poll()
            }
        }

        impl Component for TestFuture {
            type Storage = VecStorage<Self>;
        }

        #[derive(Debug)]
        struct TestError;

        impl Display for TestError {
            fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
                fmt.write_str("TestError")
            }
        }

        impl Error for TestError {
            fn description(&self) -> &str {
                "An error used for testing"
            }

            fn cause(&self) -> Option<&Error> {
                None
            }
        }

        let mut world = World::new();
        world.add_resource(Errors::new());
        world.register::<TestComponent>();
        world.register::<TestFuture>();
        let success = world
            .create_entity()
            .with(TestFuture {
                result: result(Ok(TestComponent)),
            })
            .build();
        let error = world
            .create_entity()
            .with(TestFuture {
                result: result(Err(BoxedErr::new(TestError))),
            })
            .build();

        let system: Merge<TestFuture> = Merge::new();

        let mut dispatcher = DispatcherBuilder::new().add(system, "merge", &[]).build();

        // Sequential dispatch used in order to avoid missing panics due to them happening in
        // another thread.
        dispatcher.dispatch_seq(&mut world.res);
        let components = world.read::<TestComponent>();
        assert!(components.get(success).is_some());
        assert!(components.get(error).is_none());
        assert_eq!(
            world.read_resource::<Errors>().errors.pop().description(),
            "An error used for testing"
        );
        assert!(world.read_resource::<Errors>().errors.try_pop().is_none());
    }
}
