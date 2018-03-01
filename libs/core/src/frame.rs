use std::borrow::Cow;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{BufRead, BufReader, Read, Write};
use std::str;

use num_traits::Num;
use regex::bytes::Regex;

use errors::{self, ErrorKind};

/// A low-level representation of an Intel HEX data frame.
///
/// The Luminator protocol uses the [Intel HEX] format but not its semantics.
/// This struct handles parsing the raw bytes into a form we can reason about,
/// dealing with checksums, and so forth. It makes no attempt to ascribe meaning
/// to the address, message type, and data (that's [`Message`]'s job).
///
/// Both owned and borrowed data are supported.
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// use flipdot_core::{Address, Data, Frame, MsgType};
///
/// # fn try_main() -> Result<(), Box<Error>> {
/// #
/// let frame = Frame::new(Address(2), MsgType(1), Data::new(vec![3, 31])?);
/// println!("Parsed frame is {}", frame);
///
/// let bytes = frame.to_bytes();
/// assert_eq!(b":02000201031FD9", bytes.as_slice());
///
/// let parsed = Frame::from_bytes(&bytes)?;
/// assert_eq!(parsed, frame);
/// #
/// # Ok(()) }
/// # fn main() { try_main().unwrap(); }
/// ```
///
/// # Format Details
///
/// The format consists of a leading colon, several numeric fields (two-character ASCII representations
/// of hex bytes), and a final carriage return/linefeed terminator. Note that for convenience,
/// `Frame` allows omitting the final CRLF sequence.
///
/// ```text
/// ┌────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬ ┄ ┬────┬────┬────┬────┬────┬────┐
/// │ :  │ DataLen │      Address      │ MsgType │  Data 0 │...│  Data N │  Chksum │ \r │ \n │
/// └────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴ ┄ ┴────┴────┴────┴────┴────┴────┘
///           └╌╌╌╌╌╌╌╌╌╌╌╌╌ # of ╌╌╌╌╌╌╌╌╌╌╌╌╌> ┆       Data bytes      ┆
/// ```
///
/// The `DataLen` field describes how many two-character data byte sequences are present.
/// Note that since it is represented as a single byte, the data length cannot exceed 255 (`0xFF`).
/// If `DataLen` is 0, there are no data bytes, and `MsgType` is followed directly by `Chksum`.
/// The checksum is a [longitudinal redundancy check] calculated on all numeric fields.
///
/// [Intel HEX]: https://en.wikipedia.org/wiki/Intel_HEX
/// [`Message`]: enum.Message.html
/// [longitudinal redundancy check]: https://en.wikipedia.org/wiki/Longitudinal_redundancy_check
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Frame<'a> {
    address: Address,
    message_type: MsgType,
    data: Data<'a>,
}

macro_attr! {
    /// A [`Frame`]'s message type.
    ///
    /// Carries no implicit meaning, but is interpreted by [`Message`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// use flipdot_core::{Address, Data, Frame, MsgType};
    ///
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// // Create a frame with message type 1.
    /// let frame = Frame::new(Address(2), MsgType(1), Data::new(vec![1, 2])?);
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    ///
    /// [`Frame`]: struct.Frame.html
    /// [`Message`]: enum.Message.html
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, NewtypeDisplay!, NewtypeLowerHex!, NewtypeUpperHex!)]
    pub struct MsgType(pub u8);
}

macro_attr! {
    /// The address of a sign, used to identify it on the bus.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// use flipdot_core::{Address, Data, Frame, MsgType};
    ///
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// // Create a frame addressed to sign 2.
    /// let frame = Frame::new(Address(2), MsgType(1), Data::new(vec![1, 2])?);
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    #[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, NewtypeDisplay!, NewtypeLowerHex!, NewtypeUpperHex!)]
    pub struct Address(pub u16);
}

impl<'a> Frame<'a> {
    /// Constructs a new `Frame` with the specified address, message type, and data.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # use flipdot_core::{Address, Data, Frame, MsgType};
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// // some_data is moved into owning_frame.
    /// let some_data = vec![1, 2, 3];
    /// let owning_frame = Frame::new(Address(0xB), MsgType(0xA), Data::new(some_data)?);
    ///
    /// // other_data is borrowed.
    /// let other_data = vec![1, 2, 3];
    /// let borrowing_frame = Frame::new(Address(0xD), MsgType(0xC), Data::new(other_data.as_slice())?);
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    pub fn new(address: Address, message_type: MsgType, data: Data<'a>) -> Self {
        Frame {
            address,
            message_type,
            data,
        }
    }

    /// Returns the message type of the frame.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # use flipdot_core::{Address, Data, Frame, MsgType};
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// let frame = Frame::new(Address(1), MsgType(1), Data::new(vec![])?);
    /// match frame.message_type() {
    ///    MsgType(1) => println!("Message 1"),
    ///    _ => println!("Something else"),
    /// }
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    pub fn message_type(&self) -> MsgType {
        self.message_type
    }

    /// Returns the address of the frame.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # use flipdot_core::{Address, Data, Frame, MsgType};
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// let frame = Frame::new(Address(1), MsgType(1), Data::new(vec![])?);
    /// if frame.address() == Address(3) {
    ///     println!("This frame is addressed to me!");
    /// }
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    pub fn address(&self) -> Address {
        self.address
    }

    /// Returns a reference to the frame's data.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # use flipdot_core::{Address, Data, Frame, MsgType};
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// let frame = Frame::new(Address(1), MsgType(1), Data::new(vec![10, 20])?);
    /// if (frame.data().as_ref() == &[10, 20]) {
    ///     println!("Found the expected data!");
    /// }
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    pub fn data(&self) -> &Cow<[u8]> {
        &self.data.0
    }

    /// Consumes the frame and returns ownership of its data.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # use flipdot_core::{Address, Data, Frame, MsgType};
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// let frame = Frame::new(Address(1), MsgType(1), Data::new(vec![6, 7])?);
    /// let frame2 = Frame::new(Address(2), MsgType(2), frame.into_data());
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    pub fn into_data(self) -> Data<'a> {
        self.data
    }

    /// Converts the frame to its wire format, *without* trailing carriage return/linefeed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # use flipdot_core::{Address, Data, Frame, MsgType};
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// let frame = Frame::new(Address(2), MsgType(1), Data::new(vec![3, 31])?);
    /// let bytes = frame.to_bytes();
    /// assert_eq!(b":02000201031FD9", bytes.as_slice());
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    pub fn to_bytes(&self) -> Vec<u8> {
        const HEX_DIGITS: &[u8] = b"0123456789ABCDEF";

        let mut payload = self.payload();
        let checksum = checksum(&payload);
        payload.push(checksum);
        let payload = payload;

        // Colon, 2 ASCII digits for each byte, and 2 bytes for optional CRLF sequence
        let mut output = Vec::<u8>::with_capacity(1 + 2 * payload.len() + 2);
        output.push(b':');
        for byte in &payload {
            output.push(HEX_DIGITS[(byte >> 4) as usize]);
            output.push(HEX_DIGITS[(byte & 0x0F) as usize]);
        }
        assert_eq!(output.len(), output.capacity() - 2);
        output
    }

    /// Converts the frame to its wire format, including trailing carriage return/linefeed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # use flipdot_core::{Address, Data, Frame, MsgType};
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// let frame = Frame::new(Address(2), MsgType(1), Data::new(vec![3, 31])?);
    /// let bytes = frame.to_bytes_with_newline();
    /// assert_eq!(b":02000201031FD9\r\n", bytes.as_slice());
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    pub fn to_bytes_with_newline(&self) -> Vec<u8> {
        let mut output = self.to_bytes();
        output.extend_from_slice(b"\r\n");
        assert_eq!(output.len(), output.capacity());
        output
    }

    /// Parses the Intel HEX wire format into a new `Frame`.
    ///
    /// # Errors
    ///
    /// Returns [`Error`]`(`[`ErrorKind::Parse`]`, _)` if the data does not conform to the
    /// Intel HEX format or if the actual number of data bytes does not match the specified amount.
    ///
    /// Returns [`Error`]`(`[`ErrorKind::Checksum`]`, _)` if the computed checksum on the data
    /// does not match the specified one.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # use flipdot_core::{Address, Data, Frame, MsgType};
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// let bytes = b":02000201031FD9\r\n";
    /// let frame = Frame::from_bytes(&bytes[..])?;
    /// assert_eq!(Frame::new(Address(2), MsgType(1), Data::new(vec![3, 31])?), frame);
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    ///
    /// [`Error`]: struct.Error.html
    /// [`ErrorKind::Parse`]: enum.ErrorKind.html
    /// [`ErrorKind::Checksum`]: enum.ErrorKind.html
    pub fn from_bytes(bytes: &[u8]) -> errors::Result<Self> {
        lazy_static! {
            static ref RE: Regex = Regex::new(r"(?x)
                ^:                                  # Colon marks beginning of frame
                (?P<data_len>[[:xdigit:]]{2})       # 2 hex digits for data length
                (?P<address>[[:xdigit:]]{4})        # 4 hex digits for address
                (?P<message_type>[[:xdigit:]]{2})   # 2 hex digits for message type
                (?P<data>(?:[[:xdigit:]]{2})*)      # Zero or more groups of 2 hex digits for data
                (?P<checksum>[[:xdigit:]]{2})       # 2 hex digits for checksum
                (?:\r\n)?$                          # Optional newline sequence
            ").unwrap(); // Regex is valid so safe to unwrap.
        }
        let captures = RE
            .captures(bytes)
            .ok_or_else(|| ErrorKind::Parse(bytes.to_owned(), "Invalid format".to_owned()))?;

        // Regex always matches all capture groups so safe to unwrap.
        let data_len = parse_hex::<u8>(captures.name("data_len").unwrap().as_bytes());
        let address = parse_hex::<u16>(captures.name("address").unwrap().as_bytes());
        let message_type = parse_hex::<u8>(captures.name("message_type").unwrap().as_bytes());
        let data_bytes = captures.name("data").unwrap().as_bytes();
        let provided_checksum = parse_hex::<u8>(captures.name("checksum").unwrap().as_bytes());

        let data = data_bytes.chunks(2).map(parse_hex::<u8>).collect::<Vec<_>>();
        if data.len() != data_len as usize {
            bail!(ErrorKind::Parse(
                bytes.to_owned(),
                format!("Expected data length {}, got {}", data_len, data.len())
            ));
        }

        let frame = Frame::new(Address(address), MsgType(message_type), Data::new(data)?);
        let payload = frame.payload();
        let computed_checksum = checksum(&payload);

        if computed_checksum != provided_checksum {
            bail!(ErrorKind::Checksum(bytes.to_owned(), provided_checksum, computed_checksum));
        }

        Ok(frame)
    }

    /// Writes the byte representation (including CRLF) of the frame to a writer.
    ///
    /// # Errors
    /// Returns [`Error`]`(`[`ErrorKind::Io`]`, _)` if the write fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// extern crate serial;
    /// # extern crate flipdot_core;
    /// # use std::error::Error;
    /// # use flipdot_core::{Address, Data, Frame, MsgType};
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// let mut port = serial::open("COM3")?;
    /// let frame = Frame::new(Address(2), MsgType(1), Data::new(vec![3, 31])?);
    /// frame.write(&mut port)?;
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    ///
    /// [`Error`]: struct.Error.html
    /// [`ErrorKind::Io`]: enum.ErrorKind.html
    pub fn write<W: Write>(&self, writer: &mut W) -> errors::Result<()> {
        writer.write_all(&self.to_bytes_with_newline())?;
        Ok(())
    }

    /// Reads the next line (up to `\n`) from the reader and converts the result
    /// into a new `Frame`.
    ///
    /// # Errors
    /// Returns [`Error`]`(`[`ErrorKind::Io`]`, _)` if the read fails.
    ///
    /// Returns [`Error`]`(`[`ErrorKind::Parse`]`, _)` if the data does not conform to the
    /// Intel HEX format or if the actual number of data bytes does not match the specified amount.
    ///
    /// Returns [`Error`]`(`[`ErrorKind::Checksum`]`, _)` if the computed checksum on the data
    /// does not match the specified one.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// extern crate serial;
    /// # extern crate flipdot_core;
    /// # use std::error::Error;
    /// # use flipdot_core::{Address, Data, Frame, MsgType};
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// let mut port = serial::open("COM3")?;
    /// let frame = Frame::read(&mut port)?;
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    ///
    /// [`Error`]: struct.Error.html
    /// [`ErrorKind::Io`]: enum.ErrorKind.html
    /// [`ErrorKind::Parse`]: enum.ErrorKind.html
    /// [`ErrorKind::Checksum`]: enum.ErrorKind.html
    pub fn read<R: Read>(mut reader: &mut R) -> errors::Result<Self> {
        // One-byte buffer seems to work best with such small payloads
        let mut buf_reader = BufReader::with_capacity(1, &mut reader);
        let mut data = Vec::<u8>::new();
        let _ = buf_reader.read_until(b'\n', &mut data)?;
        let frame = Frame::from_bytes(&data)?;
        Ok(frame)
    }

    /// Returns the payload portion of the wire format.
    ///
    /// These are the numeric fields other than the checksum, upon which the checksum is computed.
    fn payload(&self) -> Vec<u8> {
        // Reserving an extra byte here so the checksum can be appended in to_bytes.
        let mut payload = Vec::<u8>::with_capacity(5 + self.data.0.len());
        payload.push(self.data.0.len() as u8);
        payload.push((self.address.0 >> 8) as u8);
        payload.push(self.address.0 as u8);
        payload.push(self.message_type.0);
        payload.extend_from_slice(&self.data.0);
        assert_eq!(payload.len(), payload.capacity() - 1);
        payload
    }
}

impl<'a> Display for Frame<'a> {
    /// Formats the frame in a human-readable way.
    ///
    /// Useful for viewing traffic on a bus. All numbers are in hex.
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Type {:02X} | Addr {:04X}", self.message_type.0, self.address.0)?;
        if self.data.0.len() > 0 {
            write!(f, " | Data ")?;
            for byte in self.data.0.iter() {
                write!(f, "{:02X} ", byte)?;
            }
        }
        Ok(())
    }
}

/// Parses a byte slice representing ASCII text into a hex digit.
///
/// Assumes that the data has already been validated and panics if it is invalid.
fn parse_hex<T: Num>(bytes: &[u8]) -> T
where
    <T as Num>::FromStrRadixErr: 'static + Error,
{
    // Regex already determined these are valid hex digits, so we can just unwrap.
    let string = str::from_utf8(bytes).unwrap();
    T::from_str_radix(string, 16).unwrap()
}

/// Computes the LRC of the given byte slice.
///
/// The canonical implementation is a wrapping add followed by the two's
/// complement (negation). Instead, we can just do a wrapping subtract
/// from zero.
fn checksum(bytes: &[u8]) -> u8 {
    bytes.iter().fold(0, |acc, &b| acc.wrapping_sub(b))
}

/// Owned or borrowed data to be placed in a [`Frame`].
///
/// Since the data length in the [`Frame`] will be represented as a single byte,
/// that length cannot exceed 255 (`0xFF`). `Data` is responsible for maintaining
/// this invariant.
///
/// # Examples
///
/// ```
/// # use std::error::Error;
/// use flipdot_core::{Address, Data, Frame, MsgType};
/// # fn try_main() -> Result<(), Box<Error>> {
/// #
/// let data = Data::new(vec![1, 2, 3])?; // Ok since length under 255
/// let frame = Frame::new(Address(2), MsgType(1), data);
/// #
/// # Ok(()) }
/// # fn main() { try_main().unwrap(); }
/// ```
///
/// [`Frame`]: struct.Frame.html
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Data<'a>(Cow<'a, [u8]>);

impl<'a> Data<'a> {
    /// Creates a new `Data` containing owned or borrowed data.
    ///
    /// Since the data length in the [`Frame`] will be represented as a single byte,
    /// that length cannot exceed 255 (`0xFF`).
    ///
    /// # Errors
    ///
    /// Returns [`Error`]`(`[`ErrorKind::Argument`]`, _)` if the data length is greater than 255 (`0xFF`).
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// use flipdot_core::Data;
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// let data = Data::new(vec![1, 2, 3])?;
    /// assert_eq!(vec![1, 2, 3], data.get().as_ref());
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    ///
    /// Borrowed data can also be used:
    ///
    /// ```
    /// # use std::error::Error;
    /// # use flipdot_core::Data;
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// let bytes = vec![1, 2, 3];
    /// let data = Data::new(bytes.as_ref())?;
    /// assert_eq!(vec![1, 2, 3], data.get().as_ref());
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    ///
    /// This will fail since the passed-in vector is too large:
    ///
    /// ```
    /// # use flipdot_core::Data;
    /// let result = Data::new(vec![0; 1000]);
    /// assert!(result.is_err());
    /// ```
    ///
    /// [`Frame`]: struct.Frame.html
    /// [`Error`]: struct.Error.html
    /// [`ErrorKind::Argument`]: enum.ErrorKind.html
    pub fn new<T: Into<Cow<'a, [u8]>>>(data: T) -> errors::Result<Self> {
        let data: Cow<'a, [u8]> = data.into();
        if data.len() > 0xFF {
            bail!(ErrorKind::Argument(format!(
                "Frame data length must fit in a single byte (at most 255), got {}",
                data.len()
            )));
        }
        Ok(Data(data))
    }

    /// Returns a reference to the inner [`Cow`]`<[u8]>`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::error::Error;
    /// # use flipdot_core::Data;
    /// # fn try_main() -> Result<(), Box<Error>> {
    /// #
    /// let data = Data::new(vec![])?;
    /// assert!(data.get().is_empty());
    /// #
    /// # Ok(()) }
    /// # fn main() { try_main().unwrap(); }
    /// ```
    ///
    /// [`Cow`]: https://doc.rust-lang.org/std/borrow/enum.Cow.html
    pub fn get(&self) -> &Cow<'a, [u8]> {
        &self.0
    }
}

// Data is mostly used with small static arrays that obviously fit in the 255-byte limit,
// so create some From impls that make that case simple. We unfortunately can't be generic
// over integers yet, so use a macro to implement for common array lengths.
macro_rules! impl_from_array_ref_with_length {
    ($length:expr) => {
        impl<'a> From<&'static [u8; $length]> for Data<'a> {
            fn from(value: &'static [u8; $length]) -> Data<'a> {
                Data::new(&value[..]).unwrap()
            }
        }
    };
}

impl_from_array_ref_with_length!(0);
impl_from_array_ref_with_length!(1);
impl_from_array_ref_with_length!(2);
impl_from_array_ref_with_length!(3);
impl_from_array_ref_with_length!(4);

#[cfg(test)]
mod tests {
    use super::*;
    use errors::*;

    #[test]
    fn roundtrip_simple_frame() {
        let frame = Frame::new(Address(0x7F), MsgType(0x02), Data::from(&[0xFF]));

        let encoded = frame.to_bytes();
        let decoded = Frame::from_bytes(&encoded).unwrap();

        assert_eq!(b":01007F02FF7F", encoded.as_slice());
        assert_eq!(frame, decoded);
    }

    #[test]
    fn roundtrip_complex_frame() {
        let data = Data::new(vec![
            0x01, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7F, 0x7F, 0x06, 0x0C, 0x18, 0x7F, 0x7F, 0x00,
        ]).unwrap();
        let frame = Frame::new(Address(0x00), MsgType(0x00), data);

        let encoded = frame.to_bytes();
        let decoded = Frame::from_bytes(&encoded).unwrap();

        assert_eq!(&b":1000000001100000000000007F7F060C187F7F00B9"[..], encoded.as_slice());
        assert_eq!(frame, decoded);
    }

    #[test]
    fn roundtrip_complex_frame_newline() {
        let data = Data::new(vec![
            0x01, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x7F, 0x7F, 0x06, 0x0C, 0x18, 0x7F, 0x7F, 0x00,
        ]).unwrap();
        let frame = Frame::new(Address(0x00), MsgType(0x00), data);

        let encoded = frame.to_bytes_with_newline();
        let decoded = Frame::from_bytes(&encoded).unwrap();

        assert_eq!(&b":1000000001100000000000007F7F060C187F7F00B9\r\n"[..], encoded.as_slice());
        assert_eq!(frame, decoded);
    }

    #[test]
    fn roundtrip_empty_data() {
        let frame = Frame::new(Address(0x2B), MsgType(0xA9), Data::from(&[]));

        let encoded = frame.to_bytes();
        let decoded = Frame::from_bytes(&encoded).unwrap();

        assert_eq!(b":00002BA92C", encoded.as_slice());
        assert_eq!(frame, decoded);
    }

    #[test]
    fn data_length_over_255_rejected() {
        let data = Data::new(vec![0; 256]);
        assert_matches!(data, Err(Error(ErrorKind::Argument(_), _)));
    }

    #[test]
    fn newline_accepted() {
        let decoded = Frame::from_bytes(b":01007F02FF7F\r\n").unwrap();
        assert_eq!(Frame::new(Address(0x7F), MsgType(0x02), Data::from(&[0xFF])), decoded);
    }

    #[test]
    fn bad_checksum_detected() {
        let decoded = Frame::from_bytes(b":01007F02FF7E");
        assert_matches!(decoded, Err(Error(ErrorKind::Checksum(_, _, _), _)));
    }

    #[test]
    fn extra_data_detected() {
        let decoded = Frame::from_bytes(b":00007F02007F");
        assert_matches!(decoded, Err(Error(ErrorKind::Parse(_, _), _)));
    }

    #[test]
    fn missing_data_detected() {
        let decoded = Frame::from_bytes(b":01007F027E");
        assert_matches!(decoded, Err(Error(ErrorKind::Parse(_, _), _)));
    }

    #[test]
    fn invalid_format_detected() {
        let decoded = Frame::from_bytes(b":01");
        assert_matches!(decoded, Err(Error(ErrorKind::Parse(_, _), _)));
    }

    #[test]
    fn garbage_detected() {
        let decoded = Frame::from_bytes(b"asdgdfg");
        assert_matches!(decoded, Err(Error(ErrorKind::Parse(_, _), _)));
    }

    #[test]
    fn bad_char_detected() {
        let decoded = Frame::from_bytes(b":01007F020z7E");
        assert_matches!(decoded, Err(Error(ErrorKind::Parse(_, _), _)));
    }

    #[test]
    fn missing_char_detected() {
        let decoded = Frame::from_bytes(b":01007F0207E");
        assert_matches!(decoded, Err(Error(ErrorKind::Parse(_, _), _)));
    }

    #[test]
    fn leading_chars_detected() {
        let decoded = Frame::from_bytes(b"abc:01007F02FF7Fa");
        assert_matches!(decoded, Err(Error(ErrorKind::Parse(_, _), _)));
    }

    #[test]
    fn trailing_chars_detected() {
        let decoded = Frame::from_bytes(b":01007F02FF7Fabc");
        assert_matches!(decoded, Err(Error(ErrorKind::Parse(_, _), _)));
    }

    #[test]
    fn parsed_lifetime_independent() {
        let decoded = {
            let string = b":01007F02FF7F".to_owned();
            Frame::from_bytes(&string).unwrap()
        };
        assert_eq!(Frame::new(Address(0x7F), MsgType(0x02), Data::from(&[0xFF])), decoded);
    }

    #[test]
    fn getters() {
        let frame = Frame::new(Address(0x7F), MsgType(0x02), Data::from(&[0xFF]));
        assert_eq!(frame.message_type(), MsgType(0x02));
        assert_eq!(frame.address(), Address(0x7F));
        assert_eq!(frame.data(), &vec![0xFFu8]);
    }

    #[test]
    fn write() {
        let frame = Frame::new(Address(0x7F), MsgType(0x02), Data::from(&[0xFF]));
        let mut output = Vec::new();
        frame.write(&mut output).unwrap();
        assert_eq!(b":01007F02FF7F\r\n", output.as_slice());
    }

    #[test]
    fn read() {
        let mut buffer = &b":01007F02FF7F\r\n"[..];
        let frame = Frame::read(&mut buffer).unwrap();
        assert_eq!(Frame::new(Address(0x7F), MsgType(0x02), Data::from(&[0xFF])), frame);
    }

    #[test]
    fn display() {
        let frame = Frame::new(Address(0x7F), MsgType(0x02), Data::from(&[0xFF, 0xCB]));
        let display = format!("{}", frame);
        assert_eq!("Type 02 | Addr 007F | Data FF CB", display.trim());
    }
}
