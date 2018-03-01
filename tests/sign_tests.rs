extern crate flipdot;
#[macro_use]
extern crate matches;

use std::cell::RefCell;
use std::error::Error;
use std::io;
use std::rc::Rc;

use flipdot::core::{ChunkCount, Data, Message, Offset, Operation, State};
use flipdot::{Address, Page, PageId, Sign, SignBus, SignType};

const CONFIG: &[u8] = &[
    0x04, 0x20, 0x00, 0x06, 0x07, 0x1E, 0x1E, 0x1E, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

#[cfg_attr(rustfmt, rustfmt_skip)]
const DATA: &[u8] = &[
    0x01, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7F, 0x7F, 0x06, 0x0C, 0x18, 0x7F, 0x7F, 0x00,
    0x3E, 0x7F, 0x41, 0x41, 0x7F, 0x3E, 0x00, 0x01, 0x01, 0x7F, 0x7F, 0x01, 0x01, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x41, 0x7F, 0x7F, 0x41, 0x00, 0x7F, 0x7F, 0x06, 0x0C, 0x18, 0x7F, 0x7F, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x26, 0x6F, 0x49, 0x49, 0x7B, 0x32, 0x00, 0x7F, 0x7F, 0x49, 0x49, 0x41, 0x00,
    0x7F, 0x7F, 0x19, 0x39, 0x6F, 0x46, 0x00, 0x0F, 0x1F, 0x30, 0x60, 0x30, 0x1F, 0x0F, 0x00, 0x41,
    0x7F, 0x7F, 0x41, 0x00, 0x3E, 0x7F, 0x41, 0x41, 0x63, 0x22, 0x00, 0x7F, 0x7F, 0x49, 0xFF, 0xFF,
];

/// Mock implementation of `SignBus` that verifies the messages sent to it
/// follow a predefined script and returns a canned response for each one.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ScriptedSignBus<I: Iterator<Item = ScriptItem>> {
    iter: I,
}

impl<I: Iterator<Item = ScriptItem>> SignBus for ScriptedSignBus<I> {
    fn process_message<'a>(&mut self, message: Message) -> Result<Option<Message<'a>>, Box<Error + Send>> {
        let current_row = self.iter.next().expect("Ran out of scripted responses");
        assert_eq!(current_row.expected, message);
        current_row.response
    }
}

impl<I: Iterator<Item = ScriptItem>> ScriptedSignBus<I> {
    pub fn new(iter: I) -> Self {
        ScriptedSignBus { iter }
    }

    pub fn done(&mut self) {
        if self.iter.next().is_some() {
            panic!("Did not use all scripted messages");
        }
    }
}

#[derive(Debug)]
struct ScriptItem {
    pub expected: Message<'static>,
    pub response: Result<Option<Message<'static>>, Box<Error + Send>>,
}

#[test]
fn happy_path() {
    let script = vec![
        ScriptItem {
            expected: Message::Hello(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::Unconfigured))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ReceiveConfig),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig))),
        },
        ScriptItem {
            expected: Message::SendData(Offset(0), Data::new(CONFIG).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::DataChunksSent(ChunkCount(1)),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::ConfigReceived))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ReceivePixels),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ReceivePixels))),
        },
        ScriptItem {
            expected: Message::SendData(Offset(0), Data::new(&DATA[0..16]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(16), Data::new(&DATA[16..32]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(32), Data::new(&DATA[32..48]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(48), Data::new(&DATA[48..64]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(64), Data::new(&DATA[64..80]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(80), Data::new(&DATA[80..96]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::DataChunksSent(ChunkCount(6)),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PixelsReceived))),
        },
        ScriptItem {
            expected: Message::PixelsComplete(Address(3)),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageLoaded))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ShowLoadedPage),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ShowLoadedPage))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageShowInProgress))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageShown))),
        },
    ];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    sign.configure().unwrap();

    let page = Page::from_bytes(90, 7, DATA).unwrap();
    sign.send_pages(&[page]).unwrap();

    sign.show_loaded_page().unwrap();

    bus.borrow_mut().done();
}

#[test]
fn config_retry() {
    let script = vec![
        ScriptItem {
            expected: Message::Hello(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::Unconfigured))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ReceiveConfig),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig))),
        },
        ScriptItem {
            expected: Message::SendData(Offset(0), Data::new(CONFIG).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::DataChunksSent(ChunkCount(1)),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::ConfigFailed))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ReceiveConfig),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig))),
        },
        ScriptItem {
            expected: Message::SendData(Offset(0), Data::new(CONFIG).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::DataChunksSent(ChunkCount(1)),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::ConfigReceived))),
        },
    ];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    sign.configure().unwrap();

    bus.borrow_mut().done();
}

#[test]
fn config_retry_unexpected_state_fails() {
    let script = vec![
        ScriptItem {
            expected: Message::Hello(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::Unconfigured))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ReceiveConfig),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig))),
        },
        ScriptItem {
            expected: Message::SendData(Offset(0), Data::new(CONFIG).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::DataChunksSent(ChunkCount(1)),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::ConfigFailed))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ReceiveConfig),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig))),
        },
        ScriptItem {
            expected: Message::SendData(Offset(0), Data::new(CONFIG).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::DataChunksSent(ChunkCount(1)),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageShown))),
        },
    ];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    let result = sign.configure();
    assert_matches!(result, Err(flipdot::Error(flipdot::ErrorKind::UnexpectedResponse(_, _), _)));

    bus.borrow_mut().done();
}

#[test]
fn config_retry_gives_up() {
    let script = vec![
        ScriptItem {
            expected: Message::Hello(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::Unconfigured))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ReceiveConfig),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig))),
        },
        ScriptItem {
            expected: Message::SendData(Offset(0), Data::new(CONFIG).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::DataChunksSent(ChunkCount(1)),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::ConfigFailed))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ReceiveConfig),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig))),
        },
        ScriptItem {
            expected: Message::SendData(Offset(0), Data::new(CONFIG).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::DataChunksSent(ChunkCount(1)),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::ConfigFailed))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ReceiveConfig),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig))),
        },
        ScriptItem {
            expected: Message::SendData(Offset(0), Data::new(CONFIG).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::DataChunksSent(ChunkCount(1)),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::ConfigFailed))),
        },
    ];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    let result = sign.configure();
    assert_matches!(result, Err(flipdot::Error(flipdot::ErrorKind::UnexpectedResponse(_, _), _)));

    bus.borrow_mut().done();
}

#[test]
fn pixels_retry() {
    let script = vec![
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ReceivePixels),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ReceivePixels))),
        },
        ScriptItem {
            expected: Message::SendData(Offset(0), Data::new(&DATA[0..16]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(16), Data::new(&DATA[16..32]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(32), Data::new(&DATA[32..48]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(48), Data::new(&DATA[48..64]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(64), Data::new(&DATA[64..80]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(80), Data::new(&DATA[80..96]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::DataChunksSent(ChunkCount(6)),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PixelsFailed))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ReceivePixels),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ReceivePixels))),
        },
        ScriptItem {
            expected: Message::SendData(Offset(0), Data::new(&DATA[0..16]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(16), Data::new(&DATA[16..32]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(32), Data::new(&DATA[32..48]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(48), Data::new(&DATA[48..64]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(64), Data::new(&DATA[64..80]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(80), Data::new(&DATA[80..96]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::DataChunksSent(ChunkCount(6)),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PixelsReceived))),
        },
        ScriptItem {
            expected: Message::PixelsComplete(Address(3)),
            response: Ok(None),
        },
    ];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    let page = Page::from_bytes(90, 7, DATA).unwrap();
    sign.send_pages(&[page]).unwrap();

    bus.borrow_mut().done();
}

#[test]
fn page_flip() {
    let script = vec![
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageLoaded))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ShowLoadedPage),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ShowLoadedPage))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageShowInProgress))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageShowInProgress))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageShowInProgress))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageShowInProgress))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageShown))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageShown))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::LoadNextPage),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::LoadNextPage))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageLoadInProgress))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageLoadInProgress))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageLoadInProgress))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageLoadInProgress))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageLoaded))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageLoaded))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ShowLoadedPage),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ShowLoadedPage))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageShowInProgress))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageShowInProgress))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageShowInProgress))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageShowInProgress))),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageShown))),
        },
    ];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    sign.show_loaded_page().unwrap();
    sign.load_next_page().unwrap();
    sign.show_loaded_page().unwrap();

    bus.borrow_mut().done();
}

#[test]
fn shut_down() {
    let script = vec![ScriptItem {
        expected: Message::Goodbye(Address(3)),
        response: Ok(None),
    }];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    sign.shut_down().unwrap();

    bus.borrow_mut().done();
}

#[test]
fn config_needs_reset() {
    let script = vec![
        ScriptItem {
            expected: Message::Hello(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageShown))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::StartReset),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::StartReset))),
        },
        ScriptItem {
            expected: Message::Hello(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::ReadyToReset))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::FinishReset),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::FinishReset))),
        },
        ScriptItem {
            expected: Message::Hello(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::Unconfigured))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ReceiveConfig),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig))),
        },
        ScriptItem {
            expected: Message::SendData(Offset(0), Data::new(CONFIG).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::DataChunksSent(ChunkCount(1)),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::ConfigReceived))),
        },
    ];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    sign.configure().unwrap();

    bus.borrow_mut().done();
}

#[test]
fn config_ready_to_reset() {
    let script = vec![
        ScriptItem {
            expected: Message::Hello(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::ReadyToReset))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::FinishReset),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::FinishReset))),
        },
        ScriptItem {
            expected: Message::Hello(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::Unconfigured))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ReceiveConfig),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig))),
        },
        ScriptItem {
            expected: Message::SendData(Offset(0), Data::new(CONFIG).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::DataChunksSent(ChunkCount(1)),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::ConfigReceived))),
        },
    ];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    sign.configure().unwrap();

    bus.borrow_mut().done();
}

#[test]
fn unexpected_response_error() {
    let script = vec![
        ScriptItem {
            expected: Message::Hello(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::Unconfigured))),
        },
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ReceiveConfig),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig))),
        },
        ScriptItem {
            expected: Message::SendData(Offset(0), Data::new(CONFIG).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::DataChunksSent(ChunkCount(1)),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::QueryState(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::PageShown))),
        },
    ];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    let result = sign.configure();
    assert_matches!(result, Err(flipdot::Error(flipdot::ErrorKind::UnexpectedResponse(_, _), _)));

    bus.borrow_mut().done();
}

#[test]
fn flip_page_unexpected_response_error() {
    let script = vec![ScriptItem {
        expected: Message::QueryState(Address(3)),
        response: Ok(Some(Message::ReportState(Address(3), State::Unconfigured))),
    }];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    let result = sign.show_loaded_page();
    assert_matches!(result, Err(flipdot::Error(flipdot::ErrorKind::UnexpectedResponse(_, _), _)));

    bus.borrow_mut().done();
}

#[test]
fn error_propagates() {
    let script = vec![ScriptItem {
        expected: Message::Hello(Address(3)),
        response: Err(Box::new(io::Error::new(io::ErrorKind::Other, "oh no!"))),
    }];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    let result = sign.configure();
    assert_matches!(result, Err(flipdot::Error(flipdot::ErrorKind::Bus, _)));

    bus.borrow_mut().done();
}

#[test]
fn create_page() {
    let script = vec![];
    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    let page = sign.create_page(PageId(17));
    assert_eq!(PageId(17), page.id());
    assert_eq!(sign.width(), page.width());
    assert_eq!(sign.height(), page.height());

    bus.borrow_mut().done();
}
