use std::error::Error;
use std::thread;
use std::time::Duration;

use log::debug;
use serial_core::prelude::*;

use flipdot_core::{Frame, Message, SignBus, State};

use crate::serial_port;

/// An implementation of [`SignBus`] that communicates with one or more signs over serial.
///
/// Messages and responses are logged using the [`log`] crate for debugging purposes. Consuming binaries
/// typically use the [`env_logger`] crate and can be run with the `RUST_LOG=debug` environment variable
/// to watch the bus messages go by.
///
/// # Examples
///
/// ```no_run
/// use flipdot_serial::SerialSignBus;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #
/// let port = serial::open("/dev/ttyUSB0")?;
/// let bus = SerialSignBus::try_new(port)?;
/// // Can now connect a Sign to the bus.
/// #
/// # Ok(()) }
/// ```
///
/// [`log`]: https://crates.io/crates/log
/// [`env_logger`]: https://crates.io/crates/env_logger
#[derive(Debug, Eq, PartialEq, Hash)]
pub struct SerialSignBus<P: SerialPort> {
    port: P,
}

impl<P: SerialPort> SerialSignBus<P> {
    /// Creates a new `SerialSignBus` that communicates over the specified serial port.
    ///
    /// # Errors
    ///
    /// Returns the underlying [`serial_core::Error`] if the serial port cannot be configured.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use flipdot_serial::SerialSignBus;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #
    /// let port = serial::open("COM3")?;
    /// let bus = SerialSignBus::try_new(port)?;
    /// #
    /// # Ok(()) }
    /// ```
    pub fn try_new(mut port: P) -> Result<Self, serial_core::Error> {
        serial_port::configure_port(&mut port, Duration::from_secs(5))?;
        Ok(SerialSignBus { port })
    }

    /// Returns a reference to the underlying serial port.
    pub fn port(&self) -> &P {
        &self.port
    }
}

impl<P: SerialPort> SignBus for SerialSignBus<P> {
    /// Handles a bus message by sending it to the serial port and reading a response if necessary.
    fn process_message<'a>(&mut self, message: Message<'_>) -> Result<Option<Message<'a>>, Box<dyn Error + Send + Sync>> {
        debug!("Bus message: {}", message);

        let response_expected = response_expected(&message);
        let delay = delay_after_send(&message);

        let frame = Frame::from(message);
        frame.write(&mut self.port)?;

        if let Some(duration) = delay {
            thread::sleep(duration);
        }

        if response_expected {
            let frame = Frame::read(&mut self.port)?;
            let message = Message::from(frame);
            debug!(" Sign reply: {}", message);

            if let Some(duration) = delay_after_receive(&message) {
                thread::sleep(duration);
            }

            Ok(Some(message))
        } else {
            Ok(None)
        }
    }
}

/// Determines whether we need to listen for a response to the given message.
fn response_expected(message: &Message<'_>) -> bool {
    // A sign is only expected to reply to messages that query its state or request
    // that it perform an operation.
    matches!(
        *message,
        Message::Hello(_) | Message::QueryState(_) | Message::RequestOperation(_, _)
    )
}

/// Returns the length of time to delay after sending a message.
fn delay_after_send(message: &Message<'_>) -> Option<Duration> {
    match *message {
        // When sending data, this delay is necessary to avoid overloading the receiving sign.
        Message::SendData(_, _) => Some(Duration::from_millis(30)),
        _ => None,
    }
}

/// Returns the length of time to delay after receiving a response.
fn delay_after_receive(message: &Message<'_>) -> Option<Duration> {
    match *message {
        // When loading or showing a page, we wait for the sign to finish the operation, which can take
        // a second or more depending on how many dots need to flip. This delay prevents us from spamming
        // the sign with status requests.
        Message::ReportState(_, State::PageLoadInProgress) | Message::ReportState(_, State::PageShowInProgress) => {
            Some(Duration::from_millis(100))
        }
        _ => None,
    }
}
