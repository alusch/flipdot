use std::time::Duration;

use serial_core::prelude::*;

use flipdot_core::{Frame, Message, SignBus};
use flipdot_serial;

use errors::{self, ErrorKind};

/// Connects to a real ODK over the specified serial port and uses it to drive a `SignBus`.
///
/// Typically this will be used to drive a [`VirtualSignBus`] in order to study the bus traffic
/// or inspect the pages of pixel data sent by the ODK.
///
/// # Examples
///
/// ```no_run
/// extern crate serial;
/// extern crate flipdot_serial;
/// extern crate flipdot_testing;
/// use flipdot_serial::SerialSignBus;
/// use flipdot_testing::{Address, Odk, VirtualSign, VirtualSignBus};
///
/// # use std::error::Error;
/// # fn try_main() -> Result<(), Box<Error>> {
/// #
/// // Populate bus with signs from addresses 2 to 126
/// // (which seems to be the possible range for actual signs).
/// let signs = (2..127).map(Address).map(VirtualSign::new);
/// let bus = VirtualSignBus::new(signs);
///
/// // Hook up ODK to virtual bus.
/// let port = serial::open("/dev/ttyUSB0")?;
/// let mut odk = Odk::new(port, bus)?;
/// loop {
///     // ODK communications are forwarded to/from the virtual bus.
///     odk.process_message()?;
/// }
/// #
/// # Ok(()) }
/// # fn main() { try_main().unwrap(); }
/// ```
///
/// [`VirtualSignBus`]: struct.VirtualSignBus.html
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Odk<P: SerialPort, B: SignBus> {
    port: P,
    bus: B,
}

impl<P: SerialPort, B: SignBus> Odk<P, B> {
    /// Create a new `Odk` that connects the specified serial port and bus.
    ///
    /// # Errors
    /// Returns [`Error`]`(`[`ErrorKind::Serial`]`, _)` if the serial port
    /// cannot be configured.
    ///
    /// [`Error`]: struct.Error.html
    /// [`ErrorKind::Serial`]: enum.ErrorKind.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # extern crate serial;
    /// # extern crate flipdot_serial;
    /// # extern crate flipdot_testing;
    /// # use flipdot_serial::SerialSignBus;
    /// # use flipdot_testing::{Address, Odk, VirtualSign, VirtualSignBus};
    /// # use std::error::Error;
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// let bus = VirtualSignBus::new(vec![VirtualSign::new(Address(3))]);
    /// let port = serial::open("COM3")?;
    /// let odk = Odk::new(port, bus)?;
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    ///
    /// Note: You would typically use the `env_logger` crate and run with
    /// `RUST_LOG=debug` to watch the bus messages go by.
    pub fn new(mut port: P, bus: B) -> errors::Result<Self> {
        flipdot_serial::configure_port(&mut port, Duration::from_secs(10))?;
        Ok(Odk { port, bus })
    }

    /// Reads the next frame from the ODK over the serial port, forwards it
    /// to the attached bus, and sends the response, if any, back to the ODK.
    ///
    /// # Errors
    ///
    /// Returns [`Error`]`(`[`ErrorKind::Core`]`, _)` if there was an error
    /// reading or writing the data.
    ///
    /// Returns [`Error`]`(`[`ErrorKind::Bus`]`, _)` if the bus failed to
    /// process the message.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # extern crate serial;
    /// # extern crate flipdot_serial;
    /// # extern crate flipdot_testing;
    /// # use flipdot_serial::SerialSignBus;
    /// # use flipdot_testing::{Address, Odk, VirtualSign, VirtualSignBus};
    /// # use std::error::Error;
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// let bus = VirtualSignBus::new(vec![VirtualSign::new(Address(3))]);
    /// let port = serial::open("/dev/ttyUSB0")?;
    /// let mut odk = Odk::new(port, bus)?;
    /// loop {
    ///     odk.process_message()?;
    /// }
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    ///
    /// [`Error`]: struct.Error.html
    /// [`ErrorKind::Core`]: enum.ErrorKind.html
    /// [`ErrorKind::Bus`]: enum.ErrorKind.html
    pub fn process_message(&mut self) -> errors::Result<()> {
        let response = {
            let frame = Frame::read(&mut self.port)?;
            let message = Message::from(frame);
            self.bus
                .process_message(message)
                .map_err(|e| errors::Error::with_boxed_chain(e, ErrorKind::Bus))?
        };

        if let Some(message) = response {
            let frame = Frame::from(message);
            frame.write(&mut self.port)?;
        }

        Ok(())
    }
}
