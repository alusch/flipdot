use std::fmt::{self, Display, Formatter};

use derive_more::{Display, LowerHex, UpperHex};

use crate::{Address, Data, Frame, MsgType};

/// High-level representation of a sign bus communication message.
///
/// Ascribes meaning to a [`Frame`] and is freely convertible to and from one
/// (with `Unknown` to allow round-tripping unknown message types). This is the
/// primary way that messages are represented in `flipdot`.
///
/// # Examples
///
/// ```
/// use flipdot_core::{Address, Message, SignBus, State};
/// use flipdot_testing::{VirtualSign, VirtualSignBus};
///
/// # fn main() -> Result<(), failure::Error> {
/// #
/// let mut bus = VirtualSignBus::new(vec![VirtualSign::new(Address(3))]);
///
/// // Message is the currency used to send and receive messages on a bus:
/// let response = bus.process_message(Message::QueryState(Address(3)))?;
/// assert_eq!(Some(Message::ReportState(Address(3), State::Unconfigured)), response);
/// #
/// # Ok(()) }
/// ```
///
/// [`Frame`]: struct.Frame.html
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Message<'a> {
    /// Send a chunk of data, with the first member indicating the offset.
    ///
    /// E.g. when sending 32 bytes of data in two 16-byte chunks, the first
    /// message would have offset 0 and the second would have offset 16.
    /// No response is expected.
    SendData(Offset, Data<'a>),
    /// Notifies that we are done sending data, and specifies how many chunks were sent.
    ///
    /// No response is expected.
    DataChunksSent(ChunkCount),

    /// Initially discovers the sign with the given address on the bus.
    ///
    /// A `ReportState` message with the sign's current state is expected.
    Hello(Address),
    /// Queries the sign with the given address for its current state.
    ///
    /// A `ReportState` message with the sign's current state is expected.
    QueryState(Address),
    /// Indicates the state of the sign with the given address.
    ///
    /// Sent by the sign in response to a `Hello` or`QueryState` message.
    ReportState(Address, State),

    /// Requests that the sign with the given address perform an operation.
    ///
    /// An `AckOperation` response is expected.
    RequestOperation(Address, Operation),
    /// Sent by the sign with the given address indicating that it will perform
    /// the given operation.
    AckOperation(Address, Operation),

    /// Indicates that the pixel data for the sign with the given address has been
    /// fully transferred.
    ///
    /// This indicates that the sign should load it into memory in preparation to show.
    /// No response is expected.
    PixelsComplete(Address),

    /// Notifies the sign with the given address to blank its display and shut down.
    ///
    /// The sign will not be usable for 30 seconds after receiving this message.
    /// Generally optional as disconnecting switched power from the sign should
    /// have the same effect. No response is expected.
    Goodbye(Address),

    /// Wraps a [`Frame`] that does not correspond to any known message.
    ///
    /// [`Frame`]: struct.Frame.html
    Unknown(Frame<'a>),

    // Don't actually use this; it's just here to prevent exhaustive matching
    // so we can extend this enum in the future without a breaking change.
    #[doc(hidden)]
    __Nonexhaustive,
}

/// The memory offset for data sent via a [`SendData`] message.
///
/// # Examples
///
/// ```
/// use flipdot_core::{Data, Message, Offset};
///
/// # fn main() -> Result<(), failure::Error> {
/// #
/// let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
/// let message1 = Message::SendData(Offset(0), Data::try_new(&data)?);
/// let message2 = Message::SendData(Offset(16), Data::try_new(&data)?);
/// // These two messages would send a total of 32 bytes, repeating the sequence twice.
/// #
/// # Ok(()) }
/// ```
///
/// [`SendData`]: enum.Message.html#variant.SendData
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Display, LowerHex, UpperHex)]
pub struct Offset(pub u16);

/// The number of chunks sent in [`SendData`] messages, reported by [`DataChunksSent`].
///
/// # Examples
///
/// ```
/// use flipdot_core::{ChunkCount, Message};
///
/// // Assume we just sent three SendData messages. That should be followed with:
/// let message = Message::DataChunksSent(ChunkCount(3));
/// ```
///
/// [`SendData`]: enum.Message.html#variant.SendData
/// [`DataChunksSent`]: enum.Message.html#variant.DataChunksSent
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Display, LowerHex, UpperHex)]
pub struct ChunkCount(pub u16);

/// Possible states that a sign can be in during operation.
///
/// These are reported by the sign in a [`ReportState`] message
/// in response to [`Hello`] or [`QueryState`].
///
/// [`ReportState`]: enum.Message.html#variant.ReportState
/// [`Hello`]: enum.Message.html#variant.Hello
/// [`QueryState`]: enum.Message.html#variant.QueryState
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum State {
    /// The initial state upon power on or after a reset.
    /// No configuration or pixel data stored.
    Unconfigured,
    /// The sign is waiting for the ODK to send the 16-byte configuration data.
    ConfigInProgress,
    /// The configuration data was successfully received.
    ConfigReceived,
    /// Error encountered while reading the configuration data.
    ConfigFailed,
    /// The sign is waiting for the ODK to send the pixel data.
    PixelsInProgress,
    /// Pixel data was successfully received.
    PixelsReceived,
    /// Error encountered while reading the pixel data.
    PixelsFailed,
    /// Page was loaded into memory and is ready to be shown.
    PageLoaded,
    /// Page is in the process of being loaded into memory.
    PageLoadInProgress,
    /// Loaded page was successfully shown.
    PageShown,
    /// Page is in the process of being shown.
    PageShowInProgress,
    /// Sign is ready to reset back to the `Unconfigured` state.
    ReadyToReset,

    // Don't actually use this; it's just here to prevent exhaustive matching
    // so we can extend this enum in the future without a breaking change.
    #[doc(hidden)]
    __Nonexhaustive,
}

/// Operations that can be requested of a sign, which trigger actions and/or state changes.
///
/// These are requested by the ODK via a [`RequestOperation`] message.
///
/// [`RequestOperation`]: enum.Message.html#variant.RequestOperation
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Operation {
    /// Receive the 16-byte configuration data.
    ReceiveConfig,
    /// Receive one or more pages of pixel data.
    ReceivePixels,
    /// Show the page that's currently loaded in memory.
    ShowLoadedPage,
    /// Load the next stored page into memory in preparation to show.
    LoadNextPage,
    /// Begin the process of resetting back to the [`Unconfigured`] state.
    ///
    /// [`Unconfigured`]: enum.State.html#variant.Unconfigured
    StartReset,
    /// Finish the process of resetting back to the [`Unconfigured`] state.
    ///
    /// [`Unconfigured`]: enum.State.html#variant.Unconfigured
    FinishReset,

    // Don't actually use this; it's just here to prevent exhaustive matching
    // so we can extend this enum in the future without a breaking change.
    #[doc(hidden)]
    __Nonexhaustive,
}

impl Display for Message<'_> {
    /// Provides a human-readable view of the message.
    ///
    /// This is useful, for example, when monitoring the traffic on a bus.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Message::SendData(offset, ref data) => {
                write!(f, "SendData [Offset {:04X}] ", offset)?;
                for byte in data.get().iter() {
                    write!(f, "{:02X} ", byte)?;
                }
            }

            Message::DataChunksSent(chunks) => write!(f, "DataChunksSent [{:04X}]", chunks)?,

            Message::Hello(address) => write!(f, "[Addr {:04X}] <-- Hello", address)?,
            Message::QueryState(address) => write!(f, "[Addr {:04X}] <-- QueryState", address)?,
            Message::ReportState(address, state) => write!(f, "[Addr {:04X}] --> ReportState [{:?}]", address, state)?,

            Message::RequestOperation(address, op) => write!(f, "[Addr {:04X}] <-- RequestOperation [{:?}]", address, op)?,
            Message::AckOperation(address, op) => write!(f, "[Addr {:04X}] --> AckOperation [{:?}]", address, op)?,

            Message::PixelsComplete(address) => write!(f, "[Addr {:04X}] <-- PixelsComplete", address)?,

            Message::Goodbye(address) => write!(f, "[Addr {:04X}] <-- Goodbye", address)?,

            Message::Unknown(ref frame) => write!(f, "Unknown {}", frame)?,

            Message::__Nonexhaustive => unreachable!(),
        }

        Ok(())
    }
}

impl<'a> From<Frame<'a>> for Message<'a> {
    /// Converts a [`Frame`] into a `Message`.
    ///
    /// This cannot fail as all valid [`Frame`]s are representable as `Message`s (though perhaps `Unknown`).
    /// The input [`Frame`] is consumed to allow efficiently reusing its data where possible.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_core::{Address, Data, Frame, Message, MsgType, State};
    /// # fn main() -> Result<(), failure::Error> {
    /// #
    /// let frame = Frame::new(Address(0x12), MsgType(4), Data::try_new(vec![0x07])?);
    /// let message = Message::from(frame);
    /// assert_eq!(Message::ReportState(Address(0x12), State::ConfigReceived), message);
    /// #
    /// # Ok(()) }
    /// ```
    ///
    /// [`Frame`]: struct.Frame.html
    fn from(frame: Frame<'a>) -> Self {
        match frame.data().len() {
            0 => match frame.message_type() {
                MsgType(1) => Message::DataChunksSent(ChunkCount(frame.address().0)),
                _ => Message::Unknown(frame),
            },

            1 => match (frame.message_type(), frame.data()[0]) {
                (MsgType(2), 0xFF) => Message::Hello(frame.address()),
                (MsgType(2), 0x00) => Message::QueryState(frame.address()),
                (MsgType(2), 0x55) => Message::Goodbye(frame.address()),

                (MsgType(4), 0x0F) => Message::ReportState(frame.address(), State::Unconfigured),
                (MsgType(4), 0x0D) => Message::ReportState(frame.address(), State::ConfigInProgress),
                (MsgType(4), 0x07) => Message::ReportState(frame.address(), State::ConfigReceived),
                (MsgType(4), 0x0C) => Message::ReportState(frame.address(), State::ConfigFailed),
                (MsgType(4), 0x03) => Message::ReportState(frame.address(), State::PixelsInProgress),
                (MsgType(4), 0x01) => Message::ReportState(frame.address(), State::PixelsReceived),
                (MsgType(4), 0x0B) => Message::ReportState(frame.address(), State::PixelsFailed),
                (MsgType(4), 0x10) => Message::ReportState(frame.address(), State::PageLoaded),
                (MsgType(4), 0x13) => Message::ReportState(frame.address(), State::PageLoadInProgress),
                (MsgType(4), 0x12) => Message::ReportState(frame.address(), State::PageShown),
                (MsgType(4), 0x11) => Message::ReportState(frame.address(), State::PageShowInProgress),
                (MsgType(4), 0x08) => Message::ReportState(frame.address(), State::ReadyToReset),

                (MsgType(3), 0xA1) => Message::RequestOperation(frame.address(), Operation::ReceiveConfig),
                (MsgType(3), 0xA2) => Message::RequestOperation(frame.address(), Operation::ReceivePixels),
                (MsgType(3), 0xA9) => Message::RequestOperation(frame.address(), Operation::ShowLoadedPage),
                (MsgType(3), 0xAA) => Message::RequestOperation(frame.address(), Operation::LoadNextPage),
                (MsgType(3), 0xA6) => Message::RequestOperation(frame.address(), Operation::StartReset),
                (MsgType(3), 0xA7) => Message::RequestOperation(frame.address(), Operation::FinishReset),

                (MsgType(5), 0x95) => Message::AckOperation(frame.address(), Operation::ReceiveConfig),
                (MsgType(5), 0x91) => Message::AckOperation(frame.address(), Operation::ReceivePixels),
                (MsgType(5), 0x96) => Message::AckOperation(frame.address(), Operation::ShowLoadedPage),
                (MsgType(5), 0x97) => Message::AckOperation(frame.address(), Operation::LoadNextPage),
                (MsgType(5), 0x93) => Message::AckOperation(frame.address(), Operation::StartReset),
                (MsgType(5), 0x94) => Message::AckOperation(frame.address(), Operation::FinishReset),

                (MsgType(6), 0x00) => Message::PixelsComplete(frame.address()),

                (_, _) => Message::Unknown(frame),
            },

            _ => match frame.message_type() {
                MsgType(0) => Message::SendData(Offset(frame.address().0), frame.into_data()),
                _ => Message::Unknown(frame),
            },
        }
    }
}

impl<'a> From<Message<'a>> for Frame<'a> {
    /// Converts a [`Message`] into a `Frame`.
    ///
    /// This cannot fail as all `Message`s can be represented as `Frame`s.
    /// The input [`Message`] is consumed to allow efficiently reusing its data where possible.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_core::{Address, Data, Frame, Message, MsgType, State};
    /// # fn main() -> Result<(), failure::Error> {
    /// #
    /// let message = Message::ReportState(Address(0xFF), State::ConfigReceived);
    /// let frame = Frame::from(message);
    /// assert_eq!(Frame::new(Address(0xFF), MsgType(4), Data::try_new(vec![0x07])?), frame);
    /// #
    /// # Ok(()) }
    /// ```
    ///
    /// [`Message`]: enum.Message.html
    fn from(message: Message<'a>) -> Self {
        match message {
            Message::SendData(Offset(offset), data) => Frame::new(Address(offset), MsgType(0), data),

            Message::DataChunksSent(ChunkCount(chunks)) => Frame::new(Address(chunks), MsgType(1), Data::from(&[])),

            Message::Hello(address) => Frame::new(address, MsgType(2), Data::from(&[0xFF])),
            Message::Goodbye(address) => Frame::new(address, MsgType(2), Data::from(&[0x55])),
            Message::QueryState(address) => Frame::new(address, MsgType(2), Data::from(&[0x00])),

            Message::ReportState(address, State::Unconfigured) => Frame::new(address, MsgType(4), Data::from(&[0x0F])),
            Message::ReportState(address, State::ConfigInProgress) => Frame::new(address, MsgType(4), Data::from(&[0x0D])),
            Message::ReportState(address, State::ConfigReceived) => Frame::new(address, MsgType(4), Data::from(&[0x07])),
            Message::ReportState(address, State::ConfigFailed) => Frame::new(address, MsgType(4), Data::from(&[0x0C])),
            Message::ReportState(address, State::PixelsInProgress) => Frame::new(address, MsgType(4), Data::from(&[0x03])),
            Message::ReportState(address, State::PixelsReceived) => Frame::new(address, MsgType(4), Data::from(&[0x01])),
            Message::ReportState(address, State::PixelsFailed) => Frame::new(address, MsgType(4), Data::from(&[0x0B])),
            Message::ReportState(address, State::PageLoaded) => Frame::new(address, MsgType(4), Data::from(&[0x10])),
            Message::ReportState(address, State::PageLoadInProgress) => Frame::new(address, MsgType(4), Data::from(&[0x13])),
            Message::ReportState(address, State::PageShown) => Frame::new(address, MsgType(4), Data::from(&[0x12])),
            Message::ReportState(address, State::PageShowInProgress) => Frame::new(address, MsgType(4), Data::from(&[0x11])),
            Message::ReportState(address, State::ReadyToReset) => Frame::new(address, MsgType(4), Data::from(&[0x08])),

            Message::RequestOperation(address, Operation::ReceiveConfig) => Frame::new(address, MsgType(3), Data::from(&[0xA1])),
            Message::RequestOperation(address, Operation::ReceivePixels) => Frame::new(address, MsgType(3), Data::from(&[0xA2])),
            Message::RequestOperation(address, Operation::ShowLoadedPage) => Frame::new(address, MsgType(3), Data::from(&[0xA9])),
            Message::RequestOperation(address, Operation::LoadNextPage) => Frame::new(address, MsgType(3), Data::from(&[0xAA])),
            Message::RequestOperation(address, Operation::StartReset) => Frame::new(address, MsgType(3), Data::from(&[0xA6])),
            Message::RequestOperation(address, Operation::FinishReset) => Frame::new(address, MsgType(3), Data::from(&[0xA7])),

            Message::AckOperation(address, Operation::ReceiveConfig) => Frame::new(address, MsgType(5), Data::from(&[0x95])),
            Message::AckOperation(address, Operation::ReceivePixels) => Frame::new(address, MsgType(5), Data::from(&[0x91])),
            Message::AckOperation(address, Operation::ShowLoadedPage) => Frame::new(address, MsgType(5), Data::from(&[0x96])),
            Message::AckOperation(address, Operation::LoadNextPage) => Frame::new(address, MsgType(5), Data::from(&[0x97])),
            Message::AckOperation(address, Operation::StartReset) => Frame::new(address, MsgType(5), Data::from(&[0x93])),
            Message::AckOperation(address, Operation::FinishReset) => Frame::new(address, MsgType(5), Data::from(&[0x94])),

            Message::PixelsComplete(address) => Frame::new(address, MsgType(6), Data::from(&[0x00])),

            Message::Unknown(frame) => frame,

            Message::__Nonexhaustive
            | Message::ReportState(_, State::__Nonexhaustive)
            | Message::RequestOperation(_, Operation::__Nonexhaustive)
            | Message::AckOperation(_, Operation::__Nonexhaustive) => unreachable!(),
        }
    }
}

/// Creates a new [`Data`] wrapper around a static slice of `u8`.
///
/// Provides a convenient shorthand for creating small pieces of data
/// that obviously won't reach the length limit and thus can safely be unwrapped.
///
/// # Panics
///
/// Panics if the length of the data is greater than 255.
///
/// [`Data`]: struct.Data.html
// fn Data::from(data: &'static [u8]) -> Data {
//     Data::try_new(data).unwrap()
// }

#[cfg(test)]
mod tests {
    use super::*;

    fn verify_roundtrip(frame: Frame<'_>, expected_message: Message<'_>) {
        let orig_frame = frame.clone();

        let converted_message = Message::from(frame);
        assert_eq!(expected_message, converted_message);

        let converted_frame = Frame::from(converted_message);
        assert_eq!(orig_frame, converted_frame);
    }

    #[test]
    fn frame_message_roundtrip() {
        verify_roundtrip(
            Frame::new(Address(16), MsgType(0), Data::from(&[0x00, 0x15, 0x51, 0xF7])),
            Message::SendData(Offset(16), Data::from(&[0x00, 0x15, 0x51, 0xF7])),
        );

        verify_roundtrip(
            Frame::new(Address(13), MsgType(1), Data::from(&[])),
            Message::DataChunksSent(ChunkCount(13)),
        );

        verify_roundtrip(
            Frame::new(Address(0x7F), MsgType(2), Data::from(&[0xFF])),
            Message::Hello(Address(0x7F)),
        );
        verify_roundtrip(
            Frame::new(Address(0x11), MsgType(2), Data::from(&[0x55])),
            Message::Goodbye(Address(0x11)),
        );
        verify_roundtrip(
            Frame::new(Address(0xFF), MsgType(2), Data::from(&[0x00])),
            Message::QueryState(Address(0xFF)),
        );

        verify_roundtrip(
            Frame::new(Address(0x01), MsgType(4), Data::from(&[0x0F])),
            Message::ReportState(Address(0x01), State::Unconfigured),
        );

        verify_roundtrip(
            Frame::new(Address(0x00), MsgType(3), Data::from(&[0xA1])),
            Message::RequestOperation(Address(0x00), Operation::ReceiveConfig),
        );
        verify_roundtrip(
            Frame::new(Address(0x01), MsgType(3), Data::from(&[0xA2])),
            Message::RequestOperation(Address(0x01), Operation::ReceivePixels),
        );
        verify_roundtrip(
            Frame::new(Address(0x11), MsgType(3), Data::from(&[0xA9])),
            Message::RequestOperation(Address(0x11), Operation::ShowLoadedPage),
        );
        verify_roundtrip(
            Frame::new(Address(0x02), MsgType(3), Data::from(&[0xAA])),
            Message::RequestOperation(Address(0x02), Operation::LoadNextPage),
        );
        verify_roundtrip(
            Frame::new(Address(0x22), MsgType(3), Data::from(&[0xA6])),
            Message::RequestOperation(Address(0x22), Operation::StartReset),
        );
        verify_roundtrip(
            Frame::new(Address(0x03), MsgType(3), Data::from(&[0xA7])),
            Message::RequestOperation(Address(0x03), Operation::FinishReset),
        );

        verify_roundtrip(
            Frame::new(Address(0xFF), MsgType(4), Data::from(&[0x0F])),
            Message::ReportState(Address(0xFF), State::Unconfigured),
        );
        verify_roundtrip(
            Frame::new(Address(0x91), MsgType(4), Data::from(&[0x0D])),
            Message::ReportState(Address(0x91), State::ConfigInProgress),
        );
        verify_roundtrip(
            Frame::new(Address(0xDC), MsgType(4), Data::from(&[0x07])),
            Message::ReportState(Address(0xDC), State::ConfigReceived),
        );
        verify_roundtrip(
            Frame::new(Address(0xA1), MsgType(4), Data::from(&[0x0C])),
            Message::ReportState(Address(0xA1), State::ConfigFailed),
        );
        verify_roundtrip(
            Frame::new(Address(0xF7), MsgType(4), Data::from(&[0x03])),
            Message::ReportState(Address(0xF7), State::PixelsInProgress),
        );
        verify_roundtrip(
            Frame::new(Address(0x0F), MsgType(4), Data::from(&[0x01])),
            Message::ReportState(Address(0x0F), State::PixelsReceived),
        );
        verify_roundtrip(
            Frame::new(Address(0x37), MsgType(4), Data::from(&[0x0B])),
            Message::ReportState(Address(0x37), State::PixelsFailed),
        );
        verify_roundtrip(
            Frame::new(Address(0x42), MsgType(4), Data::from(&[0x10])),
            Message::ReportState(Address(0x42), State::PageLoaded),
        );
        verify_roundtrip(
            Frame::new(Address(0x68), MsgType(4), Data::from(&[0x13])),
            Message::ReportState(Address(0x68), State::PageLoadInProgress),
        );
        verify_roundtrip(
            Frame::new(Address(0x1C), MsgType(4), Data::from(&[0x12])),
            Message::ReportState(Address(0x1C), State::PageShown),
        );
        verify_roundtrip(
            Frame::new(Address(0x9D), MsgType(4), Data::from(&[0x11])),
            Message::ReportState(Address(0x9D), State::PageShowInProgress),
        );
        verify_roundtrip(
            Frame::new(Address(0x87), MsgType(4), Data::from(&[0x08])),
            Message::ReportState(Address(0x87), State::ReadyToReset),
        );

        verify_roundtrip(
            Frame::new(Address(0xABCD), MsgType(5), Data::from(&[0x95])),
            Message::AckOperation(Address(0xABCD), Operation::ReceiveConfig),
        );
        verify_roundtrip(
            Frame::new(Address(0xFF00), MsgType(5), Data::from(&[0x91])),
            Message::AckOperation(Address(0xFF00), Operation::ReceivePixels),
        );
        verify_roundtrip(
            Frame::new(Address(0x0C0F), MsgType(5), Data::from(&[0x96])),
            Message::AckOperation(Address(0x0C0F), Operation::ShowLoadedPage),
        );
        verify_roundtrip(
            Frame::new(Address(0x11DD), MsgType(5), Data::from(&[0x97])),
            Message::AckOperation(Address(0x11DD), Operation::LoadNextPage),
        );
        verify_roundtrip(
            Frame::new(Address(0x1337), MsgType(5), Data::from(&[0x93])),
            Message::AckOperation(Address(0x1337), Operation::StartReset),
        );
        verify_roundtrip(
            Frame::new(Address(0x1987), MsgType(5), Data::from(&[0x94])),
            Message::AckOperation(Address(0x1987), Operation::FinishReset),
        );

        verify_roundtrip(
            Frame::new(Address(0xFFFF), MsgType(6), Data::from(&[0x00])),
            Message::PixelsComplete(Address(0xFFFF)),
        );

        verify_roundtrip(
            Frame::new(Address(0xF00D), MsgType(99), Data::from(&[])),
            Message::Unknown(Frame::new(Address(0xF00D), MsgType(99), Data::from(&[]))),
        );

        verify_roundtrip(
            Frame::new(Address(0xBEEF), MsgType(255), Data::from(&[0xAA])),
            Message::Unknown(Frame::new(Address(0xBEEF), MsgType(255), Data::from(&[0xAA]))),
        );

        verify_roundtrip(
            Frame::new(Address(0xABAB), MsgType(17), Data::from(&[0x7A, 0x1C])),
            Message::Unknown(Frame::new(Address(0xABAB), MsgType(17), Data::from(&[0x7A, 0x1C]))),
        );
    }

    #[test]
    fn display() {
        let message = Message::SendData(Offset(0x10), Data::from(&[0x20, 0xFF]));
        let display = format!("{}", message);
        assert_eq!("SendData [Offset 0010] 20 FF", display.trim());

        let message = Message::DataChunksSent(ChunkCount(3));
        let display = format!("{}", message);
        assert_eq!("DataChunksSent [0003]", display);

        let message = Message::Hello(Address(0x7F));
        let display = format!("{}", message);
        assert_eq!("[Addr 007F] <-- Hello", display);

        let message = Message::QueryState(Address(5));
        let display = format!("{}", message);
        assert_eq!("[Addr 0005] <-- QueryState", display);

        let message = Message::ReportState(Address(7), State::Unconfigured);
        let display = format!("{}", message);
        assert_eq!("[Addr 0007] --> ReportState [Unconfigured]", display);

        let message = Message::RequestOperation(Address(16), Operation::ReceivePixels);
        let display = format!("{}", message);
        assert_eq!("[Addr 0010] <-- RequestOperation [ReceivePixels]", display);

        let message = Message::AckOperation(Address(17), Operation::FinishReset);
        let display = format!("{}", message);
        assert_eq!("[Addr 0011] --> AckOperation [FinishReset]", display);

        let message = Message::PixelsComplete(Address(32));
        let display = format!("{}", message);
        assert_eq!("[Addr 0020] <-- PixelsComplete", display);

        let message = Message::Goodbye(Address(1));
        let display = format!("{}", message);
        assert_eq!("[Addr 0001] <-- Goodbye", display);

        let message = Message::Unknown(Frame::new(Address(1), MsgType(2), Data::from(&[])));
        let display = format!("{}", message);
        assert_eq!("Unknown Type 02 | Addr 0001", display);
    }
}
