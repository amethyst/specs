//! Specs error module.
//!
//! There are specific types for errors (e.g. `WrongGeneration`)
//! and additionally one `Error` type that can represent them all.
//! Each error in this module has an `Into<Error>` implementation.

use std::error::Error as StdError;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};

use world::{Entity, Generation};

/// A boxed error implementing `Debug`, `Display` and `Error`.
pub struct BoxedErr(pub Box<StdError + Send + Sync + 'static>);

impl BoxedErr {
    /// Creates a new boxed error.
    pub fn new<T>(err: T) -> Self
    where
        T: StdError + Send + Sync + 'static,
    {
        BoxedErr(Box::new(err))
    }
}

impl AsRef<StdError> for BoxedErr {
    fn as_ref(&self) -> &(StdError + 'static) {
        self.0.as_ref()
    }
}

impl Debug for BoxedErr {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{:}", self.0)
    }
}

impl Display for BoxedErr {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f, "{}", self.as_ref())
    }
}

impl StdError for BoxedErr {
    fn description(&self) -> &str {
        self.as_ref().description()
    }
}

/// The Specs error type.
/// This is an enum which is able to represent
/// all error types of this library.
///
/// Please note that you should not use `__NonExhaustive`,
/// which is a variant specifically added for extensibility
/// without breakage.
#[derive(Debug)]
pub enum Error {
    /// A custom, boxed error.
    Custom(BoxedErr),
    /// Wrong generation error.
    WrongGeneration(WrongGeneration),

    #[doc(hidden)]
    __NonExhaustive,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match *self {
            Error::Custom(ref e) => write!(f, "Custom: {}", e),
            Error::WrongGeneration(ref e) => write!(f, "Wrong generation: {}", e),

            Error::__NonExhaustive => unimplemented!(),
        }
    }
}

impl From<WrongGeneration> for Error {
    fn from(e: WrongGeneration) -> Self {
        Error::WrongGeneration(e)
    }
}

impl StdError for Error {
    fn description(&self) -> &str {
        "A Specs error"
    }

    fn cause(&self) -> Option<&StdError> {
        let e = match *self {
            Error::Custom(ref e) => e.as_ref(),
            Error::WrongGeneration(ref e) => e,

            Error::__NonExhaustive => unimplemented!(),
        };

        Some(e)
    }
}

/// Wrong generation error.
#[derive(Debug)]
pub struct WrongGeneration {
    /// The action that failed because of the wrong generation.
    pub action: &'static str,
    /// The actual generation of this id.
    pub actual_gen: Generation,
    /// The entity that has been passed, containing
    /// the id and the invalid generation.
    pub entity: Entity,
}

impl Display for WrongGeneration {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(
            f,
            "Tried to {} entity {:?}, but the generation is wrong; it should be {:?}",
            self.action,
            self.entity,
            self.actual_gen
        )
    }
}

impl StdError for WrongGeneration {
    fn description(&self) -> &str {
        "Used an entity with a generation that is not valid anymore \
         (e.g. because the entity has been deleted)"
    }
}
