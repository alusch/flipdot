error_chain! {
    foreign_links {
        Io(::std::io::Error) #[doc = "Error while performing I/O."];
    }

    errors {
        /// Invalid input was provided, for example trying to create a [`Frame`] with too many data bytes.
        ///
        /// [`Frame`]: struct.Frame.html
        Argument(message: String) {
            description("Invalid argument")
            display("Invalid argument: {}", message)
        }

        /// The specified data did not match the HEX format.
        ///
        /// Either the overall input was malformed or the number of data bytes did not match the indicated value.
        Parse(data: Vec<u8>, message: String) {
            description("Failed to parse frame")
            display("Error parsing [{}] {}", String::from_utf8_lossy(data), message)
        }

        /// The checksum in the frame did not match the computed checksum.
        Checksum(data: Vec<u8>, expected: u8, actual: u8) {
            description("Checksums don't match")
            display("Checksum error for [{}] expected 0x{:02X}, got 0x{:02X}", String::from_utf8_lossy(data), expected, actual)
        }
    }
}

impl From<Error> for Box<::std::error::Error + Send> {
    fn from(e: Error) -> Self {
        Box::new(e)
    }
}
