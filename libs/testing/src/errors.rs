use std::fmt;

use failure::{Backtrace, Context, Fail};

/// The error type.
#[derive(Debug)]
pub struct Error {
    inner: Context<ErrorKind>,
}

/// The specific kind of error that occurred.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Fail)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Failed to configure the serial port.
    #[fail(display = "Failed to configure the serial port")]
    Configuration,

    /// The sign bus failed to process a message.
    #[fail(display = "Sign bus failed to process message")]
    Bus,

    /// Failure reading/writing data.
    #[fail(display = "Failure reading/writing data")]
    Communication,
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
