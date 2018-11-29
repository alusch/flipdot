use std::fmt;

use failure::{Backtrace, Context, Fail};

/// The error type.
#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

/// The specific kind of error that occurred.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
    /// The sign bus failed to process a message.
    #[fail(display = "Sign bus failed to process message")]
    Bus,

    /// Sign did not respond properly according to the protocol.
    #[fail(display = "Sign did not respond properly according to the protocol")]
    UnexpectedResponse,

    // Don't actually use this; it's just here to prevent exhaustive matching
    // so we can extend this enum in the future without a breaking change.
    #[doc(hidden)]
    #[fail(display = "")]
    __Nonexhaustive,
}

impl Error {
    /// The specific kind of error that occurred.
    pub fn kind(&self) -> ErrorKind {
        *self.inner.get_context()
    }
}

impl Fail for Error {
    fn cause(&self) -> Option<&dyn Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Error {
        Error::from(Context::new(kind))
    }
}

impl From<Context<ErrorKind>> for Error {
    fn from(inner: Context<ErrorKind>) -> Error {
        Error { inner }
    }
}
