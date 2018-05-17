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
    /// [`Data`] length exceeded the maximum of 255 bytes.
    ///
    /// [`Data`]: struct.Data.html
    #[fail(display = "Data exceeded the maximum of 255 bytes")]
    DataTooLong,

    /// Failed reading/writing a [`Frame`] of data.
    ///
    /// [`Frame`]: struct.Frame.html
    #[fail(display = "Failed reading/writing a frame of data")]
    Io,

    /// Failed to parse data into a [`Frame`].
    ///
    /// [`Frame`]: struct.Frame.html
    #[fail(display = "Failed to parse data into a Frame")]
    InvalidFrame,

    /// [`Frame`] data didn't match declared length.
    ///
    /// [`Frame`]: struct.Frame.html
    #[fail(display = "Frame data didn't match declared length")]
    FrameDataMismatch,

    /// [`Frame`] checksum didn't match declared checksum.
    ///
    /// [`Frame`]: struct.Frame.html
    #[fail(display = "Frame checksum didn't match declared checksum")]
    BadChecksum,

    /// [`SignType`] configuration data was not 16 bytes long.
    ///
    /// [`SignType`]: enum.SignType.html
    #[fail(display = "Sign configuration data was not 16 bytes long")]
    WrongConfigLength,

    /// Configuration data didn't match any known [`SignType`].
    ///
    /// [`SignType`]: enum.SignType.html
    #[fail(display = "Configuration data didn't match any known sign")]
    UnknownConfig,

    /// Data length didn't match the width/height of the [`Page`].
    ///
    /// [`Page`]: struct.Page.html
    #[fail(display = "Data length didn't match the width/height of the page")]
    WrongPageLength,

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
    fn cause(&self) -> Option<&Fail> {
        self.inner.cause()
    }

    fn backtrace(&self) -> Option<&Backtrace> {
        self.inner.backtrace()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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

/// Root-cause error indicating that a value was too large.
///
/// This exists primarily to provide better error messages in the `failure` cause chain,
/// but you can also downcast if you need to interact with it programmatically.
///
/// This type will be the `cause` of [`Error`]s of kind [`ErrorKind::DataTooLong`].
///
/// # Examples
///
/// ```
/// # extern crate failure;
/// # extern crate flipdot_core;
/// use failure::Fail;
/// use flipdot_core::{Data, ErrorKind, MaxExceededError};
///
/// # fn main() {
/// let result = Data::new(vec![0; 256]);
/// match result {
///     Err(ref e) if e.kind() == ErrorKind::DataTooLong => {
///         if let Some(cause) = e.cause().and_then(|c| c.downcast_ref::<MaxExceededError>()) {
///             println!("Data length exceeded max: {} > {}", cause.actual, cause.max);
///         }
///     }
///     _ => {}
/// }
/// # }
/// ```
///
/// [`Error`]: struct.Error.html
/// [`ErrorKind::DataTooLong`]: enum.ErrorKind.html#variant.DataTooLong
#[derive(Clone, Eq, PartialEq, Debug, Fail)]
#[fail(display = "{} - Expected maximum of {}, got {}", message, max, actual)]
pub struct MaxExceededError {
    /// The maximum that was exceeded.
    pub max: usize,

    /// The actual value that was provided.
    pub actual: usize,

    /// More details about which value was invalid.
    pub message: String,
}

impl MaxExceededError {
    /// Creates a new `MaxExceededError` for a given failure.
    pub fn new<T: Into<String>>(max: usize, actual: usize, message: T) -> Self {
        MaxExceededError {
            max,
            actual,
            message: message.into(),
        }
    }
}

/// Root-cause error indicating that a value did not match what was expected.
///
/// This exists primarily to provide better error messages in the `failure` cause chain,
/// but you can also downcast if you need to interact with it programmatically.
///
/// This type will be the `cause` of [`Error`]s of the following kinds:
/// * [`ErrorKind::FrameDataMismatch`]
/// * [`ErrorKind::BadChecksum`]
/// * [`ErrorKind::WrongConfigLength`]
/// * [`ErrorKind::WrongPageLength`]
///
/// # Examples
///
/// ```
/// # extern crate failure;
/// # extern crate flipdot_core;
/// use failure::Fail;
/// use flipdot_core::{Frame, ErrorKind, WrongValueError};
///
/// # fn main() {
/// let result = Frame::from_bytes(b":01007F02FF7E");
/// match result {
///     Err(ref e) if e.kind() == ErrorKind::BadChecksum => {
///         if let Some(cause) = e.cause().and_then(|c| c.downcast_ref::<WrongValueError>()) {
///             println!("Bad checkum: got {} instead of {}", cause.actual, cause.expected);
///         }
///     }
///     _ => {}
/// }
/// # }
/// ```
///
/// [`Error`]: struct.Error.html
/// [`ErrorKind::FrameDataMismatch`]: enum.ErrorKind.html#variant.FrameDataMismatch
/// [`ErrorKind::BadChecksum`]: enum.ErrorKind.html#variant.BadChecksum
/// [`ErrorKind::WrongConfigLength`]: enum.ErrorKind.html#variant.WrongConfigLength
/// [`ErrorKind::WrongPageLength`]: enum.ErrorKind.html#variant.WrongPageLength
#[derive(Clone, Eq, PartialEq, Debug, Fail)]
#[fail(display = "{} - Expected {}, got {}", message, expected, actual)]
pub struct WrongValueError {
    /// The expected value.
    pub expected: usize,

    /// The actual value that was provided.
    pub actual: usize,

    /// More details about which value was invalid.
    pub message: String,
}

impl WrongValueError {
    /// Creates a new `WrongValueError` for a given failure.
    pub fn new<T: Into<String>>(expected: usize, actual: usize, message: T) -> Self {
        WrongValueError {
            expected,
            actual,
            message: message.into(),
        }
    }
}
