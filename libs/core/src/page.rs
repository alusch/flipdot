use std::borrow::Cow;
use std::fmt::{self, Display, Formatter};

use derive_more::{Display, LowerHex, UpperHex};
use thiserror::Error;

/// Errors relating to [`Page`]s.
#[derive(Copy, Clone, Debug, Error)]
#[non_exhaustive]
pub enum PageError {
    /// Data length didn't match the width/height of the [`Page`].
    #[error(
        "Wrong number of data bytes for a {}x{} page: Expected {}, got {}",
        width,
        height,
        expected,
        actual
    )]
    WrongPageLength {
        /// The page width.
        width: u32,

        /// The page height.
        height: u32,

        /// The expected length of the page data.
        expected: usize,

        /// The actual length of the page data that was provided.
        actual: usize,
    },
}

/// A page of a message for display on a sign.
///
/// # Examples
///
/// ```
/// use flipdot_core::{Page, PageId};
///
/// let mut page = Page::new(PageId(1), 30, 10); // Create 30x10 page with ID 1
/// page.set_pixel(3, 5, true); // Turn on pixel at column 3 and row 5
/// ```
///
/// # Format Details
///
/// Data is stored in the native format, which consists of a 4-byte header and the data itself,
/// padded to a a multiple of 16 bytes. The pixel data is column-major, with one or more bytes per
/// column and one bit per pixel. The least significant bit is oriented toward the top of the display.
/// The `ID` field is a "page number" used to identify individual pages in multi-page messages.
/// The other bytes in the header are unknown, but from inspection of real ODKs seem to be most
/// commonly `0x10 0x00 0x00`, which is what [`Page::new`] currently uses.
///
/// ```text
///                   ┌─┬ ┄ ┬─┐
///              Bits │7│...│0│
///                   └─┴ ┄ ┴─┘
///                    \     /
/// ┌────┬────┬────┬────┬────┬────┬────┬────┬────┬────┬ ┄ ┬────┬ ┄ ┬────┐
/// │ ID │ ?? │ ?? │ ?? │  0 │  1 │  2 │  3 │  4 │  5 │...│0xFF│...│0xFF│
/// └────┴────┴────┴────┴────┴────┴────┴────┴────┴────┴ ┄ ┴────┴ ┄ ┴────┘
/// ┆   4-byte header   ┆            Data bytes           ┆   Padding   ┆
/// ```
///
/// Depending on the intended dimensions of the sign, the same data will be interpreted differently:
///
/// ```text
///                 7 height                         16 height
///
///                                            Bytes   0   2   4  ...
///  Bytes   0   1   2   3   4   5  ...                1   3   5  ...
///        ┌───┬───┬───┬───┬───┬───┬ ┄               ┌───┬───┬───┬ ┄
///    0   │ 0 │ 0 │ 0 │ 0 │ 0 │ 0 │             0   │ 0 │ 0 │ 0 │
///    |   ├───┼───┼───┼───┼───┼───┼ ┄           |   ├───┼───┼───┼ ┄
///   Row  │...│...│...│...│...│...│             |   │...│...│...│
///    |   ├───┼───┼───┼───┼───┼───┼ ┄           |   ├───┼───┼───┼ ┄
///    7   │ 6 │ 6 │ 6 │ 6 │ 6 │ 6 │             |   │ 7 │ 7 │ 7 │
///        └───┴───┴───┴───┴───┴───┴ ┄          Row  ╞═══╪═══╪═══╪ ┄
///          0 - - - Column- - - 5               |   │ 0 │ 0 │ 0 │
///                                              |   ├───┼───┼───┼ ┄
///              (bit 7 unused)                  |   │...│...│...│
///                                              |   ├───┼───┼───┼ ┄
///                                             15   │ 7 │ 7 │ 7 │
///                                                  └───┴───┴───┴ ┄
///                                                   0 - Col - 2
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Page<'a> {
    width: u32,
    height: u32,
    bytes: Cow<'a, [u8]>,
}

/// The page number of a [`Page`].
///
/// Used to identify a particular page in a multi-page message.
///
/// # Examples
///
/// ```
/// use flipdot_core::{Page, PageId};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// #
/// let page = Page::new(PageId(1), 10, 10);
/// assert_eq!(PageId(1), page.id());
/// #
/// # Ok(()) }
/// ```
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Display, LowerHex, UpperHex)]
pub struct PageId(pub u8);

/// Whether the sign or controller (ODK) is in charge of flipping pages.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub enum PageFlipStyle {
    /// The sign will flip pages itself.
    Automatic,

    /// The controller will notify the sign when to load/show pages.
    Manual,
}

impl<'a> Page<'a> {
    /// Creates a new `Page` with given ID and dimensions.
    ///
    /// All pixels are initially set to off. The data is owned by this `Page`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_core::{Page, PageId};
    /// let page = Page::new(PageId(1), 90, 7); // Create 90x7 page with ID 1
    /// assert_eq!(false, page.get_pixel(75, 3)); // All pixels initially off
    /// ```
    pub fn new(id: PageId, width: u32, height: u32) -> Self {
        let mut bytes = Vec::<u8>::with_capacity(Self::total_bytes(width, height));

        // 4-byte header
        bytes.extend_from_slice(&[id.0, 0x10, 0x00, 0x00]);

        // Fill remaining data bytes with 0 for a blank initial image
        bytes.resize(Self::data_bytes(width, height), 0x00);

        // Pad to multiple of 16 with 0xFF bytes
        bytes.resize(Self::total_bytes(width, height), 0xFF);

        Page {
            width,
            height,
            bytes: bytes.into(),
        }
    }

    /// Creates a new `Page` with given dimensions from the underlying byte representation.
    ///
    /// The data must be convertible to [`Cow`], which allows us to create efficient views of
    /// `Page`s over existing data without making copies.
    ///
    /// It is the caller's responsibility to ensure that the header and padding bytes are
    /// set appropriately as they are not validated.
    ///
    /// # Errors
    ///
    /// Returns [`PageError::WrongPageLength`] if the data length does not match
    /// the specified dimensions.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_core::{Page, PageId};
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// #
    /// let data: Vec<u8> = vec![1, 16, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255];
    /// let page = Page::from_bytes(8, 8, data)?;
    /// assert_eq!(PageId(1), page.id());
    /// assert_eq!(true, page.get_pixel(0, 0));
    /// assert_eq!(false, page.get_pixel(1, 0));
    ///
    /// let bad_data: Vec<u8> = vec![1, 0, 0, 0, 1];
    /// let bad_page = Page::from_bytes(1, 8, bad_data);
    /// assert!(bad_page.is_err());
    /// #
    /// # Ok(()) }
    /// ```
    pub fn from_bytes<T: Into<Cow<'a, [u8]>>>(width: u32, height: u32, bytes: T) -> Result<Self, PageError> {
        let page = Page {
            width,
            height,
            bytes: bytes.into(),
        };

        let expected_bytes = Self::total_bytes(width, height);
        if page.bytes.len() != expected_bytes {
            return Err(PageError::WrongPageLength {
                width,
                height,
                expected: expected_bytes,
                actual: page.bytes.len(),
            });
        }

        Ok(page)
    }

    /// Returns the ID (page number) of this page.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_core::{Page, PageId};
    /// let page = Page::new(PageId(1), 90, 7);
    /// println!("This is page {}", page.id().0);
    /// ```
    pub fn id(&self) -> PageId {
        PageId(self.bytes[0])
    }

    /// Returns the width of this page.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_core::{Page, PageId};
    /// let page = Page::new(PageId(1), 90, 7);
    /// println!("Page is {} pixels wide", page.width());
    /// ```
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the height of this page.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_core::{Page, PageId};
    /// let page = Page::new(PageId(1), 90, 7);
    /// println!("Page is {} pixels tall", page.height());
    /// ```
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns whether or not the pixel at the given `(x, y)` coordinate is on.
    ///
    /// # Panics
    ///
    /// Panics if `x` or `y` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_core::{Page, PageId};
    /// let page = Page::new(PageId(1), 90, 7);
    /// let (x, y) = (45, 2);
    /// println!("Pixel at {}, {} on? {}", x, y, page.get_pixel(x, y));
    /// ```
    pub fn get_pixel(&self, x: u32, y: u32) -> bool {
        let (byte_index, bit_index) = self.byte_bit_indices(x, y);
        let mask = 1 << bit_index;
        let byte = &self.bytes[byte_index];
        *byte & mask == mask
    }

    /// Turns the pixel at the given `(x, y)` coordinate on or off.
    ///
    /// # Panics
    ///
    /// Panics if `x` or `y` is out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_core::{Page, PageId};
    /// let mut page = Page::new(PageId(1), 90, 7);
    /// page.set_pixel(5, 5, true); // Turn on pixel...
    /// page.set_pixel(5, 5, false); // And turn it back off.
    /// ```
    pub fn set_pixel(&mut self, x: u32, y: u32, value: bool) {
        let (byte_index, bit_index) = self.byte_bit_indices(x, y);
        let mask = 1 << bit_index;
        let byte = &mut self.bytes.to_mut()[byte_index];
        if value {
            *byte |= mask;
        } else {
            *byte &= !mask;
        }
    }

    /// Returns the raw byte representation of this page.
    ///
    /// This is generally called on your behalf when sending a page to a sign.
    ///
    /// # Examples
    ///
    /// ```
    /// # use flipdot_core::{Page, PageId};
    /// let mut page = Page::new(PageId(1), 8, 8);
    /// page.set_pixel(0, 0, true);
    /// let bytes = page.as_bytes();
    /// assert_eq!(vec![1, 16, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255], bytes);
    /// ```
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the number of bytes used to store each column.
    fn bytes_per_column(height: u32) -> usize {
        (height as usize + 7) / 8 // Divide by 8 rounding up
    }

    /// Returns the number of actual meaningful bytes (including header but not padding).
    fn data_bytes(width: u32, height: u32) -> usize {
        4 + width as usize * Self::bytes_per_column(height)
    }

    /// Returns the total number of bytes, including the padding.
    fn total_bytes(width: u32, height: u32) -> usize {
        (Self::data_bytes(width, height) + 15) / 16 * 16 // Round to multiple of 16
    }

    /// Given an x-y coordinate, returns the byte and bit at which it is stored.
    fn byte_bit_indices(&self, x: u32, y: u32) -> (usize, u8) {
        if x >= self.width || y >= self.height {
            panic!(
                "Coordinate ({}, {}) out of bounds for page of size {} x {}",
                x, y, self.width, self.height
            );
        }

        let byte_index = 4 + x as usize * Self::bytes_per_column(self.height) + y as usize / 8;
        let bit_index = y % 8;
        (byte_index, bit_index as u8)
    }
}

impl Display for Page<'_> {
    /// Formats the page for display using ASCII art.
    ///
    /// Produces a multiline string with one character per pixel and a border.
    /// Should be displayed in a fixed-width font.
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let border = str::repeat("-", self.width as usize);
        writeln!(f, "+{}+", border)?;
        for y in 0..self.height {
            write!(f, "|")?;
            for x in 0..self.width {
                let dot = if self.get_pixel(x, y) { '@' } else { ' ' };
                write!(f, "{}", dot)?;
            }
            writeln!(f, "|")?;
        }
        write!(f, "+{}+", border)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn one_byte_per_column_empty() -> Result<(), Box<dyn Error>> {
        let page = Page::new(PageId(3), 90, 7);
        let bytes = page.as_bytes();
        #[rustfmt::skip]
        const EXPECTED: &[u8] = &[
            0x03, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF,
        ];
        assert_eq!(bytes, EXPECTED);

        let page2 = Page::from_bytes(90, 7, bytes)?;
        assert_eq!(page, page2);

        Ok(())
    }

    #[test]
    fn two_bytes_per_column_empty() -> Result<(), Box<dyn Error>> {
        let page = Page::new(PageId(1), 40, 12);
        let bytes = page.as_bytes();
        #[rustfmt::skip]
        const EXPECTED: &[u8] = &[
            0x01, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];
        assert_eq!(bytes, EXPECTED);

        let page2 = Page::from_bytes(40, 12, bytes)?;
        assert_eq!(page, page2);

        Ok(())
    }

    #[test]
    fn one_byte_per_column_set_bits() -> Result<(), Box<dyn Error>> {
        let mut page = Page::new(PageId(3), 90, 7);
        page.set_pixel(0, 0, true);
        page.set_pixel(89, 5, true);
        page.set_pixel(89, 6, true);
        page.set_pixel(4, 4, true);
        page.set_pixel(4, 4, false);
        let bytes = page.as_bytes();
        #[rustfmt::skip]
        const EXPECTED: &[u8] = &[
            0x03, 0x10, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x60, 0xFF, 0xFF,
        ];
        assert_eq!(bytes, EXPECTED);

        let page2 = Page::from_bytes(90, 7, bytes)?;
        assert_eq!(page, page2);

        Ok(())
    }

    #[test]
    fn two_bytes_per_column_set_bits() -> Result<(), Box<dyn Error>> {
        let mut page = Page::new(PageId(1), 40, 12);
        page.set_pixel(0, 0, true);
        page.set_pixel(0, 11, true);
        page.set_pixel(39, 5, true);
        page.set_pixel(39, 6, true);
        page.set_pixel(39, 8, true);
        page.set_pixel(4, 4, true);
        page.set_pixel(4, 4, false);
        let bytes = page.as_bytes();
        #[rustfmt::skip]
        const EXPECTED: &[u8] = &[
            0x01, 0x10, 0x00, 0x00, 0x01, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x60, 0x01, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
        ];
        assert_eq!(bytes, EXPECTED);

        let page2 = Page::from_bytes(40, 12, bytes)?;
        assert_eq!(page, page2);

        Ok(())
    }

    #[test]
    fn wrong_size_rejected() {
        let error = Page::from_bytes(90, 7, vec![0x01, 0x01, 0x03]).unwrap_err();
        assert!(matches!(
            error,
            PageError::WrongPageLength {
                expected: 96,
                actual: 3,
                ..
            }
        ));
    }

    #[test]
    fn set_get_pixels() {
        let mut page = Page::new(PageId(1), 16, 16);

        page.set_pixel(0, 0, true);
        assert_eq!(true, page.get_pixel(0, 0));
        page.set_pixel(0, 0, false);
        assert_eq!(false, page.get_pixel(0, 0));

        page.set_pixel(13, 10, true);
        assert_eq!(true, page.get_pixel(13, 10));
        page.set_pixel(13, 10, false);
        assert_eq!(false, page.get_pixel(13, 10));
    }

    #[test]
    #[should_panic]
    fn out_of_bounds_x() {
        let mut page = Page::new(PageId(1), 8, 8);
        page.set_pixel(9, 0, true);
    }

    #[test]
    #[should_panic]
    fn out_of_bounds_y() {
        let mut page = Page::new(PageId(1), 8, 8);
        page.set_pixel(0, 9, true);
    }

    #[test]
    fn display() {
        let mut page = Page::new(PageId(1), 2, 2);
        page.set_pixel(0, 0, true);
        page.set_pixel(1, 1, true);
        let display = format!("{}", page);
        let expected = "\
                        +--+\n\
                        |@ |\n\
                        | @|\n\
                        +--+";
        assert_eq!(expected, display);
    }
}
