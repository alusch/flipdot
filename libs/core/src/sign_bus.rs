use std::error::Error;
use std::fmt::{self, Debug, Formatter};

use crate::Message;

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
/// use std::cell::RefCell;
/// use std::rc::Rc;
/// use flipdot::{Address, PageFlipStyle, Sign, SignBus, SignType};
/// use flipdot::serial::SerialSignBus;
/// use flipdot_testing::{VirtualSign, VirtualSignBus};
///
/// # fn use_serial() -> bool { false }
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #
/// let bus: Rc<RefCell<dyn SignBus>> = if use_serial() {
///     let port = serial::open("/dev/ttyUSB0")?;
///     Rc::new(RefCell::new(SerialSignBus::try_new(port)?))
/// } else {
///     Rc::new(RefCell::new(VirtualSignBus::new(vec![VirtualSign::new(Address(3), PageFlipStyle::Manual)])))
/// };
///
/// let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
/// sign.configure()?;
/// #
/// # Ok(()) }
/// ```
///
/// Implementing a custom bus:
///
/// ```
/// use flipdot_core::{Message, SignBus, State};
///
/// struct ExampleSignBus {}
///
/// impl SignBus for ExampleSignBus {
///     fn process_message<'a>(&mut self, message: Message)
///         -> Result<Option<Message<'a>>, Box<dyn std::error::Error + Send + Sync>> {
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
/// [`flipdot`]: https://docs.rs/flipdot
/// [`flipdot-testing`]: https://docs.rs/flipdot_testing
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
    fn process_message<'a>(&mut self, message: Message<'_>) -> Result<Option<Message<'a>>, Box<dyn Error + Send + Sync>>;
}

// Provide a Debug representation so types that contain trait objects can derive Debug.
impl Debug for dyn SignBus {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "<SignBus trait>")
    }
}
