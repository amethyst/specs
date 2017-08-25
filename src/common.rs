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

use std::cell::RefCell;
use std::convert::AsRef;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::Write;
use std::marker::PhantomData;

use futures::{Async, Future};
use futures::executor::{Notify, spawn, Spawn};

use {Component, Entity, Entities, FetchMut, Join, RunningTime, System, WriteStorage};

/// A boxed error implementing `Debug`, `Display` and `Error`.
pub struct BoxedErr(pub Box<Error + Send + Sync + 'static>);

impl BoxedErr {
    /// Creates a new boxed error.
    pub fn new<T>(err: T) -> Self
        where T: Error + Send + Sync + 'static
    {
        BoxedErr(Box::new(err))
    }
}

impl AsRef<Error> for BoxedErr {
    fn as_ref(&self) -> &(Error + 'static) {
        self.0.as_ref()
    }
}

impl Debug for BoxedErr {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{:?}", self.as_ref())
    }
}

impl Display for BoxedErr {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.as_ref())
    }
}

impl Error for BoxedErr {
    fn description(&self) -> &str {
        self.as_ref().description()
    }
}

/// A boxed, thread-safe future with `T` as item and `BoxedErr` as error type.
pub type BoxedFuture<T> = Box<Future<Item = T, Error = BoxedErr> + Send + Sync + 'static>;

/// A resource you can use to store errors that occurred outside of
/// the ECS but were catched inside, therefore should be handled by the user.
#[derive(Debug)]
pub struct Errors {
    /// The collection of errors.
    pub errors: Vec<BoxedErr>,
}

impl Errors {
    /// Creates a new instance of `Errors`.
    pub fn new() -> Self {
        Errors { errors: Vec::new() }
    }

    /// Add an error to the error `Vec`.
    pub fn add(&mut self, error: BoxedErr) {
        self.errors.push(error);
    }

    /// Prints all errors and exits in case there's been an error. Useful for debugging.
    pub fn print_and_exit(&mut self) {
        use std::io::stderr;
        use std::process::exit;

        if self.errors.is_empty() {
            return;
        }

        let stderr = stderr();
        let mut stderr = stderr.lock();

        writeln!(&mut stderr,
                 "Exiting program because of {} errors...",
                 self.errors.len()).unwrap();

        for (ind, error) in self.errors.drain(..).enumerate() {
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
pub struct Merge<F> {
    future_type: PhantomData<F>,
    spawns: Vec<RefCell<(Entity, Spawn<F>)>>,
}

impl<F> Merge<F> {
    /// Creates a new merge system.
    pub fn new() -> Self {
        Merge {
            future_type: PhantomData,
            spawns: Vec::new(),
        }
    }
}

struct NotifyIgnore;
impl Notify for NotifyIgnore {
    fn notify(&self, _: usize) {
        /* Intetionally ignore */
    }
}

static NOTIFY_IGNORE: &&'static NotifyIgnore = &&NotifyIgnore;

impl<'a, T, F> System<'a> for Merge<F>
    where T: Component + Send + Sync + 'static,
          F: Future<Item = T, Error = BoxedErr> + Component + Send + Sync,
{
    type SystemData = (Entities<'a>,
                       FetchMut<'a, Errors>,
                       WriteStorage<'a, F>,
                       WriteStorage<'a, T>);

    fn run(&mut self, (entities, mut errors, mut futures, mut pers): Self::SystemData) {

        for (e, ()) in (&*entities, &futures.check()).join() {
            self.spawns.push(RefCell::new((e, spawn(futures.remove(e).unwrap()))));
        }

        self.spawns.retain(|spawn| {
            let mut spawn = spawn.borrow_mut();
            match (*spawn).1.poll_future_notify(NOTIFY_IGNORE, 0) {
                Ok(Async::NotReady) => {
                    true
                }
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
