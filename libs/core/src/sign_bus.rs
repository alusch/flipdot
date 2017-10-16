use std::error::Error;
use std::fmt::{self, Debug, Formatter};

use Message;

/// Abstraction over a bus containing devices that are able to send and receive [`Message`]s.
///
/// Typically `SerialSignBus` from [`flipdot`] or `VirtualSignBus` from [`flipdot-testing`] are sufficient,
/// and you do not need to implement this yourself.
///
/// # Examples
///
/// Using `SignBus` as a trait object to allow choosing the type of bus at runtime:
///
/// ```
/// # extern crate serial;
/// # extern crate flipdot;
/// # extern crate flipdot_testing;
/// # use std::error::Error;
/// use std::cell::RefCell;
/// use std::rc::Rc;
/// use flipdot::{Address, Sign, SignBus, SignType};
/// use flipdot::serial::SerialSignBus;
/// use flipdot_testing::{VirtualSign, VirtualSignBus};
///
/// # fn use_serial() -> bool { false }
/// # fn try_main() -> Result<(), Box<Error>> {
/// #
/// let bus: Rc<RefCell<SignBus>> = if use_serial() {
///     let port = serial::open("/dev/ttyUSB0")?;
///     Rc::new(RefCell::new(SerialSignBus::new(port)?))
/// } else {
///     Rc::new(RefCell::new(VirtualSignBus::new(vec![VirtualSign::new(Address(3))])))
/// };
///
/// let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
/// sign.configure()?;
/// #
/// # Ok(()) }
/// # fn main() { try_main().unwrap(); }
/// ```
///
/// Implementing a custom bus:
///
/// ```
/// # use std::error::Error;
/// use flipdot_core::{Message, SignBus, State};
///
/// struct ExampleSignBus {}
///
/// impl SignBus for ExampleSignBus {
///     fn process_message<'a>(&mut self, message: Message)
///         -> Result<Option<Message<'a>>, Box<Error + Send>> {
///         match message {
///             Message::Hello(address) |
///             Message::QueryState(address) =>
///                 Ok(Some(Message::ReportState(address, State::Unconfigured))),
///             _ => Ok(None), // Implement rest of protocol here...
///         }
///     }
/// }
/// ```
///
/// [`Message`]: enum.Message.html
/// [`flipdot`]: https://docs.rs/flipdot
/// [`flipdot-testing`]: https://docs.rs/flipdot-testing
pub trait SignBus {
    /// Sends a message to the bus and returns an optional response.
    ///
    /// The caller is the "controller" (e.g. an ODK), and this method conceptually delivers the message
    /// to a certain sign on the bus and returns an optional response from it.
    ///
    /// # Examples
    ///
    /// See the [trait-level documentation].
    ///
    /// [trait-level documentation]: #examples
    fn process_message<'a>(&mut self, message: Message) -> Result<Option<Message<'a>>, Box<Error + Send>>;
}

// Provide a Debug representation so types that contain trait objects can derive Debug.
impl Debug for SignBus {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "<SignBus trait>")
    }
}
