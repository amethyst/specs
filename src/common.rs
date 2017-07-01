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
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::io::Write;
use std::marker::PhantomData;

use futures::{Async, Future};
use {Component, DenseVecStorage, Entity, Entities, FetchMut, Join, RunningTime, System,
     WriteStorage};

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

/// The component type for futures of `T`.
pub type FutureComponent<T> = Box<Future<Item = T, Error = BoxedErr> + Send + Sync>;

impl<T> Component for FutureComponent<T>
    where T: Send + Sync + 'static
{
    type Storage = DenseVecStorage<FutureComponent<T>>;
}

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
/// for `T`.
///
/// In case of an error, it will be added to the `Errors` resource.
pub struct Merge<T> {
    asset_type: PhantomData<T>,
    tmp: Vec<Entity>,
}

impl<T> Merge<T> {
    /// Creates a new merge system.
    pub fn new() -> Self {
        Merge {
            asset_type: PhantomData,
            tmp: Vec::new(),
        }
    }
}

impl<'a, T> System<'a> for Merge<T>
    where T: Component + Send + Sync + 'static
{
    type SystemData = (Entities<'a>,
                       FetchMut<'a, Errors>,
                       WriteStorage<'a, FutureComponent<T>>,
                       WriteStorage<'a, T>);

    fn run(&mut self, (entities, mut errors, mut future, mut pers): Self::SystemData) {
        let mut delete = &mut self.tmp;

        for (e, future) in (&*entities, &mut future).join() {
            match future.poll() {
                Ok(Async::Ready(x)) => {
                    pers.insert(e, x);
                    delete.push(e);
                }
                Ok(Async::NotReady) => {}
                Err(err) => {
                    errors.add(err);
                    delete.push(e);
                }
            }
        }

        for e in delete.drain(..) {
            future.remove(e);
        }
    }

    fn running_time(&self) -> RunningTime {
        RunningTime::Short
    }
}
