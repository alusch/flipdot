use std::cell::RefCell;
use std::error::Error;
use std::fmt::Debug;
use std::io::{self, Read};
use std::iter;
use std::rc::Rc;

use flipdot::core::*;
use flipdot::serial::*;
use flipdot::*;
use flipdot_testing::*;

mod mock_serial_port;
use crate::mock_serial_port::{MockSerialPort, SerialFailure};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ErrorReader {}

impl Read for ErrorReader {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::Other, "Dummy read error"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BusFailure {
    Error,
    WrongMessage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ErrorSignBus {
    failure: BusFailure,
}

impl ErrorSignBus {
    pub fn new(failure: BusFailure) -> Self {
        ErrorSignBus { failure }
    }
}

impl SignBus for ErrorSignBus {
    fn process_message<'a>(&mut self, _: Message<'_>) -> Result<Option<Message<'a>>, Box<dyn Error + Send + Sync>> {
        match self.failure {
            BusFailure::Error => Err(format!("Dummy sign bus error"))?,
            BusFailure::WrongMessage => Ok(Some(Message::Goodbye(Address(0)))),
        }
    }
}

#[test]
fn format_errors() -> Result<(), Box<dyn Error>> {
    // Core
    print_error("Too much data", Data::try_new(vec![0; 256]));
    print_error("I/O error", Frame::read(&mut ErrorReader {}));
    print_error("Bad frame data", Frame::from_bytes(b":01"));
    print_error("Wrong frame data size", Frame::from_bytes(b":01007F027E"));
    print_error("Wrong frame checksum", Frame::from_bytes(b":01007F02FF7E"));
    print_error("Wrong page data length", Page::from_bytes(90, 7, vec![1, 2, 3]));
    print_error("Wrong config data length", SignType::from_bytes(&vec![1, 2, 3]));
    print_error("Unknown config", SignType::from_bytes(&vec![0; 16]));

    // Serial
    print_error(
        "Serial config failure",
        SerialSignBus::try_new(MockSerialPort::new(vec![], SerialFailure::WriteSettings)),
    );

    // Testing
    let mut odk = Odk::try_new(
        MockSerialPort::new(vec![], SerialFailure::Read),
        ErrorSignBus::new(BusFailure::Error),
    )?;
    print_error("ODK read error", odk.process_message());

    let mut odk = Odk::try_new(
        MockSerialPort::new(b":01007F02FF7F\r\n".to_vec(), SerialFailure::None),
        ErrorSignBus::new(BusFailure::Error),
    )?;
    print_error("ODK bus error", odk.process_message());

    // Flipdot
    let bus = Rc::new(RefCell::new(ErrorSignBus::new(BusFailure::Error)));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
    print_error("Sign bus error", sign.configure());

    let bus = Rc::new(RefCell::new(ErrorSignBus::new(BusFailure::WrongMessage)));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
    print_error("Sign wrong message", sign.configure());

    Ok(())
}

fn print_error<V: Debug, E: Error>(title: &'static str, result: Result<V, E>) {
    println!("** {} **", title);

    let e = result.unwrap_err();
    let mut current = Some(&e as &dyn Error);
    let errors = iter::from_fn(|| {
        let ret = current;
        current = current.and_then(|err| err.source());
        ret
    });

    let headings = iter::once("Error").chain(iter::repeat("Caused by"));
    for (heading, failure) in headings.zip(errors) {
        println!("{}: {}", heading, failure);
    }
    println!();
}
