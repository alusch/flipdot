use std::time::Duration;

use failure::ResultExt;
use serial_core as serial;
use serial_core::prelude::*;

use errors::{Error, ErrorKind};

/// Configures the given serial port appropriately for use with Luminator signs.
///
/// Specifically, the signs require 8N1 format at 19200 baud. Also sets the provided timeout value.
///
/// # Errors
///
/// Returns an error of kind [`ErrorKind::Configuration`] if the underlying serial port
/// reports an error.
///
/// # Examples
///
/// ```no_run
/// # extern crate serial;
/// # extern crate flipdot_serial;
/// # extern crate failure;
/// # use failure::Error;
/// use std::time::Duration;
///
/// # fn try_main() -> Result<(), Error> {
/// #
/// let mut port = serial::open("COM3")?;
/// flipdot_serial::configure_port(&mut port, Duration::from_secs(5))?;
/// // Now ready for communication with a sign (8N1 19200 baud).
/// #
/// # Ok(()) }
/// # fn main() { try_main().unwrap(); }
/// ```
///
/// [`ErrorKind::Configuration`]: enum.ErrorKind.html#variant.Configuration
pub fn configure_port<P: SerialPort>(port: &mut P, timeout: Duration) -> Result<(), Error> {
    port.reconfigure(&|settings| {
        settings.set_baud_rate(serial::Baud19200)?;
        settings.set_char_size(serial::Bits8);
        settings.set_parity(serial::ParityNone);
        settings.set_stop_bits(serial::Stop1);
        settings.set_flow_control(serial::FlowNone);
        Ok(())
    }).context(ErrorKind::Configuration)?;
    port.set_timeout(timeout).context(ErrorKind::Configuration)?;
    Ok(())
}
