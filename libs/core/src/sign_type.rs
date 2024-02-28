use thiserror::Error;

/// Errors related to [`SignType`]s.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SignTypeError {
    /// [`SignType`] configuration data was not 16 bytes long.
    #[error("Wrong sign configuration data length: Expected {}, got {}", expected, actual)]
    WrongConfigLength {
        /// The expected configuration data length.
        expected: u8,

        /// The actual value of the configuration data that was provided.
        actual: usize,
    },

    /// Configuration data didn't match any known [`SignType`].
    #[error("Configuration data didn't match any known sign: {:?}", bytes)]
    UnknownConfig {
        /// The provided configuration data.
        bytes: Vec<u8>,
    },
}

/// The configuration information for a particular model of sign.
///
/// In order to communicate with a sign, we need to send the proper configuration
/// data, which includes an ID, size information, and a few other things.
/// This enum represents the signs for which that data is known, and thus
/// we are able to communicate with.
///
/// # Examples
///
/// ```
/// use flipdot_core::SignType;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #
/// let sign_type = SignType::Max3000Front112x16;
/// assert_eq!((112, 16), sign_type.dimensions());
///
/// let config = sign_type.to_bytes();
/// let parsed_type = SignType::from_bytes(config)?;
/// assert_eq!(sign_type, parsed_type);
/// #
/// # Ok(()) }
/// ```
///
/// # Format Details
///
/// The first byte indicates the type of sign, and different types have different formats for
/// the rest of the configuration block.
///
/// ## Max3000
///
/// ```text
///      ┌────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┐
///      │0x04│ ID │0x00│ ?? │  H │ W1 │ W2 │ W3 │ W4 │  B │0x00│0x00│0x00│0x00│0x00│0x00│
///      └────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┘
/// Byte    0    1    2    3    4    5    6    7    8    9   10   11   12   13   14   15
/// ```
///
/// Max3000 signs have an initial byte of `0x04`. `ID` is a unique ID for the particular sign type
/// within the family, e.g. the 90 × 7 side sign has ID `0x20`. Byte 2 seems to always be zero,
/// and byte 3 is unknown. `H` is the height in pixels, and `W1 + W2 + W3 + W4` is the
/// total width. `B` indicates the number of bits per column (either 8 or 16). The remaining
/// bytes appear unused and are always zero.
///
/// ## Horizon
///
/// ```text
///      ┌────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬────┐
///      │0x08│ ID │0x00│ ?? │ ?? │  H │0x00│  W │ A1 │ A2 │ B1 │ B2 │ ?? │0x00│0x00│0x00│
///      └────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴────┘
/// Byte    0    1    2    3    4    5    6    7    8    9   10   11   12   13   14   15
/// ```
///
/// Horizon signs have an initial byte of `0x08`. `ID` is a unique ID for the particular sign type
/// within the family, e.g. the 96 × 8 side sign has ID `0xB4`. Byte 2 seems to always be zero,
/// and bytes 3 and 4 are unknown. `H` is the height in pixels, and `W` is the width. The next
/// four bytes seem to indicate the arrangement of sub-panels to create the final width:
/// `W = A1 × B1 + A2 × B2`. Byte 12 is unknown (generally zero but `0x04` for the 40 × 12 dash sign).
/// The remaining bytes appear unused and are always zero.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SignType {
    /// Max3000 flip-dot sign, front, 112 × 16 pixels
    Max3000Front112x16,
    /// Max3000 flip-dot sign, front, 98 × 16 pixels
    Max3000Front98x16,
    /// Max3000 flip-dot sign, side, 90 × 7 pixels
    Max3000Side90x7,
    /// Max3000 flip-dot sign, rear, 30 × 10 pixels
    Max3000Rear30x10,
    /// Max3000 flip-dot sign, rear, 23 × 10 pixels
    Max3000Rear23x10,
    /// Max3000 flip-dot sign, dash, 30 × 7 pixels
    Max3000Dash30x7,
    /// Horizon LED sign, front, 160 × 16 pixels
    HorizonFront160x16,
    /// Horizon LED sign, front, 140 × 16 pixels
    HorizonFront140x16,
    /// Horizon LED sign, side, 96 × 8 pixels
    HorizonSide96x8,
    /// Horizon LED sign, rear, 48 × 16 pixels
    HorizonRear48x16,
    /// Horizon LED sign, dash, 40 × 12 pixels
    HorizonDash40x12,
}

impl SignType {
    /// Converts a slice representing configuration data into a `SignType`.
    ///
    /// # Errors
    ///
    /// Returns:
    /// * [`SignTypeError::WrongConfigLength`] if the data is not 16 bytes long.
    /// * [`SignTypeError::UnknownConfig`] if the data does not correspond to a known sign type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_core::SignType;
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #
    /// let bytes = vec![0x04, 0x62, 0x00, 0x04, 0x0A, 0x1E, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    /// let sign_type = SignType::from_bytes(&bytes)?;
    /// assert_eq!(SignType::Max3000Rear30x10, sign_type);
    /// #
    /// # Ok(()) }
    /// ```
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, SignTypeError> {
        if bytes.len() != 16 {
            return Err(SignTypeError::WrongConfigLength {
                expected: 16,
                actual: bytes.len(),
            });
        }

        match (bytes[0], bytes[1]) {
            (0x4, 0x47) => Ok(SignType::Max3000Front112x16),
            (0x4, 0x4D) => Ok(SignType::Max3000Front98x16),
            (0x4, 0x20) => Ok(SignType::Max3000Side90x7),
            (0x4, 0x62) => Ok(SignType::Max3000Rear30x10),
            (0x4, 0x61) => Ok(SignType::Max3000Rear23x10),
            (0x4, 0x26) => Ok(SignType::Max3000Dash30x7),

            (0x8, 0xB1) => Ok(SignType::HorizonFront160x16),
            (0x8, 0xB2) => Ok(SignType::HorizonFront140x16),
            (0x8, 0xB4) => Ok(SignType::HorizonSide96x8),
            (0x8, 0xB5) => Ok(SignType::HorizonRear48x16),
            (0x8, 0xB9) => Ok(SignType::HorizonDash40x12),

            _ => Err(SignTypeError::UnknownConfig { bytes: bytes.into() }),
        }
    }

    /// Gets the dimensions (width, height), in pixels, of this sign type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_core::SignType;
    /// let sign_type = SignType::HorizonFront140x16;
    /// assert_eq!((140, 16), sign_type.dimensions());
    /// ```
    pub fn dimensions(self) -> (u32, u32) {
        match self {
            SignType::Max3000Front112x16 => (112, 16),
            SignType::Max3000Front98x16 => (98, 16),
            SignType::Max3000Side90x7 => (90, 7),
            SignType::Max3000Rear23x10 => (23, 10),
            SignType::Max3000Rear30x10 => (30, 10),
            SignType::Max3000Dash30x7 => (30, 7),

            SignType::HorizonFront160x16 => (160, 16),
            SignType::HorizonFront140x16 => (140, 16),
            SignType::HorizonSide96x8 => (96, 8),
            SignType::HorizonRear48x16 => (48, 16),
            SignType::HorizonDash40x12 => (40, 12),
        }
    }

    /// Gets the 16-byte configuration data for this sign type.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_core::SignType;
    /// let sign_type = SignType::Max3000Rear30x10;
    /// let expected = vec![0x04, 0x62, 0x00, 0x04, 0x0A, 0x1E, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    /// assert_eq!(expected, sign_type.to_bytes());
    /// ```
    pub fn to_bytes(self) -> &'static [u8] {
        match self {
            SignType::Max3000Front112x16 => &[
                0x04, 0x47, 0x00, 0x0F, 0x10, 0x1C, 0x1C, 0x1C, 0x1C, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
            SignType::Max3000Front98x16 => &[
                0x04, 0x4D, 0x00, 0x0D, 0x10, 0x0E, 0x1C, 0x1C, 0x1C, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
            SignType::Max3000Side90x7 => &[
                0x04, 0x20, 0x00, 0x06, 0x07, 0x1E, 0x1E, 0x1E, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
            SignType::Max3000Rear23x10 => &[
                0x04, 0x61, 0x00, 0x04, 0x0A, 0x17, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
            SignType::Max3000Rear30x10 => &[
                0x04, 0x62, 0x00, 0x04, 0x0A, 0x1E, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
            SignType::Max3000Dash30x7 => &[
                0x04, 0x26, 0x00, 0x03, 0x07, 0x1E, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],

            SignType::HorizonFront160x16 => &[
                0x08, 0xB1, 0x00, 0x15, 0x0C, 0x10, 0x00, 0xA0, 0x04, 0x00, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
            SignType::HorizonFront140x16 => &[
                0x08, 0xB2, 0x00, 0x12, 0x04, 0x10, 0x00, 0x8C, 0x01, 0x03, 0x14, 0x28, 0x00, 0x00, 0x00, 0x00,
            ],
            SignType::HorizonSide96x8 => &[
                0x08, 0xB4, 0x00, 0x07, 0x0C, 0x08, 0x00, 0x60, 0x02, 0x00, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
            SignType::HorizonRear48x16 => &[
                0x08, 0xB5, 0x00, 0x07, 0x0C, 0x10, 0x00, 0x30, 0x01, 0x00, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
            SignType::HorizonDash40x12 => &[
                0x08, 0xB9, 0x00, 0x06, 0x8C, 0x0C, 0x00, 0x28, 0x01, 0x00, 0x28, 0x00, 0x04, 0x00, 0x00, 0x00,
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    fn verify_roundtrip(sign_type: SignType, expected_bytes: &[u8]) -> Result<(), Box<dyn Error>> {
        let encoded = sign_type.to_bytes();
        assert_eq!(expected_bytes, encoded);

        let decoded = SignType::from_bytes(encoded)?;
        assert_eq!(sign_type, decoded);

        Ok(())
    }

    #[test]
    fn sign_type_roundtrip() -> Result<(), Box<dyn Error>> {
        verify_roundtrip(
            SignType::Max3000Front112x16,
            &vec![
                0x04, 0x47, 0x00, 0x0F, 0x10, 0x1C, 0x1C, 0x1C, 0x1C, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
        )?;

        verify_roundtrip(
            SignType::Max3000Front98x16,
            &vec![
                0x04, 0x4D, 0x00, 0x0D, 0x10, 0x0E, 0x1C, 0x1C, 0x1C, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
        )?;

        verify_roundtrip(
            SignType::Max3000Side90x7,
            &vec![
                0x04, 0x20, 0x00, 0x06, 0x07, 0x1E, 0x1E, 0x1E, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
        )?;

        verify_roundtrip(
            SignType::Max3000Rear23x10,
            &vec![
                0x04, 0x61, 0x00, 0x04, 0x0A, 0x17, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
        )?;

        verify_roundtrip(
            SignType::Max3000Rear30x10,
            &vec![
                0x04, 0x62, 0x00, 0x04, 0x0A, 0x1E, 0x00, 0x00, 0x00, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
        )?;

        verify_roundtrip(
            SignType::Max3000Dash30x7,
            &vec![
                0x04, 0x26, 0x00, 0x03, 0x07, 0x1E, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
        )?;

        verify_roundtrip(
            SignType::HorizonFront160x16,
            &vec![
                0x08, 0xB1, 0x00, 0x15, 0x0C, 0x10, 0x00, 0xA0, 0x04, 0x00, 0x28, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
        )?;

        verify_roundtrip(
            SignType::HorizonFront140x16,
            &vec![
                0x08, 0xB2, 0x00, 0x12, 0x04, 0x10, 0x00, 0x8C, 0x01, 0x03, 0x14, 0x28, 0x00, 0x00, 0x00, 0x00,
            ],
        )?;

        verify_roundtrip(
            SignType::HorizonSide96x8,
            &vec![
                0x08, 0xB4, 0x00, 0x07, 0x0C, 0x08, 0x00, 0x60, 0x02, 0x00, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
        )?;

        verify_roundtrip(
            SignType::HorizonRear48x16,
            &vec![
                0x08, 0xB5, 0x00, 0x07, 0x0C, 0x10, 0x00, 0x30, 0x01, 0x00, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00,
            ],
        )?;

        verify_roundtrip(
            SignType::HorizonDash40x12,
            &vec![
                0x08, 0xB9, 0x00, 0x06, 0x8C, 0x0C, 0x00, 0x28, 0x01, 0x00, 0x28, 0x00, 0x04, 0x00, 0x00, 0x00,
            ],
        )?;

        Ok(())
    }

    #[test]
    fn sizes_correct() {
        assert_eq!((112, 16), SignType::Max3000Front112x16.dimensions());
        assert_eq!((98, 16), SignType::Max3000Front98x16.dimensions());
        assert_eq!((90, 7), SignType::Max3000Side90x7.dimensions());
        assert_eq!((23, 10), SignType::Max3000Rear23x10.dimensions());
        assert_eq!((30, 10), SignType::Max3000Rear30x10.dimensions());
        assert_eq!((30, 7), SignType::Max3000Dash30x7.dimensions());

        assert_eq!((160, 16), SignType::HorizonFront160x16.dimensions());
        assert_eq!((140, 16), SignType::HorizonFront140x16.dimensions());
        assert_eq!((96, 8), SignType::HorizonSide96x8.dimensions());
        assert_eq!((48, 16), SignType::HorizonRear48x16.dimensions());
        assert_eq!((40, 12), SignType::HorizonDash40x12.dimensions());
    }

    #[test]
    fn unknown_type_rejected() {
        let data = vec![
            0x10, 0xB9, 0x00, 0x06, 0x8C, 0x0C, 0x00, 0x28, 0x01, 0x00, 0x28, 0x00, 0x04, 0x00, 0x00, 0x00,
        ];
        let error = SignType::from_bytes(&data).unwrap_err();
        assert!(matches!(error, SignTypeError::UnknownConfig { .. }));
    }

    #[test]
    fn unknown_horizon_rejected() {
        let data = vec![
            0x08, 0xBA, 0x00, 0x06, 0x8C, 0x0C, 0x00, 0x18, 0x01, 0x00, 0x28, 0x00, 0x04, 0x00, 0x00, 0x00,
        ];
        let error = SignType::from_bytes(&data).unwrap_err();
        assert!(matches!(error, SignTypeError::UnknownConfig { .. }));
    }

    #[test]
    fn unknown_max3000_rejected() {
        let data = vec![
            0x04, 0x21, 0x00, 0x06, 0x07, 0x10, 0x10, 0x10, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let error = SignType::from_bytes(&data).unwrap_err();
        assert!(matches!(error, SignTypeError::UnknownConfig { .. }));
    }

    #[test]
    fn not_enough_data() {
        let data = vec![0x04];
        let error = SignType::from_bytes(&data).unwrap_err();
        assert!(matches!(
            error,
            SignTypeError::WrongConfigLength {
                expected: 16,
                actual: 1,
                ..
            }
        ));
    }

    #[test]
    fn too_much_data() {
        let data = vec![
            0x08, 0xB9, 0x00, 0x06, 0x8C, 0x0C, 0x00, 0x28, 0x01, 0x00, 0x28, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00,
        ];
        let error = SignType::from_bytes(&data).unwrap_err();
        assert!(matches!(
            error,
            SignTypeError::WrongConfigLength {
                expected: 16,
                actual: 17,
                ..
            }
        ));
    }
}
