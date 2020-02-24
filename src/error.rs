//! Specs errors
//!
//! There are specific types for errors (e.g. `WrongGeneration`)
//! and additionally one `Error` type that can represent them all.
//! Each error in this module has an `Into<Error>` implementation.

use std::{
    convert::Infallible,
    error::Error as StdError,
    fmt::{Debug, Display, Formatter, Result as FmtResult},
};

use crate::world::{Entity, Generation};

/// A boxed error implementing `Debug`, `Display` and `Error`.
pub struct BoxedErr(pub Box<dyn StdError + Send + Sync + 'static>);

impl BoxedErr {
    /// Creates a new boxed error.
    pub fn new<T>(err: T) -> Self
    where
        T: StdError + Send + Sync + 'static,
    {
        BoxedErr(Box::new(err))
    }
}

impl AsRef<dyn StdError> for BoxedErr {
    fn as_ref(&self) -> &(dyn StdError + 'static) {
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

impl StdError for BoxedErr {}

/// The Specs error type.
/// This is an enum which is able to represent
/// all error types of this library.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A custom, boxed error.
    Custom(BoxedErr),
    /// Wrong generation error.
    WrongGeneration(WrongGeneration),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match *self {
            Error::Custom(ref e) => write!(f, "Custom: {}", e),
            Error::WrongGeneration(ref e) => write!(f, "Wrong generation: {}", e),
        }
    }
}

impl From<Infallible> for Error {
    fn from(e: Infallible) -> Self {
        match e {}
    }
}

impl From<WrongGeneration> for Error {
    fn from(e: WrongGeneration) -> Self {
        Error::WrongGeneration(e)
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        let e = match *self {
            Error::Custom(ref e) => e.as_ref(),
            Error::WrongGeneration(ref e) => e,
        };

        Some(e)
    }
}

/// Wrong generation error.
#[derive(Debug, PartialEq, Eq)]
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
            "Tried to {} entity {:?}, but the generation is no longer valid; it should be {:?}",
            self.action, self.entity, self.actual_gen
        )
    }
}

impl StdError for WrongGeneration {}

/// Reexport of `Infallible` for a smoother transition.
#[deprecated = "Use std::convert::Infallible instead"]
pub type NoError = Infallible;
