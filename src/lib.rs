//! A library for interacting with Luminator flip-dot and LED signs over RS-485.
//!
//! Provides a way to connect to a sign, define messages spanning one or more pages, send those pages to the sign,
//! then switch between them. No special graphics or text functionality is provided; you are responsible for setting
//! the pixels on the pages yourself.
//!
//! Tested with a MAX3000 90 × 7 side sign. Should work with any flip-dot or LED sign that uses the 7-pin circular
//! connector, but no guarantees.
//!
//! Intended only for hobbyist and educational purposes. Not affiliated with Luminator in any way.
//!
//! # Examples
//!
//! ```no_run
//! use std::cell::RefCell;
//! use std::rc::Rc;
//! use flipdot::{Address, PageFlipStyle, PageId, Sign, SignType, SerialSignBus};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! #
//! // Set up bus. Because the bus can be shared among
//! // multiple signs, it must be wrapped in an Rc<RefCell>.
//! let port = serial::open("/dev/ttyUSB0")?;
//! let bus = SerialSignBus::try_new(port)?;
//! let bus = Rc::new(RefCell::new(bus));
//!
//! // Create a sign with the appropriate address and type.
//! let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
//!
//! // First, the configuration data must be sent to the sign.
//! sign.configure()?;
//!
//! // Next, we can create some pages, turn on pixels, and send them to the sign.
//! let mut page1 = sign.create_page(PageId(0));
//! page1.set_pixel(0, 0, true);
//! let mut page2 = sign.create_page(PageId(1));
//! page2.set_pixel(1, 1, true);
//!
//! // We only need to explicitly flip pages if PageFlipStyle::Manual is returned,
//! // otherwise the sign will automatically show and flip pages.
//! if sign.send_pages(&[page1, page2])? == PageFlipStyle::Manual {
//!     // The first page is now loaded in the sign's memory and can be shown.
//!     sign.show_loaded_page()?;
//!
//!     // Load the second page into memory, then show it.
//!     sign.load_next_page()?;
//!     sign.show_loaded_page()?;
//! }
//! #
//! # Ok(()) }
//! ```
//!
//! # Sub-crates
//!
//! In addition to the high-level API of [`Sign`], several lower-level components are provided
//! that can be combined for more specialized use-cases.
//!
//! - [`flipdot-core`] \(re-exported as [`core`]\) contains the basic types describing the protocol, and is useful
//!   if you want to implement a custom [`SignBus`] or otherwise operate at the level of the raw protocol.
//! - [`flipdot-serial`] \(re-exported as [`serial`]\) contains functions for configuring the serial port,
//!   as well as the implementation of [`SerialSignBus`].
//! - [`flipdot-testing`] contains tools not directly related to communicating with signs,
//!   but useful for testing and debugging.
//!
//! [`flipdot-core`]: https://docs.rs/flipdot-core
//! [`flipdot-serial`]: https://docs.rs/flipdot-serial
//! [`flipdot-testing`]: https://docs.rs/flipdot-testing
#![doc(html_root_url = "https://docs.rs/flipdot/0.7.1")]
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

pub use flipdot_core as core;
pub use flipdot_serial as serial;

mod sign;

pub use self::sign::{Sign, SignError};

pub use crate::core::{Address, Page, PageFlipStyle, PageId, SignBus, SignType};
pub use crate::serial::SerialSignBus;
