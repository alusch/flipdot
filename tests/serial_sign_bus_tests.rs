extern crate flipdot;
extern crate serial_core;

use std::cell::RefCell;
use std::io::{self, Read, Write};
use std::rc::Rc;
use std::time::Duration;

use flipdot::{Address, PageId, SerialSignBus, Sign, SignType};
use flipdot::core::{Frame, Message, Operation, State};
use serial_core::{PortSettings, SerialDevice};

/// Mock serial port implementation that reads data from a vector
/// and discards writes. Used to verify `SerialSignBus`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct MockSerialPort {
    data: Vec<u8>,
    offset: usize,
    settings: PortSettings,
}

impl MockSerialPort {
    fn new(data: Vec<u8>) -> Self {
        MockSerialPort {
            data: data,
            offset: 0,
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

    fn done(&self) {
        assert_eq!(self.offset, self.data.len());
    }
}

impl Read for MockSerialPort {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // Ensure we haven't walked off the end of our data.
        assert!(self.offset < self.data.len());

        let mut slice = &self.data[self.offset..];
        let bytes_read = slice.read(buf).unwrap();

        // Ensure we were able to fully satisfy the read.
        assert_eq!(buf.len(), bytes_read);
        self.offset += bytes_read;

        Ok(bytes_read)
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
        self.settings = *settings;
        Ok(())
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

#[test]
fn serial_sign_bus_works() {
    let mut buf = Vec::new();
    buf.extend(Frame::from(Message::ReportState(Address(1), State::Unconfigured)).to_bytes_with_newline());
    buf.extend(Frame::from(Message::AckOperation(Address(1), Operation::ReceiveConfig)).to_bytes_with_newline());
    buf.extend(Frame::from(Message::ReportState(Address(1), State::ConfigReceived)).to_bytes_with_newline());
    buf.extend(Frame::from(Message::AckOperation(Address(1), Operation::ReceivePixels)).to_bytes_with_newline());
    buf.extend(Frame::from(Message::ReportState(Address(1), State::PixelsReceived)).to_bytes_with_newline());
    buf.extend(Frame::from(Message::ReportState(Address(1), State::PageLoaded)).to_bytes_with_newline());
    buf.extend(Frame::from(Message::AckOperation(Address(1), Operation::ShowLoadedPage)).to_bytes_with_newline());
    buf.extend(Frame::from(Message::ReportState(Address(1), State::PageShowInProgress)).to_bytes_with_newline());
    buf.extend(Frame::from(Message::ReportState(Address(1), State::PageShown)).to_bytes_with_newline());

    let port = MockSerialPort::new(buf);
    let bus = SerialSignBus::new(port).unwrap();

    // Ensure serial port was configured correctly.
    let expected = PortSettings {
        baud_rate: serial_core::BaudRate::Baud19200,
        char_size: serial_core::CharSize::Bits8,
        parity: serial_core::Parity::ParityNone,
        stop_bits: serial_core::StopBits::Stop1,
        flow_control: serial_core::FlowControl::FlowNone,
    };
    assert_eq!(expected, bus.port().read_settings().unwrap());

    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(1), SignType::HorizonFront160x16);

    // Send sign commands and verify success.
    sign.configure().unwrap();
    let pages = [sign.create_page(PageId(1))];
    sign.send_pages(&pages).unwrap();
    sign.show_loaded_page().unwrap();

    // Ensure all data read.
    bus.borrow().port().done();
}
