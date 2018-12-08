use std::cell::RefCell;
use std::rc::Rc;

use flipdot::core::{Frame, Message, Operation, State};
use flipdot::{Address, PageId, SerialSignBus, Sign, SignType};
use serial_core::{PortSettings, SerialDevice};

mod mock_serial_port;
use crate::mock_serial_port::{MockSerialPort, SerialFailure};

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

    let port = MockSerialPort::new(buf, SerialFailure::None);
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
