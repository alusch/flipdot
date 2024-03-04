//! Tools for testing and debugging Luminator sign communications.
//!
//! For the basic task of sign communication, you likely want to use the high-level API
//! in the [`flipdot`] crate instead.
//!
//! This crate isn't directly related to controlling a real sign, but provides some helpful diagnostic tools.
//! [`VirtualSignBus`] is a general-purpose mock implementation of one or more signs attached to the bus,
//! and [`Odk`] allows connecting a real ODK over serial to a [`SignBus`](flipdot_core::SignBus).
//!
//! Intended only for hobbyist and educational purposes. Not affiliated with Luminator in any way.
//!
//! # Examples
//!
//! ```no_run
//! use flipdot_core::PageFlipStyle;
//! use flipdot_serial::SerialSignBus;
//! use flipdot_testing::{Address, Odk, VirtualSign, VirtualSignBus};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! #
//! // Populate bus with signs from addresses 2 to 126
//! // (which seems to be the possible range for actual signs).
//! let signs = (2..127).map(Address).map(|addr| VirtualSign::new(addr, PageFlipStyle::Manual));
//! let bus = VirtualSignBus::new(signs);
//!
//! // Hook up ODK to virtual bus.
//! let port = serial::open("COM3")?;
//! let mut odk = Odk::try_new(port, bus)?;
//! loop {
//!     // ODK communications are forwarded to/from the virtual bus.
//!     odk.process_message()?;
//! }
//! #
//! # Ok(()) }
//! ```
//!
//! [`flipdot`]: https://docs.rs/flipdot
#![doc(html_root_url = "https://docs.rs/flipdot-testing/0.7.1")]
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

mod odk;
mod virtual_sign_bus;

pub use self::odk::{Odk, OdkError};
pub use self::virtual_sign_bus::{VirtualSign, VirtualSignBus};

pub use flipdot_core::Address;
