//! Core types for describing communication with Luminator flip-dot and LED signs.
//!
//! For the basic task of sign communication, you likely want to use the high-level API
//! in the [`flipdot`] crate instead.
//!
//! However, `flipdot_core` is useful for crates that want to interact with the sign protocol
//! at a lower level than the `flipdot` crate, or who want to provide their own [`SignBus`]
//! implementations for use by `flipdot`.
//!
//! Tested with a MAX3000 90 × 7 side sign. Should work with any flip-dot or LED sign that uses the 7-pin circular
//! connector, but no guarantees.
//!
//! Intended only for hobbyist and educational purposes. Not affiliated with Luminator in any way.
//!
//! # Examples
//!
//! ```no_run
//! extern crate flipdot_core;
//! # extern crate flipdot_testing;
//! # extern crate failure;
//! # use failure::Error;
//! use flipdot_core::{Address, Message, Operation, SignBus, SignType, State};
//! # use flipdot_testing::{VirtualSign, VirtualSignBus};
//!
//! # fn get_bus() -> Box<SignBus> { Box::new(VirtualSignBus::new(vec![VirtualSign::new(Address(3))])) }
//! # fn try_main() -> Result<(), Error> {
//! #
//! // Assume we have a helper function to obtain a SignBus.
//! let mut bus: Box<SignBus> = get_bus();
//!
//! // Discover the sign and verify that is has not yet been configured.
//! let message = Message::Hello(Address(3));
//! let response = bus.process_message(message)?;
//! assert_eq!(Some(Message::ReportState(Address(3), State::Unconfigured)), response);
//!
//! // Request that the sign receive the configuration data and verify that it acknowledges.
//! let message = Message::RequestOperation(Address(3), Operation::ReceiveConfig);
//! let response = bus.process_message(message)?;
//! assert_eq!(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig)), response);
//!
//! #
//! # Ok(()) }
//! # fn main() { try_main().unwrap(); }
//! ```
//!
//! [`flipdot`]: https://docs.rs/flipdot
//! [`SignBus`]: trait.SignBus.html
#![doc(html_root_url = "https://docs.rs/flipdot-core/0.3.0")]
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

#[macro_use]
extern crate failure;
#[macro_use]
extern crate failure_derive;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate macro_attr;
#[macro_use]
extern crate newtype_derive;
extern crate num_traits;
extern crate regex;

mod errors;
mod frame;
mod message;
mod page;
mod sign_bus;
mod sign_type;

pub use self::errors::{Error, ErrorKind, MaxExceededError, WrongValueError};
pub use self::frame::{Address, Data, Frame, MsgType};
pub use self::message::{ChunkCount, Message, Offset, Operation, State};
pub use self::page::{Page, PageId};
pub use self::sign_bus::SignBus;
pub use self::sign_type::SignType;
