use std::mem;

use log::{debug, info, warn};

use flipdot_core::{Address, ChunkCount, Message, Offset, Operation, Page, SignBus, SignType, State};

/// Mock implementation of a bus containing one or more signs.
///
/// The bus is populated with one or more [`VirtualSign`]s which actually implement the sign protocol.
/// `VirtualSignBus` forwards messages to each virtual sign in turn until one of them handles it.
///
/// While most likely not a 100% accurate implementation of the protocol, it is sufficient
/// for interacting with a real ODK.
///
/// Messages and responses are logged using the [`log`] crate for debugging purposes. Consuming binaries
/// typically use the [`env_logger`] crate and can be run with the `RUST_LOG=debug` environment variable
/// to watch the bus messages go by.
///
/// # Examples
///
/// ```no_run
/// use flipdot_serial::SerialSignBus;
/// use flipdot_testing::{Address, Odk, VirtualSign, VirtualSignBus};
///
/// # fn main() -> Result<(), failure::Error> {
/// #
/// let bus = VirtualSignBus::new(vec![VirtualSign::new(Address(3))]);
/// let port = serial::open("/dev/ttyUSB0")?;
/// let mut odk = Odk::new(port, bus)?;
/// loop {
///     // VirtualSignBus processes the messsages from the real ODK over serial.
///     odk.process_message()?;
/// }
/// #
/// # Ok(()) }
/// ```
///
/// [`VirtualSign`]: struct.VirtualSign.html
/// [`log`]: https://crates.io/crates/log
/// [`env_logger`]: https://crates.io/crates/env_logger
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VirtualSignBus<'a> {
    signs: Vec<VirtualSign<'a>>,
}

impl<'a> VirtualSignBus<'a> {
    /// Creates a new `VirtualSignBus` with the specified virtual signs.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use flipdot_serial::SerialSignBus;
    /// # use flipdot_testing::{Address, Odk, VirtualSign, VirtualSignBus};
    /// #
    /// # fn main() -> Result<(), failure::Error> {
    /// #
    /// let bus = VirtualSignBus::new(vec![VirtualSign::new(Address(3))]);
    /// let port = serial::open("COM3")?;
    /// let odk = Odk::new(port, bus)?;
    /// #
    /// # Ok(()) }
    /// ```
    pub fn new<I>(signs: I) -> Self
    where
        I: IntoIterator<Item = VirtualSign<'a>>,
    {
        VirtualSignBus {
            signs: signs.into_iter().collect(),
        }
    }

    /// Returns a reference to the [`VirtualSign`] at a specific index matching the original order passed to `new`.
    ///
    /// Useful when writing tests in order to verify properties of an individual sign.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_testing::{Address, VirtualSign, VirtualSignBus};
    /// let signs = vec![VirtualSign::new(Address(5)), VirtualSign::new(Address(16))];
    /// let bus = VirtualSignBus::new(signs);
    /// let second_sign = bus.sign(1);
    /// assert_eq!(Address(16), second_sign.address());
    /// ```
    ///
    /// [`VirtualSign`]: struct.VirtualSign.html
    pub fn sign(&self, index: usize) -> &VirtualSign<'a> {
        &self.signs[index]
    }
}

impl SignBus for VirtualSignBus<'_> {
    /// Handles a bus message by trying each sign in turn to see if it can handle it (i.e. returns a `Some` response).
    fn process_message<'a>(&mut self, message: Message<'_>) -> Result<Option<Message<'a>>, failure::Error> {
        debug!("Bus message: {}", message);
        for sign in &mut self.signs {
            let response = sign.process_message(&message);
            if let Some(response_message) = response {
                debug!(" Vsign {:04X}: {}", sign.address().0, response_message);
                return Ok(Some(response_message));
            }
        }
        Ok(None)
    }
}

/// Mock implementation of a single sign on a [`VirtualSignBus`].
///
/// Encapsulates all the state associated with a virtual sign and implements the sign protocol for it.
/// In general, you do not need to interact with this class directly; you simply pass it off to a
/// [`VirtualSignBus`], which forwards messages appropriately.
///
/// # Examples
///
/// See [`VirtualSignBus`].
///
/// [`VirtualSignBus`]: struct.VirtualSignBus.html
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct VirtualSign<'a> {
    address: Address,
    state: State,
    pages: Vec<Page<'a>>,
    pending_data: Vec<u8>,
    data_chunks: u16,
    width: u32,
    height: u32,
    sign_type: Option<SignType>,
}

impl VirtualSign<'_> {
    /// Creates a new `VirtualSign` with the specified address.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::iter;
    /// # use flipdot_testing::{Address, VirtualSign, VirtualSignBus};
    /// let sign = VirtualSign::new(Address(22));
    /// let bus = VirtualSignBus::new(iter::once(sign));
    /// ```
    pub fn new(address: Address) -> Self {
        VirtualSign {
            address,
            state: State::Unconfigured,
            pages: vec![],
            pending_data: vec![],
            data_chunks: 0,
            width: 0,
            height: 0,
            sign_type: None,
        }
    }

    /// Returns the sign's address.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_testing::{Address, VirtualSign};
    /// let sign = VirtualSign::new(Address(22));
    /// assert_eq!(Address(22), sign.address());
    /// ```
    pub fn address(&self) -> Address {
        self.address
    }

    /// Returns the sign's current state.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_core::State;
    /// # use flipdot_testing::{Address, VirtualSign};
    /// #
    /// let sign = VirtualSign::new(Address(3));
    /// assert_eq!(State::Unconfigured, sign.state());
    /// ```
    pub fn state(&self) -> State {
        self.state
    }

    /// Returns the sign's configured type.
    ///
    /// This is initially `None` and will only be set if the sign has received a configuration message over the bus.
    /// Note that even if it has, this may still be `None` if the configuration did not match any known types
    /// (e.g. potentially when driving from a real ODK).
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_testing::{Address, VirtualSign};
    /// let sign = VirtualSign::new(Address(17));
    /// assert_eq!(None, sign.sign_type());
    /// ```
    pub fn sign_type(&self) -> Option<SignType> {
        self.sign_type
    }

    /// Returns the sign's current `Page`s as a slice.
    ///
    /// May be empty if no pages have yet been sent to this sign or it has been reset.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_testing::{Address, VirtualSign};
    /// let sign = VirtualSign::new(Address(1));
    /// assert!(sign.pages().is_empty());
    /// ```
    pub fn pages(&self) -> &[Page<'_>] {
        &self.pages
    }

    /// Handle a bus message, updating our state accordingly.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_core::{Message, State};
    /// # use flipdot_testing::{Address, VirtualSign};
    /// #
    /// let mut sign = VirtualSign::new(Address(3));
    /// let response = sign.process_message(&Message::QueryState(Address(3)));
    /// assert_eq!(Some(Message::ReportState(Address(3), State::Unconfigured)), response);
    /// ```
    pub fn process_message<'a>(&mut self, message: &Message<'_>) -> Option<Message<'a>> {
        match *message {
            Message::Hello(address) | Message::QueryState(address) if address == self.address => self.query_state(),
            Message::RequestOperation(address, Operation::ReceiveConfig) if address == self.address => self.receive_config(),
            Message::SendData(offset, ref data) => self.send_data(offset, data.get()),
            Message::DataChunksSent(chunks) => self.data_chunks_sent(chunks),
            Message::RequestOperation(address, Operation::ReceivePixels) if address == self.address => self.receive_pixels(),
            Message::PixelsComplete(address) if address == self.address => self.pixels_complete(),
            Message::RequestOperation(address, Operation::ShowLoadedPage) if address == self.address => self.show_loaded_page(),
            Message::RequestOperation(address, Operation::LoadNextPage) if address == self.address => self.load_next_page(),
            Message::RequestOperation(address, Operation::StartReset) if address == self.address => self.start_reset(),
            Message::RequestOperation(address, Operation::FinishReset) if address == self.address => self.finish_reset(),
            Message::Goodbye(address) if address == self.address => self.goodbye(),
            _ => None,
        }
    }

    /// Handles `QueryState` or `Hello` messages
    fn query_state<'a>(&mut self) -> Option<Message<'a>> {
        let state = self.state;

        // We don't actually need to do anything to load or show a page,
        // so just flip over to the final state for the next time we get asked.
        match state {
            State::PageLoadInProgress => self.state = State::PageLoaded,
            State::PageShowInProgress => self.state = State::PageShown,
            _ => {}
        };

        Some(Message::ReportState(self.address, state))
    }

    /// Handles `RequestOperation` messages for `ReceiveConfig`.
    fn receive_config<'a>(&mut self) -> Option<Message<'a>> {
        match self.state {
            State::Unconfigured | State::ConfigFailed => {
                self.state = State::ConfigInProgress;
                Some(Message::AckOperation(self.address, Operation::ReceiveConfig))
            }
            _ => None,
        }
    }

    /// Handles `SendData` messages.
    fn send_data<'a>(&mut self, offset: Offset, data: &[u8]) -> Option<Message<'a>> {
        if self.state == State::ConfigInProgress && offset == Offset(0) && data.len() == 16 {
            let (kind, width, height) = match data[0] {
                0x04 => ("Max3000", data[5..9].iter().sum(), data[4]),
                0x08 => ("Horizon", data[7], data[5]),
                _ => return None,
            };

            info!(
                "Vsign {:04X} configuration: {} x {} {} sign",
                self.address.0, width, height, kind
            );

            self.sign_type = SignType::from_bytes(data).ok();
            match self.sign_type {
                Some(sign_type) => info!("Vsign {:04X} matches known type: {:?}", self.address.0, sign_type),
                None => warn!("Please report unknown configuration {:?}", data),
            }

            self.width = u32::from(width);
            self.height = u32::from(height);
            self.data_chunks += 1;
        } else if self.state == State::PixelsInProgress {
            if offset == Offset(0) {
                self.flush_pixels();
            }
            self.pending_data.extend_from_slice(data);
            self.data_chunks += 1;
        }
        None
    }

    /// Handles `DataChunksSent` messages.
    fn data_chunks_sent<'a>(&mut self, chunks: ChunkCount) -> Option<Message<'a>> {
        if ChunkCount(self.data_chunks) == chunks {
            match self.state {
                State::ConfigInProgress => self.state = State::ConfigReceived,
                State::PixelsInProgress => self.state = State::PixelsReceived,
                _ => {}
            }
        } else {
            match self.state {
                State::ConfigInProgress => self.state = State::ConfigFailed,
                State::PixelsInProgress => self.state = State::PixelsFailed,
                _ => {}
            }
        }
        self.flush_pixels();
        self.data_chunks = 0;
        None
    }

    /// Handles `RequestOperation` messages for `ReceivePixels`.
    fn receive_pixels<'a>(&mut self) -> Option<Message<'a>> {
        match self.state {
            State::ConfigReceived
            | State::PixelsFailed
            | State::PageLoaded
            | State::PageLoadInProgress
            | State::PageShown
            | State::PageShowInProgress => {
                self.state = State::PixelsInProgress;
                self.pages.clear();
                Some(Message::AckOperation(self.address, Operation::ReceivePixels))
            }
            _ => None,
        }
    }

    /// Handles `PixelsComplete` messages.
    fn pixels_complete<'a>(&mut self) -> Option<Message<'a>> {
        if self.state == State::PixelsReceived {
            self.state = State::PageLoaded;
            for page in &self.pages {
                info!(
                    "Vsign {:04X} Page {} ({} x {})\n{}",
                    self.address.0,
                    page.id(),
                    page.width(),
                    page.height(),
                    page
                );
            }
        }
        None
    }

    /// Handles `RequestOperation` messages for `ShowLoadedPage`.
    fn show_loaded_page<'a>(&mut self) -> Option<Message<'a>> {
        if self.state == State::PageLoaded {
            self.state = State::PageShowInProgress;
            Some(Message::AckOperation(self.address, Operation::ShowLoadedPage))
        } else {
            None
        }
    }

    /// Handles `RequestOperation` messages for `LoadNextPage`.
    fn load_next_page<'a>(&mut self) -> Option<Message<'a>> {
        if self.state == State::PageShown {
            self.state = State::PageLoadInProgress;
            Some(Message::AckOperation(self.address, Operation::LoadNextPage))
        } else {
            None
        }
    }

    /// Handles `RequestOperation` messages for `StartReset`.
    fn start_reset<'a>(&mut self) -> Option<Message<'a>> {
        self.state = State::ReadyToReset;
        Some(Message::AckOperation(self.address, Operation::StartReset))
    }

    /// Handles `RequestOperation` messages for `FinishReset`.
    fn finish_reset<'a>(&mut self) -> Option<Message<'a>> {
        if self.state == State::ReadyToReset {
            self.reset();
            Some(Message::AckOperation(self.address, Operation::FinishReset))
        } else {
            None
        }
    }

    /// Handles `Goodbye` messages.
    fn goodbye<'a>(&mut self) -> Option<Message<'a>> {
        self.reset();
        None
    }

    /// Convert the currently-buffered pixel data into a `Page` and add it to our page vector.
    fn flush_pixels(&mut self) {
        if !self.pending_data.is_empty() {
            let data = mem::replace(&mut self.pending_data, Default::default());
            if self.width > 0 && self.height > 0 {
                let page = Page::from_bytes(self.width, self.height, data).expect("Error loading page");
                self.pages.push(page);
            }
        }
    }

    /// Reset the sign back to its initial unconfigured state. Used for the reset and shutdown operations.
    fn reset(&mut self) {
        self.state = State::Unconfigured;
        self.pages.clear();
        self.pending_data.clear();
        self.data_chunks = 0;
        self.width = 0;
        self.height = 0;
        self.sign_type = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flipdot_core::{Data, PageId};

    #[test]
    fn normal_behavior() {
        let mut page1 = Page::new(PageId(0), 90, 7);
        for x in 0..page1.width() {
            for y in 0..page1.height() {
                page1.set_pixel(x, y, x % 2 == y % 2);
            }
        }

        let mut page2 = Page::new(PageId(1), 90, 7);
        for x in 0..page2.width() {
            for y in 0..page2.height() {
                page2.set_pixel(x, y, x % 2 != y % 2);
            }
        }

        // Initial values
        let mut sign = VirtualSign::new(Address(3));
        assert_eq!(Address(3), sign.address());
        assert_eq!(None, sign.sign_type());
        assert_eq!(0, sign.pages().len());
        assert_eq!(0, sign.width);
        assert_eq!(0, sign.height);

        // Discover and configuration
        let response = sign.process_message(&Message::Hello(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::Unconfigured)), response);

        let response = sign.process_message(&Message::RequestOperation(Address(3), Operation::ReceiveConfig));
        assert_eq!(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig)), response);

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::ConfigInProgress)), response);

        let response = sign.process_message(&Message::SendData(
            Offset(0x00),
            Data::new(SignType::Max3000Side90x7.to_bytes()).unwrap(),
        ));
        assert_eq!(None, response);

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::ConfigInProgress)), response);

        let response = sign.process_message(&Message::DataChunksSent(ChunkCount(1)));
        assert_eq!(None, response);

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::ConfigReceived)), response);

        assert_eq!(Some(SignType::Max3000Side90x7), sign.sign_type());
        assert_eq!(90, sign.width);
        assert_eq!(7, sign.height);

        let response = sign.process_message(&Message::RequestOperation(Address(3), Operation::ReceivePixels));
        assert_eq!(Some(Message::AckOperation(Address(3), Operation::ReceivePixels)), response);

        // Send page
        assert_eq!(0, sign.pages().len());

        let mut chunks_sent = 0;
        for (i, chunk) in page1.as_bytes().chunks(16).enumerate() {
            let response = sign.process_message(&Message::QueryState(Address(3)));
            assert_eq!(Some(Message::ReportState(Address(3), State::PixelsInProgress)), response);

            let response = sign.process_message(&Message::SendData(Offset((i * 16) as u16), Data::new(chunk).unwrap()));
            assert_eq!(None, response);

            chunks_sent += 1;
        }

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::PixelsInProgress)), response);

        let response = sign.process_message(&Message::DataChunksSent(ChunkCount(chunks_sent)));
        assert_eq!(None, response);

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::PixelsReceived)), response);

        let response = sign.process_message(&Message::PixelsComplete(Address(3)));
        assert_eq!(None, response);

        assert_eq!(&[page1], sign.pages());

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::PageLoaded)), response);

        // Show page
        let response = sign.process_message(&Message::RequestOperation(Address(3), Operation::ShowLoadedPage));
        assert_eq!(Some(Message::AckOperation(Address(3), Operation::ShowLoadedPage)), response);

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::PageShowInProgress)), response);

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::PageShown)), response);

        // Load next page
        let response = sign.process_message(&Message::RequestOperation(Address(3), Operation::LoadNextPage));
        assert_eq!(Some(Message::AckOperation(Address(3), Operation::LoadNextPage)), response);

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::PageLoadInProgress)), response);

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::PageLoaded)), response);

        // Send different page
        let response = sign.process_message(&Message::RequestOperation(Address(3), Operation::ReceivePixels));
        assert_eq!(Some(Message::AckOperation(Address(3), Operation::ReceivePixels)), response);

        assert_eq!(0, sign.pages().len());

        let mut chunks_sent = 0;
        for (i, chunk) in page2.as_bytes().chunks(16).enumerate() {
            let response = sign.process_message(&Message::QueryState(Address(3)));
            assert_eq!(Some(Message::ReportState(Address(3), State::PixelsInProgress)), response);

            let response = sign.process_message(&Message::SendData(Offset((i * 16) as u16), Data::new(chunk).unwrap()));
            assert_eq!(None, response);

            chunks_sent += 1;
        }

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::PixelsInProgress)), response);

        let response = sign.process_message(&Message::DataChunksSent(ChunkCount(chunks_sent)));
        assert_eq!(None, response);

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::PixelsReceived)), response);

        let response = sign.process_message(&Message::PixelsComplete(Address(3)));
        assert_eq!(None, response);

        assert_eq!(&[page2], sign.pages());

        // Reset
        let response = sign.process_message(&Message::RequestOperation(Address(3), Operation::StartReset));
        assert_eq!(Some(Message::AckOperation(Address(3), Operation::StartReset)), response);

        let response = sign.process_message(&Message::Hello(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::ReadyToReset)), response);

        let response = sign.process_message(&Message::RequestOperation(Address(3), Operation::FinishReset));
        assert_eq!(Some(Message::AckOperation(Address(3), Operation::FinishReset)), response);

        let response = sign.process_message(&Message::Hello(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::Unconfigured)), response);

        assert_eq!(Address(3), sign.address());
        assert_eq!(None, sign.sign_type());
        assert_eq!(0, sign.pages().len());
        assert_eq!(0, sign.width);
        assert_eq!(0, sign.height);

        // Configure again
        let response = sign.process_message(&Message::Hello(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::Unconfigured)), response);

        let response = sign.process_message(&Message::RequestOperation(Address(3), Operation::ReceiveConfig));
        assert_eq!(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig)), response);

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::ConfigInProgress)), response);

        let response = sign.process_message(&Message::SendData(
            Offset(0x00),
            Data::new(SignType::Max3000Side90x7.to_bytes()).unwrap(),
        ));
        assert_eq!(None, response);

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::ConfigInProgress)), response);

        let response = sign.process_message(&Message::DataChunksSent(ChunkCount(1)));
        assert_eq!(None, response);

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::ConfigReceived)), response);

        assert_eq!(Some(SignType::Max3000Side90x7), sign.sign_type());
        assert_eq!(90, sign.width);
        assert_eq!(7, sign.height);

        // Shut down
        let response = sign.process_message(&Message::Goodbye(Address(3)));
        assert_eq!(None, response);

        assert_eq!(None, sign.sign_type());
        assert_eq!(0, sign.pages().len());
        assert_eq!(0, sign.width);
        assert_eq!(0, sign.height);

        let response = sign.process_message(&Message::Hello(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::Unconfigured)), response);
    }

    #[test]
    fn invalid_operations() {
        let mut sign = VirtualSign::new(Address(3));

        let response = sign.process_message(&Message::RequestOperation(Address(3), Operation::ReceivePixels));
        assert_eq!(None, response);

        let response = sign.process_message(&Message::RequestOperation(Address(3), Operation::ShowLoadedPage));
        assert_eq!(None, response);

        let response = sign.process_message(&Message::RequestOperation(Address(3), Operation::LoadNextPage));
        assert_eq!(None, response);

        let response = sign.process_message(&Message::RequestOperation(Address(3), Operation::FinishReset));
        assert_eq!(None, response);

        let response = sign.process_message(&Message::RequestOperation(Address(3), Operation::ReceiveConfig));
        assert_eq!(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig)), response);

        let response = sign.process_message(&Message::RequestOperation(Address(3), Operation::ReceiveConfig));
        assert_eq!(None, response);
    }

    #[test]
    fn unknown_config() {
        let mut sign = VirtualSign::new(Address(3));

        let response = sign.process_message(&Message::Hello(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::Unconfigured)), response);

        let response = sign.process_message(&Message::RequestOperation(Address(3), Operation::ReceiveConfig));
        assert_eq!(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig)), response);

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::ConfigInProgress)), response);

        let data = vec![
            0x04, 0x99, 0x00, 0x0F, 0x09, 0x1C, 0x1C, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let response = sign.process_message(&Message::SendData(Offset(0x00), Data::new(data).unwrap()));
        assert_eq!(None, response);

        let response = sign.process_message(&Message::DataChunksSent(ChunkCount(1)));
        assert_eq!(None, response);

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::ConfigReceived)), response);

        assert_eq!(None, sign.sign_type());
        assert_eq!(56, sign.width);
        assert_eq!(9, sign.height);
    }

    #[test]
    fn invalid_config() {
        let mut sign = VirtualSign::new(Address(3));

        let response = sign.process_message(&Message::Hello(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::Unconfigured)), response);

        let response = sign.process_message(&Message::RequestOperation(Address(3), Operation::ReceiveConfig));
        assert_eq!(Some(Message::AckOperation(Address(3), Operation::ReceiveConfig)), response);

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::ConfigInProgress)), response);

        let data = vec![
            0x0F, 0x99, 0x00, 0x0F, 0x09, 0x1C, 0x1C, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let response = sign.process_message(&Message::SendData(Offset(0x00), Data::new(data).unwrap()));
        assert_eq!(None, response);

        let response = sign.process_message(&Message::DataChunksSent(ChunkCount(1)));
        assert_eq!(None, response);

        let response = sign.process_message(&Message::QueryState(Address(3)));
        assert_eq!(Some(Message::ReportState(Address(3), State::ConfigFailed)), response);

        assert_eq!(None, sign.sign_type());
        assert_eq!(0, sign.width);
        assert_eq!(0, sign.height);
    }
}
