use std::time::Duration;

use serial_core as serial;
use serial_core::prelude::*;

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
/// use std::time::Duration;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #
/// let mut port = serial::open("COM3")?;
/// flipdot_serial::configure_port(&mut port, Duration::from_secs(5))?;
/// // Now ready for communication with a sign (8N1 19200 baud).
/// #
/// # Ok(()) }
/// ```
///
/// [`ErrorKind::Configuration`]: enum.ErrorKind.html#variant.Configuration
pub fn configure_port<P: SerialPort>(port: &mut P, timeout: Duration) -> Result<(), serial_core::Error> {
    port.reconfigure(&|settings| {
        settings.set_baud_rate(serial::Baud19200)?;
        settings.set_char_size(serial::Bits8);
        settings.set_parity(serial::ParityNone);
        settings.set_stop_bits(serial::Stop1);
        settings.set_flow_control(serial::FlowNone);
        Ok(())
    })?;

    port.set_timeout(timeout)?;

    Ok(())
}
