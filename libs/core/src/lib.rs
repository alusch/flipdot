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
//! use flipdot_core::{Address, Message, Operation, PageFlipStyle, SignBus, SignType, State};
//! # use flipdot_testing::{VirtualSign, VirtualSignBus};
//!
//! # fn get_bus() -> Box<dyn SignBus> { Box::new(VirtualSignBus::new(vec![VirtualSign::new(Address(3), PageFlipStyle::Manual)])) }
//! # fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//! #
//! // Assume we have a helper function to obtain a SignBus.
//! let mut bus: Box<dyn SignBus> = get_bus();
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
//! ```
//!
//! [`flipdot`]: https://docs.rs/flipdot
#![doc(html_root_url = "https://docs.rs/flipdot-core/0.8.0")]
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

mod frame;
mod message;
mod page;
mod sign_bus;
mod sign_type;

pub use self::frame::{Address, Data, Frame, FrameError, MsgType};
pub use self::message::{ChunkCount, Message, Offset, Operation, State};
pub use self::page::{Page, PageError, PageFlipStyle, PageId};
pub use self::sign_bus::SignBus;
pub use self::sign_type::{SignType, SignTypeError};
