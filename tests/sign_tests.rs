use std::cell::RefCell;
use std::error::Error;
use std::io;
use std::rc::Rc;

use flipdot::core::{ChunkCount, Data, Message, Offset, Operation, State};
use flipdot::{Address, Page, PageFlipStyle, PageId, Sign, SignBus, SignError, SignType};

const CONFIG: &[u8] = &[
    0x04, 0x20, 0x00, 0x06, 0x07, 0x1E, 0x1E, 0x1E, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

#[rustfmt::skip]
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
    fn process_message<'a>(&mut self, message: Message<'_>) -> Result<Option<Message<'a>>, Box<dyn Error + Send + Sync>> {
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
    pub response: Result<Option<Message<'static>>, Box<dyn Error + Send + Sync>>,
}

#[test]
fn happy_path() -> Result<(), Box<dyn Error>> {
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
            expected: Message::SendData(Offset(0), Data::try_new(CONFIG).unwrap()),
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
            expected: Message::SendData(Offset(0), Data::try_new(&DATA[0..16]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(16), Data::try_new(&DATA[16..32]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(32), Data::try_new(&DATA[32..48]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(48), Data::try_new(&DATA[48..64]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(64), Data::try_new(&DATA[64..80]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(80), Data::try_new(&DATA[80..96]).unwrap()),
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

    sign.configure()?;

    let page = Page::from_bytes(90, 7, DATA)?;
    assert_eq!(PageFlipStyle::Manual, sign.send_pages(&[page])?);

    sign.show_loaded_page()?;

    bus.borrow_mut().done();

    Ok(())
}

#[test]
fn config_retry() -> Result<(), Box<dyn Error>> {
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
            expected: Message::SendData(Offset(0), Data::try_new(CONFIG).unwrap()),
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
            expected: Message::SendData(Offset(0), Data::try_new(CONFIG).unwrap()),
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

    sign.configure()?;

    bus.borrow_mut().done();

    Ok(())
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
            expected: Message::SendData(Offset(0), Data::try_new(CONFIG).unwrap()),
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
            expected: Message::SendData(Offset(0), Data::try_new(CONFIG).unwrap()),
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

    let error = sign.configure().unwrap_err();
    assert!(matches!(error, SignError::UnexpectedResponse { .. }));

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
            expected: Message::SendData(Offset(0), Data::try_new(CONFIG).unwrap()),
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
            expected: Message::SendData(Offset(0), Data::try_new(CONFIG).unwrap()),
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
            expected: Message::SendData(Offset(0), Data::try_new(CONFIG).unwrap()),
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

    let error = sign.configure().unwrap_err();
    assert!(matches!(error, SignError::UnexpectedResponse { .. }));

    bus.borrow_mut().done();
}

#[test]
fn pixels_retry() -> Result<(), Box<dyn Error>> {
    let script = vec![
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ReceivePixels),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ReceivePixels))),
        },
        ScriptItem {
            expected: Message::SendData(Offset(0), Data::try_new(&DATA[0..16]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(16), Data::try_new(&DATA[16..32]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(32), Data::try_new(&DATA[32..48]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(48), Data::try_new(&DATA[48..64]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(64), Data::try_new(&DATA[64..80]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(80), Data::try_new(&DATA[80..96]).unwrap()),
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
            expected: Message::SendData(Offset(0), Data::try_new(&DATA[0..16]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(16), Data::try_new(&DATA[16..32]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(32), Data::try_new(&DATA[32..48]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(48), Data::try_new(&DATA[48..64]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(64), Data::try_new(&DATA[64..80]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(80), Data::try_new(&DATA[80..96]).unwrap()),
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
    ];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    let page = Page::from_bytes(90, 7, DATA)?;
    assert_eq!(PageFlipStyle::Manual, sign.send_pages(&[page])?);

    bus.borrow_mut().done();

    Ok(())
}

#[test]
fn pixels_auto_flip() -> Result<(), Box<dyn Error>> {
    let script = vec![
        ScriptItem {
            expected: Message::RequestOperation(Address(3), Operation::ReceivePixels),
            response: Ok(Some(Message::AckOperation(Address(3), Operation::ReceivePixels))),
        },
        ScriptItem {
            expected: Message::SendData(Offset(0), Data::try_new(&DATA[0..16]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(16), Data::try_new(&DATA[16..32]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(32), Data::try_new(&DATA[32..48]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(48), Data::try_new(&DATA[48..64]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(64), Data::try_new(&DATA[64..80]).unwrap()),
            response: Ok(None),
        },
        ScriptItem {
            expected: Message::SendData(Offset(80), Data::try_new(&DATA[80..96]).unwrap()),
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
            response: Ok(Some(Message::ReportState(Address(3), State::ShowingPages))),
        },
    ];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    let page = Page::from_bytes(90, 7, DATA)?;
    assert_eq!(PageFlipStyle::Automatic, sign.send_pages(&[page])?);

    bus.borrow_mut().done();

    Ok(())
}

#[test]
fn page_flip() -> Result<(), Box<dyn Error>> {
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

    sign.show_loaded_page()?;
    sign.load_next_page()?;
    sign.show_loaded_page()?;

    bus.borrow_mut().done();

    Ok(())
}

#[test]
fn shut_down() -> Result<(), Box<dyn Error>> {
    let script = vec![ScriptItem {
        expected: Message::Goodbye(Address(3)),
        response: Ok(None),
    }];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    sign.shut_down()?;

    bus.borrow_mut().done();

    Ok(())
}

#[test]
fn config_needs_reset() -> Result<(), Box<dyn Error>> {
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
            expected: Message::SendData(Offset(0), Data::try_new(CONFIG).unwrap()),
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

    sign.configure()?;

    bus.borrow_mut().done();

    Ok(())
}

#[test]
fn config_ready_to_reset() -> Result<(), Box<dyn Error>> {
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
            expected: Message::SendData(Offset(0), Data::try_new(CONFIG).unwrap()),
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

    sign.configure()?;

    bus.borrow_mut().done();

    Ok(())
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
            expected: Message::SendData(Offset(0), Data::try_new(CONFIG).unwrap()),
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

    let error = sign.configure().unwrap_err();
    assert!(matches!(error, SignError::UnexpectedResponse { .. }));

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

    let error = sign.show_loaded_page().unwrap_err();
    assert!(matches!(error, SignError::UnexpectedResponse { .. }));

    bus.borrow_mut().done();
}

#[test]
fn error_propagates() {
    let script = vec![ScriptItem {
        expected: Message::Hello(Address(3)),
        response: Err(io::Error::new(io::ErrorKind::Other, "oh no!").into()),
    }];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    let error = sign.configure().unwrap_err();
    assert!(matches!(error, SignError::Bus { .. }));
    assert!(error.source().unwrap().is::<io::Error>());

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

#[test]
fn configure_if_needed() -> Result<(), Box<dyn Error>> {
    let script = vec![
        ScriptItem {
            expected: Message::Hello(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::Unconfigured))),
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
            expected: Message::SendData(Offset(0), Data::try_new(CONFIG).unwrap()),
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
            expected: Message::Hello(Address(3)),
            response: Ok(Some(Message::ReportState(Address(3), State::ConfigReceived))),
        },
    ];

    let bus = ScriptedSignBus::new(script.into_iter());
    let bus = Rc::new(RefCell::new(bus));
    let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);

    sign.configure_if_needed()?;
    sign.configure_if_needed()?;

    bus.borrow_mut().done();

    Ok(())
}
