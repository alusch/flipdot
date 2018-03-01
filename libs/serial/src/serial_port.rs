use std::time::Duration;

use serial_core as serial;
use serial_core::prelude::*;

use errors::{self, ErrorKind, ResultExt};

/// Configures the given serial port appropriately for use with Luminator signs.
///
/// Specifically, the signs require 8N1 format at 19200 baud. Also sets the provided timeout value.
///
/// # Errors
///
/// Returns [`Error`]`(`[`ErrorKind::Serial`]`, _)` if the underlying serial port
/// reports an error.
///
/// # Examples
///
/// ```no_run
/// # extern crate serial;
/// # extern crate flipdot_serial;
/// # use std::error::Error;
/// use std::time::Duration;
///
/// # fn try_main() -> Result<(), Box<Error>> {
/// #
/// let mut port = serial::open("COM3")?;
/// flipdot_serial::configure_port(&mut port, Duration::from_secs(5))?;
/// // Now ready for communication with a sign (8N1 19200 baud).
/// #
/// # Ok(()) }
/// # fn main() { try_main().unwrap(); }
/// ```
///
/// [`Error`]: struct.Error.html
/// [`ErrorKind::Serial`]: enum.ErrorKind.html
pub fn configure_port<P: SerialPort>(port: &mut P, timeout: Duration) -> errors::Result<()> {
    port.reconfigure(&|settings| {
        settings.set_baud_rate(serial::Baud19200)?;
        settings.set_char_size(serial::Bits8);
        settings.set_parity(serial::ParityNone);
        settings.set_stop_bits(serial::Stop1);
        settings.set_flow_control(serial::FlowNone);
        Ok(())
    }).chain_err(|| ErrorKind::Serial("Couldn't configure serial port".to_owned()))?;
    port.set_timeout(timeout)
        .chain_err(|| ErrorKind::Serial("Couldn't set serial timeout".to_owned()))?;
    Ok(())
}
