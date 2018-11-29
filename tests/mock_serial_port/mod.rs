use serial_core;

use std::io::{self, Cursor, Read, Write};
use std::time::Duration;

use serial_core::{PortSettings, SerialDevice};

#[allow(dead_code)] // Tests use different subsets of these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SerialFailure {
    None,
    WriteSettings,
    Read,
}

/// Mock serial port implementation that reads data from a vector
/// and discards writes. Used to verify `SerialSignBus`.
#[derive(Debug, Clone)]
pub struct MockSerialPort {
    failure: SerialFailure,
    data: Cursor<Vec<u8>>,
    settings: PortSettings,
}

impl MockSerialPort {
    pub fn new(data: Vec<u8>, failure: SerialFailure) -> Self {
        MockSerialPort {
            failure,
            data: Cursor::new(data),
            // Initialize settings to some weird defaults to verify we set them correctly later.
            settings: PortSettings {
                baud_rate: serial_core::BaudRate::Baud110,
                char_size: serial_core::CharSize::Bits7,
                parity: serial_core::Parity::ParityEven,
                stop_bits: serial_core::StopBits::Stop2,
                flow_control: serial_core::FlowControl::FlowSoftware,
            },
        }
    }

    #[allow(dead_code)] // Not used by all tests.
    pub fn done(&self) {
        assert_eq!(self.data.position(), self.data.get_ref().len() as u64);
    }
}

impl Read for MockSerialPort {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self.failure {
            SerialFailure::Read => Err(io::Error::new(io::ErrorKind::Other, "Dummy I/O error")),
            _ => self.data.read(buf),
        }
    }
}

impl Write for MockSerialPort {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl SerialDevice for MockSerialPort {
    type Settings = PortSettings;

    fn read_settings(&self) -> serial_core::Result<Self::Settings> {
        Ok(self.settings)
    }

    fn write_settings(&mut self, settings: &Self::Settings) -> serial_core::Result<()> {
        match self.failure {
            SerialFailure::WriteSettings => Err(serial_core::Error::new(
                serial_core::ErrorKind::NoDevice,
                "Dummy serial error",
            )),
            _ => {
                self.settings = *settings;
                Ok(())
            }
        }
    }

    fn timeout(&self) -> Duration {
        unimplemented!();
    }

    fn set_timeout(&mut self, _: Duration) -> serial_core::Result<()> {
        Ok(())
    }

    fn set_rts(&mut self, _: bool) -> serial_core::Result<()> {
        unimplemented!();
    }

    fn set_dtr(&mut self, _: bool) -> serial_core::Result<()> {
        unimplemented!();
    }

    fn read_cts(&mut self) -> serial_core::Result<bool> {
        unimplemented!();
    }

    fn read_dsr(&mut self) -> serial_core::Result<bool> {
        unimplemented!();
    }

    fn read_ri(&mut self) -> serial_core::Result<bool> {
        unimplemented!();
    }

    fn read_cd(&mut self) -> serial_core::Result<bool> {
        unimplemented!();
    }
}
