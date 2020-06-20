use std::time::Duration;

use serial_core::prelude::*;
use thiserror::Error;

use flipdot_core::{Frame, Message, SignBus};

/// Errors related to [`Odk`]s.
///
/// [`Odk`]: struct.Odk.html
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum OdkError {
    /// The sign bus failed to process a message.
    #[error("Sign bus failed to process message")]
    Bus {
        /// The underlying bus error.
        #[from]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Failure reading/writing data.
    #[error("ODK communication error")]
    Communication {
        /// The underlying communication error.
        #[from]
        source: flipdot_core::FrameError,
    },
}

/// Connects to a real ODK over the specified serial port and uses it to drive a `SignBus`.
///
/// Typically this will be used to drive a [`VirtualSignBus`] in order to study the bus traffic
/// or inspect the pages of pixel data sent by the ODK.
///
/// # Examples
///
/// ```no_run
/// use flipdot_serial::SerialSignBus;
/// use flipdot_testing::{Address, Odk, VirtualSign, VirtualSignBus};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #
/// // Populate bus with signs from addresses 2 to 126
/// // (which seems to be the possible range for actual signs).
/// let signs = (2..127).map(Address).map(VirtualSign::new);
/// let bus = VirtualSignBus::new(signs);
///
/// // Hook up ODK to virtual bus.
/// let port = serial::open("/dev/ttyUSB0")?;
/// let mut odk = Odk::try_new(port, bus)?;
/// loop {
///     // ODK communications are forwarded to/from the virtual bus.
///     odk.process_message()?;
/// }
/// #
/// # Ok(()) }
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
    ///
    /// Returns the underlying `serial_core::Error` if the serial port cannot be configured.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use flipdot_serial::SerialSignBus;
    /// # use flipdot_testing::{Address, Odk, VirtualSign, VirtualSignBus};
    /// #
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #
    /// let bus = VirtualSignBus::new(vec![VirtualSign::new(Address(3))]);
    /// let port = serial::open("COM3")?;
    /// let odk = Odk::try_new(port, bus)?;
    /// #
    /// # Ok(()) }
    /// ```
    ///
    /// Note: You would typically use the `env_logger` crate and run with
    /// `RUST_LOG=debug` to watch the bus messages go by.
    pub fn try_new(mut port: P, bus: B) -> Result<Self, serial_core::Error> {
        flipdot_serial::configure_port(&mut port, Duration::from_secs(10))?;
        Ok(Odk { port, bus })
    }

    /// Reads the next frame from the ODK over the serial port, forwards it
    /// to the attached bus, and sends the response, if any, back to the ODK.
    ///
    /// # Errors
    ///
    /// Returns:
    /// * [`OdkError::Communication`] if there was an error reading or writing the data.
    /// * [`OdkError::Bus`] if the bus failed to process the message.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use flipdot_serial::SerialSignBus;
    /// # use flipdot_testing::{Address, Odk, VirtualSign, VirtualSignBus};
    /// #
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #
    /// let bus = VirtualSignBus::new(vec![VirtualSign::new(Address(3))]);
    /// let port = serial::open("/dev/ttyUSB0")?;
    /// let mut odk = Odk::try_new(port, bus)?;
    /// loop {
    ///     odk.process_message()?;
    /// }
    /// #
    /// # Ok(()) }
    /// ```
    ///
    /// [`OdkError::Communication`]: enum.OdkError.html#variant.Communication
    /// [`OdkError::Bus`]: enum.OdkError.html#variant.Bus
    pub fn process_message(&mut self) -> Result<(), OdkError> {
        let response = {
            let frame = Frame::read(&mut self.port)?;
            let message = Message::from(frame);
            self.bus.process_message(message)?
        };

        if let Some(message) = response {
            let frame = Frame::from(message);
            frame.write(&mut self.port)?;
        }

        Ok(())
    }
}
