//! Tools for communicating with Luminator signs over serial.
//!
//! For the basic task of sign communication, you likely want to use the high-level API
//! in the [`flipdot`] crate instead.
//!
//! However, you can use the [`configure_port`] function to configure serial port appropriately
//! if you're doing custom lower-level communication.
//!
//! Intended only for hobbyist and educational purposes. Not affiliated with Luminator in any way.
//!
//! # Examples
//!
//! ```no_run
//! use std::time::Duration;
//!
//! # fn main() -> Result<(), failure::Error> {
//! #
//! let mut port = serial::open("COM3")?;
//! flipdot_serial::configure_port(&mut port, Duration::from_secs(5))?;
//! // Now ready for communication with a sign (8N1 19200 baud).
//! #
//! # Ok(()) }
//! ```
//!
//! [`flipdot`]: https://docs.rs/flipdot
//! [`configure_port`]: fn.configure_port.html
#![doc(html_root_url = "https://docs.rs/flipdot-serial/0.5.0")]
#![deny(
    missing_copy_implementations,
    missing_debug_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code
)]
#![warn(
    missing_docs,
    unused_extern_crates,
    unused_import_braces,
    unused_qualifications,
    unused_results
)]

mod errors;
mod serial_port;
mod serial_sign_bus;

pub use self::errors::{Error, ErrorKind};
pub use self::serial_port::configure_port;
pub use self::serial_sign_bus::SerialSignBus;
