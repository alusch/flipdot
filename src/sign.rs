use std::cell::RefCell;
use std::iter;
use std::rc::Rc;

use log::warn;
use thiserror::Error;

use crate::core::{Address, ChunkCount, Data, Message, Offset, Operation, Page, PageFlipStyle, PageId, SignBus, SignType, State};

/// Errors related to [`Sign`]s.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SignError {
    /// The sign bus failed to process a message.
    #[error("Sign bus failed to process message")]
    Bus {
        /// The underlying bus error.
        #[from]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Sign did not respond properly according to the protocol.
    #[error(
        "Sign did not respond properly according to the protocol: Expected {}, got {}",
        expected,
        actual
    )]
    UnexpectedResponse {
        /// The expected response according to the protocol.
        expected: String,

        /// The actual response received.
        actual: String,
    },
}

/// A single sign on an associated bus.
///
/// Basic operation consists of configuring the sign, sending one or more pages of a message,
/// then requesting a page flip as desired. The types of signs that are supported are "dumb"
/// in that they don't have any display logic of their own; all operations are remotely controlled.
///
/// # Examples
///
/// ```no_run
/// use std::cell::RefCell;
/// use std::rc::Rc;
/// use flipdot::{Address, PageFlipStyle, PageId, Sign, SignType, SerialSignBus};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #
/// // Set up bus. Because the bus can be shared among
/// // multiple signs, it must be wrapped in an Rc<RefCell>.
/// let port = serial::open("/dev/ttyUSB0")?;
/// let bus = SerialSignBus::try_new(port)?;
/// let bus = Rc::new(RefCell::new(bus));
///
/// // Create a sign with the appropriate address and type.
/// let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
///
/// // First, the configuration data must be sent to the sign.
/// sign.configure()?;
///
/// // Next, we can create some pages, turn on pixels, and send them to the sign.
/// let mut page1 = sign.create_page(PageId(0));
/// page1.set_pixel(0, 0, true);
/// let mut page2 = sign.create_page(PageId(1));
/// page2.set_pixel(1, 1, true);
/// if sign.send_pages(&[page1, page2])? == PageFlipStyle::Manual {
///     // The first page is now loaded in the sign's memory and can be shown.
///     sign.show_loaded_page()?;
///
///     // Load the second page into memory, then show it.
///     sign.load_next_page()?;
///     sign.show_loaded_page()?;
/// }
/// #
/// # Ok(()) }
/// ```
#[derive(Debug)]
pub struct Sign {
    address: Address,
    sign_type: SignType,
    bus: Rc<RefCell<dyn SignBus>>,
}

impl Sign {
    /// Creates a new `Sign` with the given address and type, which will represent and control
    /// an actual sign on the provided [`SignBus`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::cell::RefCell;
    /// # use std::rc::Rc;
    /// # use flipdot::{Address, PageId, Sign, SignType};
    /// # use flipdot_testing::VirtualSignBus;
    /// #
    /// # // Placeholder bus for expository purposes
    /// # fn get_bus<'a>() -> Rc<RefCell<VirtualSignBus<'a>>> { Rc::new(RefCell::new(VirtualSignBus::new(vec![]))) }
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #
    /// let bus = get_bus();
    /// let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
    /// #
    /// # Ok(()) }
    /// ```
    pub fn new(bus: Rc<RefCell<dyn SignBus>>, address: Address, sign_type: SignType) -> Self {
        Sign { address, sign_type, bus }
    }

    /// Returns the sign's address.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::cell::RefCell;
    /// # use std::rc::Rc;
    /// # use flipdot::{Address, PageId, Sign, SignType};
    /// # use flipdot_testing::VirtualSignBus;
    /// #
    /// # // Placeholder bus for expository purposes
    /// # fn get_bus<'a>() -> Rc<RefCell<VirtualSignBus<'a>>> { Rc::new(RefCell::new(VirtualSignBus::new(vec![]))) }
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #
    /// let bus = get_bus();
    /// let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
    /// assert_eq!(Address(3), sign.address());
    /// #
    /// # Ok(()) }
    /// ```
    pub fn address(&self) -> Address {
        self.address
    }

    /// Returns the sign's type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::cell::RefCell;
    /// # use std::rc::Rc;
    /// # use flipdot::{Address, PageId, Sign, SignType};
    /// # use flipdot_testing::VirtualSignBus;
    /// #
    /// # // Placeholder bus for expository purposes
    /// # fn get_bus<'a>() -> Rc<RefCell<VirtualSignBus<'a>>> { Rc::new(RefCell::new(VirtualSignBus::new(vec![]))) }
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #
    /// let bus = get_bus();
    /// let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
    /// assert_eq!(SignType::Max3000Side90x7, sign.sign_type());
    /// #
    /// # Ok(()) }
    /// ```
    pub fn sign_type(&self) -> SignType {
        self.sign_type
    }

    /// Returns the width in pixels of the sign's display area.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::cell::RefCell;
    /// # use std::rc::Rc;
    /// # use flipdot::{Address, PageId, Sign, SignType};
    /// # use flipdot_testing::VirtualSignBus;
    /// #
    /// # // Placeholder bus for expository purposes
    /// # fn get_bus<'a>() -> Rc<RefCell<VirtualSignBus<'a>>> { Rc::new(RefCell::new(VirtualSignBus::new(vec![]))) }
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #
    /// let bus = get_bus();
    /// let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
    /// assert_eq!(90, sign.width());
    /// #
    /// # Ok(()) }
    /// ```
    pub fn width(&self) -> u32 {
        self.sign_type.dimensions().0
    }

    /// Returns the height in pixels of the sign's display area.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::cell::RefCell;
    /// # use std::rc::Rc;
    /// # use flipdot::{Address, PageId, Sign, SignType};
    /// # use flipdot_testing::VirtualSignBus;
    /// #
    /// # // Placeholder bus for expository purposes
    /// # fn get_bus<'a>() -> Rc<RefCell<VirtualSignBus<'a>>> { Rc::new(RefCell::new(VirtualSignBus::new(vec![]))) }
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #
    /// let bus = get_bus();
    /// let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
    /// assert_eq!(7, sign.height());
    /// #
    /// # Ok(()) }
    /// ```
    pub fn height(&self) -> u32 {
        self.sign_type.dimensions().1
    }

    /// Creates a page with the given ID that matches the sign's dimensions.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::cell::RefCell;
    /// # use std::rc::Rc;
    /// # use flipdot::{Address, PageId, Sign, SignType};
    /// # use flipdot_testing::VirtualSignBus;
    /// #
    /// # // Placeholder bus for expository purposes
    /// # fn get_bus<'a>() -> Rc<RefCell<VirtualSignBus<'a>>> { Rc::new(RefCell::new(VirtualSignBus::new(vec![]))) }
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #
    /// let bus = get_bus();
    /// let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
    /// let mut page = sign.create_page(PageId(1));
    ///
    /// assert_eq!(PageId(1), page.id());
    /// assert_eq!(page.width(), sign.width());
    /// assert_eq!(page.height(), sign.height());
    ///
    /// page.set_pixel(1, 5, true);
    /// #
    /// # Ok(()) }
    /// ```
    pub fn create_page<'a>(&self, id: PageId) -> Page<'a> {
        let (x, y) = self.sign_type.dimensions();
        Page::new(id, x, y)
    }

    /// Opens communications with the sign and sends the necessary configuration.
    ///
    /// This must be called first before communicating with the sign. If the sign has already
    /// been configured, it will be reset and its page memory will be cleared.
    ///
    /// # Errors
    ///
    /// Returns:
    /// * [`SignError::Bus`] if the underlying bus failed to process a message.
    /// * [`SignError::UnexpectedResponse`] if the sign did not send the expected response according
    ///   to the protocol. In this case it is recommended to re-[`configure`](Self::configure) the sign and start over.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::cell::RefCell;
    /// # use std::rc::Rc;
    /// # use flipdot::{Address, PageFlipStyle, PageId, Sign, SignType};
    /// # use flipdot_testing::{VirtualSign, VirtualSignBus};
    /// #
    /// # // Placeholder bus for expository purposes
    /// # fn get_bus<'a>() -> Rc<RefCell<VirtualSignBus<'a>>> {
    /// #     Rc::new(RefCell::new(VirtualSignBus::new(vec![VirtualSign::new(Address(3), PageFlipStyle::Manual)])))
    /// # }
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #
    /// let bus = get_bus();
    /// let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
    /// sign.configure()?;
    /// // Sign is now ready to receive pages.
    /// #
    /// # Ok(()) }
    /// ```
    pub fn configure(&self) -> Result<(), SignError> {
        self.ensure_unconfigured()?;

        let config = self.sign_type.to_bytes();
        self.send_data(
            &iter::once(config),
            Operation::ReceiveConfig,
            State::ConfigReceived,
            State::ConfigFailed,
        )
    }

    /// Sends one or more pages of pixel data to the sign.
    ///
    /// Can be called at any time after [`configure`](Self::configure). Replaces any pages that had been previously sent.
    /// Upon return, the first page will be loaded and ready to be shown.
    ///
    /// # Errors
    ///
    /// Returns:
    /// * [`SignError::Bus`] if the underlying bus failed to process a message.
    /// * [`SignError::UnexpectedResponse`] if the sign did not send the expected response according
    ///   to the protocol. In this case it is recommended to re-[`configure`](Self::configure) the sign and start over.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::cell::RefCell;
    /// # use std::rc::Rc;
    /// # use flipdot::{Address, PageFlipStyle, PageId, Sign, SignType};
    /// # use flipdot_testing::{VirtualSign, VirtualSignBus};
    /// #
    /// # // Placeholder bus for expository purposes
    /// # fn get_bus<'a>() -> Rc<RefCell<VirtualSignBus<'a>>> {
    /// #     Rc::new(RefCell::new(VirtualSignBus::new(vec![VirtualSign::new(Address(3), PageFlipStyle::Manual)])))
    /// # }
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #
    /// let bus = get_bus();
    /// let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
    /// sign.configure()?;
    ///
    /// let page = sign.create_page(PageId(1));
    /// if sign.send_pages(&[page])? == PageFlipStyle::Manual {
    ///     // Page has now been loaded but not shown.
    /// } else {
    ///     // Sign is now showing the page automatically.
    /// }
    /// #
    /// # Ok(()) }
    /// ```
    pub fn send_pages<'a, I>(&self, pages: I) -> Result<PageFlipStyle, SignError>
    where
        I: IntoIterator<Item = &'a Page<'a>>,
        <I as IntoIterator>::IntoIter: Clone,
    {
        let data = pages.into_iter().map(Page::as_bytes);
        self.send_data(&data, Operation::ReceivePixels, State::PixelsReceived, State::PixelsFailed)?;

        self.send_message_expect_response(Message::PixelsComplete(self.address), &None)?;

        let response = self.send_message(Message::QueryState(self.address))?;
        match response {
            Some(Message::ReportState(address, state)) if address == self.address && state == State::ShowingPages => {
                Ok(PageFlipStyle::Automatic)
            }
            _ => {
                Ok(PageFlipStyle::Manual)
            }
        }
    }

    /// Loads the next page into memory.
    ///
    /// Once a page has been shown, this is called to prepare the next page to be shown.
    ///
    /// If [`send_pages`](Self::send_pages) returned [`PageFlipStyle::Automatic`], you should not call this function since the sign will show and flip pages itself.
    ///
    /// # Errors
    ///
    /// Returns:
    /// * [`SignError::Bus`] if the underlying bus failed to process a message.
    /// * [`SignError::UnexpectedResponse`] if the sign did not send the expected response according
    ///   to the protocol. In this case it is recommended to re-[`configure`](Self::configure) the sign and start over.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::cell::RefCell;
    /// # use std::rc::Rc;
    /// # use flipdot::{Address, PageFlipStyle, PageId, Sign, SignType};
    /// # use flipdot_testing::{VirtualSign, VirtualSignBus};
    /// #
    /// # // Placeholder bus for expository purposes
    /// # fn get_bus<'a>() -> Rc<RefCell<VirtualSignBus<'a>>> {
    /// #     Rc::new(RefCell::new(VirtualSignBus::new(vec![VirtualSign::new(Address(3), PageFlipStyle::Manual)])))
    /// # }
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #
    /// let bus = get_bus();
    /// let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
    /// sign.configure()?;
    ///
    /// let pages = [sign.create_page(PageId(1)), sign.create_page(PageId(2))];
    /// if sign.send_pages(&pages)? == PageFlipStyle::Manual {
    ///     sign.show_loaded_page()?;
    ///
    ///     sign.load_next_page()?;
    ///     // Page 1 is now shown and page 2 is loaded.
    /// }
    /// #
    /// # Ok(()) }
    /// ```
    pub fn load_next_page(&self) -> Result<(), SignError> {
        self.switch_page(State::PageLoaded, State::PageShown, Operation::LoadNextPage)
    }

    /// Shows the currently loaded page on the display.
    ///
    /// Once a page has been loaded (either via [`send_pages`](Self::send_pages) or [`load_next_page`](Self::load_next_page)), this method will make it visible.
    ///
    /// If [`send_pages`](Self::send_pages) returned [`PageFlipStyle::Automatic`], you should not call this function since the sign will show and flip pages itself.
    ///
    /// # Errors
    ///
    /// Returns:
    /// * [`SignError::Bus`] if the underlying bus failed to process a message.
    /// * [`SignError::UnexpectedResponse`] if the sign did not send the expected response according
    ///   to the protocol. In this case it is recommended to re-[`configure`](Self::configure) the sign and start over.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::cell::RefCell;
    /// # use std::rc::Rc;
    /// # use flipdot::{Address, PageFlipStyle, PageId, Sign, SignType};
    /// # use flipdot_testing::{VirtualSign, VirtualSignBus};
    /// #
    /// # // Placeholder bus for expository purposes
    /// # fn get_bus<'a>() -> Rc<RefCell<VirtualSignBus<'a>>> {
    /// #     Rc::new(RefCell::new(VirtualSignBus::new(vec![VirtualSign::new(Address(3), PageFlipStyle::Manual)])))
    /// # }
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #
    /// let bus = get_bus();
    /// let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
    /// sign.configure()?;
    ///
    /// let page = sign.create_page(PageId(1));
    /// if sign.send_pages(&[page])? == PageFlipStyle::Manual {
    ///     sign.show_loaded_page()?;
    ///     // Page is now shown.
    /// }
    /// #
    /// # Ok(()) }
    /// ```
    pub fn show_loaded_page(&self) -> Result<(), SignError> {
        self.switch_page(State::PageShown, State::PageLoaded, Operation::ShowLoadedPage)
    }

    /// Blanks the display and shuts the sign down.
    ///
    /// The sign will not be usable for 30 seconds after calling this method.
    /// Generally optional as disconnecting switched power from the sign should have the same effect.
    ///
    /// # Errors
    ///
    /// Returns:
    /// * [`SignError::Bus`] if the underlying bus failed to process a message.
    /// * [`SignError::UnexpectedResponse`] if the sign did not send the expected response according
    ///   to the protocol. In this case it is recommended to re-[`configure`](Self::configure) the sign and start over.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::cell::RefCell;
    /// # use std::rc::Rc;
    /// # use flipdot::{Address, PageFlipStyle, PageId, Sign, SignType};
    /// # use flipdot_testing::{VirtualSign, VirtualSignBus};
    /// #
    /// # // Placeholder bus for expository purposes
    /// # fn get_bus<'a>() -> Rc<RefCell<VirtualSignBus<'a>>> {
    /// #     Rc::new(RefCell::new(VirtualSignBus::new(vec![VirtualSign::new(Address(3), PageFlipStyle::Manual)])))
    /// # }
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #
    /// let bus = get_bus();
    /// let sign = Sign::new(bus.clone(), Address(3), SignType::Max3000Side90x7);
    /// sign.configure()?;
    ///
    /// let page = sign.create_page(PageId(1));
    /// if sign.send_pages(&[page])? == PageFlipStyle::Manual {
    ///     sign.show_loaded_page()?;
    /// }
    ///
    /// sign.shut_down()?;
    /// // Sign is now blanked.
    /// #
    /// # Ok(()) }
    /// ```
    pub fn shut_down(&self) -> Result<(), SignError> {
        self.send_message_expect_response(Message::Goodbye(self.address), &None)
    }

    /// Borrows the bus mutably and sends a message.
    ///
    /// Enforces that only leaf calls borrow the bus to avoid runtime errors,
    /// and conveniently localizes the error chaining on failure.
    fn send_message(&self, message: Message<'_>) -> Result<Option<Message<'_>>, SignError> {
        let mut bus = self.bus.borrow_mut();
        Ok(bus.process_message(message)?)
    }

    /// Borrows the bus mutably, sends a message, and verifies that the response is as expected.
    ///
    /// Serves the same purpose as `send_message` when exactly one response is expected.
    fn send_message_expect_response(
        &self,
        message: Message<'_>,
        expected_response: &Option<Message<'_>>,
    ) -> Result<(), SignError> {
        let response = self.send_message(message)?;
        verify_response(expected_response, &response)
    }

    /// Ensures that the sign is in the `Unconfigured` state.
    ///
    /// If it already is, nothing to do. Otherwise start or finish a reset as appropriate.
    /// This ensures that the sign is in a known good state before we begin configuring it.
    fn ensure_unconfigured(&self) -> Result<(), SignError> {
        let response = self.send_message(Message::Hello(self.address))?;
        match response {
            Some(Message::ReportState(address, State::Unconfigured)) if address == self.address => {}

            Some(Message::ReportState(address, State::ReadyToReset)) if address == self.address => {
                self.send_message_expect_response(
                    Message::RequestOperation(self.address, Operation::FinishReset),
                    &Some(Message::AckOperation(self.address, Operation::FinishReset)),
                )?;

                self.send_message_expect_response(
                    Message::Hello(self.address),
                    &Some(Message::ReportState(self.address, State::Unconfigured)),
                )?;
            }

            _ => {
                self.send_message_expect_response(
                    Message::RequestOperation(self.address, Operation::StartReset),
                    &Some(Message::AckOperation(self.address, Operation::StartReset)),
                )?;

                self.send_message_expect_response(
                    Message::Hello(self.address),
                    &Some(Message::ReportState(self.address, State::ReadyToReset)),
                )?;

                self.send_message_expect_response(
                    Message::RequestOperation(self.address, Operation::FinishReset),
                    &Some(Message::AckOperation(self.address, Operation::FinishReset)),
                )?;

                self.send_message_expect_response(
                    Message::Hello(self.address),
                    &Some(Message::ReportState(self.address, State::Unconfigured)),
                )?;
            }
        };
        Ok(())
    }

    /// Sends a chunk of data and verifies proper receipt with retries.
    ///
    /// Requests `operation` from the sign and fails if it does not acknowledge.
    /// Sends `data` in 16-byte chunks, then queries the sign's state.
    /// If `success`, we're done. If `failure`, repeat the process a fixed number
    /// of times in case the data was corrupted in transit. Fails after exhausting
    /// the retries or if any other state is reported.
    fn send_data<'a, I>(&self, data: &I, operation: Operation, success: State, failure: State) -> Result<(), SignError>
    where
        I: Iterator<Item = &'a [u8]> + Clone,
    {
        const MAX_ATTEMPTS: u32 = 3;
        let mut attempts = 1;
        loop {
            self.send_message_expect_response(
                Message::RequestOperation(self.address, operation),
                &Some(Message::AckOperation(self.address, operation)),
            )?;

            let mut chunks_sent = 0;
            for item in data.clone() {
                for (i, chunk) in item.chunks(16).enumerate() {
                    // Safe to unwrap the Data creation as the chunk will obviously always be less than 255 bytes.
                    self.send_message_expect_response(
                        Message::SendData(Offset((i * 16) as u16), Data::try_new(chunk).unwrap()),
                        &None,
                    )?;
                    chunks_sent += 1;
                }
            }

            self.send_message_expect_response(Message::DataChunksSent(ChunkCount(chunks_sent)), &None)?;

            let response = self.send_message(Message::QueryState(self.address))?;
            if response == Some(Message::ReportState(self.address, failure)) && attempts < MAX_ATTEMPTS {
                attempts += 1;
            } else {
                verify_response(&Some(Message::ReportState(self.address, success)), &response)?;
                break;
            }
        }

        Ok(())
    }

    /// Loads or shows a page and waits for the operation to complete.
    ///
    /// Queries the sign's current state. If `target`, we're done. If `trigger`, request `operation`.
    /// Continue looping while the state is `PageLoadInProgress` or `PageShowInProgress`, waiting
    /// to enter `target`. Fails if any other state is reported.
    fn switch_page(&self, target: State, trigger: State, operation: Operation) -> Result<(), SignError> {
        loop {
            let response = self.send_message(Message::QueryState(self.address))?;
            match response {
                Some(Message::ReportState(address, state)) if address == self.address && state == State::ShowingPages => {
                    warn!("Sign flips its own pages automatically; show_loaded_page/load_next_page have no effect.");
                    break;
                }

                Some(Message::ReportState(address, state)) if address == self.address && state == target => {
                    break;
                }

                Some(Message::ReportState(address, state)) if address == self.address && state == trigger => {
                    self.send_message_expect_response(
                        Message::RequestOperation(self.address, operation),
                        &Some(Message::AckOperation(self.address, operation)),
                    )?;
                }

                Some(Message::ReportState(address, State::PageLoadInProgress))
                | Some(Message::ReportState(address, State::PageShowInProgress))
                    if address == self.address => {}

                _ => {
                    return Err(SignError::UnexpectedResponse {
                        expected: format!("Some(ReportState({:?}, Page*))", self.address),
                        actual: format!("{:?}", response),
                    })
                }
            };
        }
        Ok(())
    }
}

/// Fails with an `UnexpectedResponse` error if `response` is not equal to `expected`.
fn verify_response(expected: &Option<Message<'_>>, response: &Option<Message<'_>>) -> Result<(), SignError> {
    if response == expected {
        Ok(())
    } else {
        Err(SignError::UnexpectedResponse {
            expected: format!("{:?}", expected),
            actual: format!("{:?}", response),
        })
    }
}
