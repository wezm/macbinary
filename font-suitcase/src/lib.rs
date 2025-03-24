#![deny(missing_docs)]

//! Font suitcase parser
//!
//! Read bitmap font data from a font suitecase used on classic Mac OS.
//!
//! ### Specifications:
//!
//! - [Inside Macintosh: Text](https://developer.apple.com/library/archive/documentation/mac/pdf/Text.pdf)

use core::fmt;

use bitflags::bitflags;
use macbinary::{FourCC, ResourceFork};
use read_binary::{
    I16Be, ParseError, U16Be,
    read::{ReadArray, ReadBinary, ReadCtxt, ReadFrom, ReadScope},
};

pub use macbinary;

#[cfg(test)]
mod test;

type short = i16;
type long = i32;

const FOND: FourCC = FourCC(u32::from_be_bytes(*b"FOND"));
const FONT: FourCC = FourCC(u32::from_be_bytes(*b"FONT"));
const NFNT: FourCC = FourCC(u32::from_be_bytes(*b"NFNT"));
const SFNT: FourCC = FourCC(u32::from_be_bytes(*b"sfnt"));

/// A font suitcase
pub struct Suitcase<'a> {
    data: ResourceFork<'a>,
}

/// Font suitcase error
#[derive(Debug)]
pub enum Error {
    /// Error encounted parsing data.
    Parse(ParseError),
    /// Data is missing the specified resource.
    MissingResource(FourCC),
}

/// Represents a `sfnt` or `NFNT`/`FONT` in a font suitcase
pub enum Font<'a> {
    /// Font is a bitmap font in a `NFNT` or `FONT` resource.
    Bitmap(FontRec<'a>),
    /// Font is an outline font (`sfnt`)
    Outline(&'a [u8]),
}

/// Font Family Record
///
/// Describes the format of the font family (`FOND`) resource.
///
/// Reference: Inside Macintosh: Text pp 4-116
pub struct FamRec<'a> {
    /// flags for family
    pub flags: short,
    /// family ID number
    pub fam_id: short,
    /// ASCII code of 1st character
    pub first_char: short,
    /// ASCII code of last character
    pub last_char: short,
    /// maximum ascent for 1 pt font
    pub ascent: short,
    /// maximum descent for 1 pt font
    pub descent: short,
    /// maximum leading for 1 pt font
    pub leading: short,
    /// maximum widMax for 1 pt font
    pub wid_max: short,
    /// offset to width table
    pub w_tab_off: long,
    /// offset to kerning table
    pub kern_off: long,
    /// offset to style-mapping table
    pub styl_off: long,
    /// style property info
    pub property: [short; 9],
    /// for international use
    pub intl: [short; 2],
    /// version number
    pub version: short,
    font_assoc: FontAssoc<'a>,
}

/// Font Association Table
///
/// Entries are sorted according to point size, with the smallest sizes coming first in the table.
/// The font size value for outline font resources is 0, so they are always listed first.
/// Plain fonts are sorted before styled fonts.
pub struct FontAssoc<'a> {
    entries: ReadArray<'a, AsscEntry>,
}

/// Font Association Entry
#[derive(Debug, Clone, Copy)]
pub struct AsscEntry {
    /// point size of font
    ///
    /// The font size value for outline font resources is 0.
    pub font_size: short,
    /// style of font
    pub font_style: short,
    /// font resource ID
    ///
    /// This integer value specifies the resource ID of the related `sfnt`, `NFNT`, or `FONT`
    /// resource.
    pub font_id: short,
}

/// Bitmapped Font Record
///
/// Describes the format of the bitmapped font (`NFNT`) resource (and, likewise, the `FONT`
/// resource).
#[derive(Debug)]
pub struct FontRec<'a> {
    /// Font type
    pub font_type: FontType,
    /// Character code of first glyph
    ///
    /// An integer value that specifies the ASCII character code of the first glyph in the font.
    pub first_char: u16,
    /// Character code of last glyph
    ///
    /// An integer value that specifies the ASCII character code of the last glyph in the font.
    pub last_char: u16,
    /// Maximum glyph width
    ///
    /// An integer value that specifies the maximum width of the widest glyph in the font, in
    /// pixels.
    pub wid_max: short,
    /// Negative of maximum glyph kern
    ///
    /// An integer value that specifies the distance from the font rectangle’s glyph origin to the
    /// left edge of the font rectangle, in pixels. If a glyph in the font kerns to the left, the
    /// amount is represented as a negative number. If the glyph origin lies on the left edge of
    /// the font rectangle, the value of the kernMax field is 0.
    pub kern_max: short,
    /// Negative of descent
    ///
    /// If this font has very large tables and this value is positive, this value is the high word
    /// of the offset to the width/offset table. If this value is negative, it is the negative of
    /// the descent and is not used by the Font Manager.
    pub n_descent: short,
    /// Width of font rectangle in pixels
    ///
    /// An integer value that specifies the width, in pixels, of the image created if all the
    /// glyphs in the font were superimposed at their glyph origins.
    pub f_rect_width: u16,
    /// height of font rectangle in pixels
    ///
    /// An integer value that specifies the height, in pixels, of the image created if all the
    /// glyphs in the font were superimposed at their glyph origins. This value equals the sum of
    /// the maximum ascent and maximum descent measurements for the font.
    pub f_rect_height: u16,
    /// Maximum ascent
    ///
    /// An integer value that specifies the maximum ascent measurement for the entire font, in
    /// pixels. The ascent is the distance from the glyph origin to the top of the font rectangle.
    pub ascent: short,
    /// Maximum descent
    ///
    /// An integer value that specifies the maximum descent measurement for the entire font, in
    /// pixels. The descent is the distance from the glyph origin to the bottom of the font
    /// rectangle.
    pub descent: short,
    /// Leading
    ///
    /// An integer value that specifies the leading measurement for the entire font, in pixels.
    /// Leading is the distance from the descent line of one line of single-spaced text to the
    /// ascent line of the next line of text.
    pub leading: short,
    /// Row width of bit image / 2
    ///
    /// An integer value that specifies the width of the bit image, in words.
    pub row_words: u16,
    /// Bit image table.
    ///
    /// The bit image of the glyphs in the font. The glyph images of every defined glyph in the
    /// font are placed sequentially in order of increasing ASCII code. The bit image is one pixel
    /// image with no undefined stretches that has a height given by the value of the font
    /// rectangle element and a width given by the value of the bit image row width element. The
    /// image is padded at the end with extra pixels to make its length a multiple of 16.
    pub bit_image_table: &'a [u8],
    /// Bitmap location table.
    ///
    /// For every glyph in the font, this table contains a word that specifies the bit offset to
    /// the location of the bitmap for that glyph in the bit image table. If a glyph is missing
    /// from the font, its entry contains the same value for its location as the entry for the next
    /// glyph. The missing glyph is the last glyph of the bit image for that font. The last word of
    /// the table contains the offset to one bit beyond the end of the bit image. You can determine
    /// the image width of each glyph from the bitmap location table by subtracting the bit offset
    /// to that glyph from the bit offset to the next glyph in the table.
    pub bitmap_location_table: ReadArray<'a, U16Be>,
    /// Width/offset table.
    ///
    /// For every glyph in the font, this table contains a word with the glyph offset in the
    /// high-order byte and the glyph’s width, in integer form, in the low-order byte. The value of
    /// the offset, when added to the maximum kerning value for the font, determines the horizontal
    /// distance from the glyph origin to the left edge of the bit image of the glyph, in pixels.
    /// If this sum is negative, the glyph origin is to the right of the glyph image’s left edge,
    /// meaning the glyph kerns to the left. If the sum is positive, the origin is to the left of
    /// the image’s left edge. If the sum equals zero, the glyph origin corresponds with the left
    /// edge of the bit image. Missing glyphs are represented by a word value of –1. The last word
    /// of this table is also –1, representing the end.
    pub width_offset_table: ReadArray<'a, I16Be>,
    /// Glyph-width table.
    ///
    /// For every glyph in the font, this table contains a word that specifies the glyph’s
    /// fixed-point glyph width at the given point size and font style, in pixels. The Font Manager
    /// gives precedence to the values in this table over those in the font family glyph-width
    /// table. There is an unsigned integer in the high-order byte and a fractional part in the
    /// low-order byte. This table is optional.
    pub glyph_width_table: Option<ReadArray<'a, U16Be>>,
}

bitflags! {
    /// Flags for a FontRec record
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct FontType: u16 {
        /// This bit is set to 1 if the font resource contains an image height table.
        const HAS_HEIGHT = 0x1;
        /// This bit is set to 1 if the font resource contains a glyph-width table.
        const HAS_GLYPH_WIDTH = 0x2;
        /// These two bits define the depth of the font.
        const BIT_DEPTH = 0xC;
        // 4-6 Reserved. Set to 0
        /// This bit is set to 1 if the font has a font color table ('fctb') resource.
        const HAS_FCTB = 0x40;
        /// This bit is set to 1 if the font is a synthetic font, created dynamically
        /// from the available font resources in response to a certain color and screen depth
        /// combination.
        const IS_SYNTHETIC = 0x80;
        /// This bit is set to 1 if the font contains colors other than black.
        const HAS_COLOR = 0x100;
        // 10-11 Reserved. Set to 0
        // 12 Reserved. Set to 1
        /// This bit is set to 1 if the font describes a fixed-width font, and is set to 0 if the
        /// font describes a proportional font.
        const FIXED_WIDTH = 0x1000;
        /// This bit is set to 1 if the font is not to be expanded to match the screen depth.
        const NO_EXPAND = 0x2000;
        // 15 Reserved. Set to 0
    }
}

/// The region within the bit image a glyph occupies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphRegion {
    /// The offset in bits into each row of the bit image where this glyph is located.
    pub offset: u16,
    /// The width of the glyph bitmap in bits.
    pub width: u16,
}

/// Width and offset of a glyph for layout
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WidthOffset {
    /// The width of the glyph in pixels.
    pub width: u8,
    /// The offset of the glyph from the glyph origin.
    ///
    /// > The value of the offset, when added to the maximum kerning value for the font, determines
    /// > the horizontal distance from the glyph origin to the left edge of the bit image of the
    /// > glyph, in pixels. If this sum is negative, the glyph origin is to the right of the glyph
    /// > image’s left edge, meaning the glyph kerns to the left. If the sum is positive, the origin
    /// > is to the left of the image’s left edge. If the sum equals zero, the glyph origin
    /// > corresponds with the left edge of the bit image.
    pub offset: i8,
}

impl<'a> Suitcase<'a> {
    /// Open a font suitcase.
    pub fn new(data: &'a [u8]) -> Result<Suitcase<'a>, Error> {
        let data = ResourceFork::new(data)?;
        Ok(Suitcase { data })
    }

    /// Retrieve an associated font from the suitcase.
    pub fn font(&self, entry: AsscEntry) -> Result<Font<'a>, Error> {
        match entry.font_size {
            // The font size value for outline font resources is 0
            0 => {
                let rsrc = self
                    .data
                    .get_resource(SFNT, entry.font_id)
                    .ok_or(Error::MissingResource(SFNT))?;
                Ok(Font::Outline(rsrc.data()))
            }
            _ => {
                // Try new font ('NFNT') resource first, followed by 'FONT'
                let rsrc = self
                    .data
                    .get_resource(NFNT, entry.font_id)
                    .or_else(|| self.data.get_resource(FONT, entry.font_id))
                    .ok_or(Error::MissingResource(FONT))?;
                let font = ReadScope::new(rsrc.data()).read::<FontRec>()?;
                Ok(Font::Bitmap(font))
            }
        }
    }

    /// The Font Family Record
    ///
    /// The font family record, describes the format of the font family (`FOND`) resource.
    pub fn font_family(&'a self) -> Result<FamRec<'a>, Error> {
        let type_list = self
            .data
            .resource_types()
            .find(|item| item.resource_type() == FOND)
            .ok_or(Error::MissingResource(FOND))?;
        // This assumes there can only be one FOND resource, but not sure that's a given.
        let rsrc = self
            .data
            .resources(type_list)
            .next()
            .ok_or(Error::MissingResource(FOND))?;
        ReadScope::new(rsrc.data())
            .read::<FamRec<'a>>()
            .map_err(Error::from)
    }
}

impl<'a> FamRec<'a> {
    /// Iterate over the font entries in this font family
    pub fn fonts(&self) -> &ReadArray<'a, AsscEntry> {
        &self.font_assoc.entries
    }
}

impl ReadBinary for FamRec<'_> {
    type HostType<'a> = FamRec<'a>;

    fn read<'a>(
        ctxt: &mut read_binary::read::ReadCtxt<'a>,
    ) -> Result<Self::HostType<'a>, ParseError> {
        let flags = ctxt.read_i16be()?;
        let fam_id = ctxt.read_i16be()?;
        let first_char = ctxt.read_i16be()?;
        let last_char = ctxt.read_i16be()?;
        let ascent = ctxt.read_i16be()?;
        let descent = ctxt.read_i16be()?;
        let leading = ctxt.read_i16be()?;
        let wid_max = ctxt.read_i16be()?;
        let w_tab_off = ctxt.read_i32be()?;
        let kern_off = ctxt.read_i32be()?;
        let styl_off = ctxt.read_i32be()?;
        let mut property = [0; 9];
        for (i, prop) in ctxt.read_array::<I16Be>(9)?.iter().enumerate() {
            property[i] = prop;
        }
        let intl = [ctxt.read_i16be()?, ctxt.read_i16be()?];
        let version = ctxt.read_i16be()?;
        let font_assoc = ctxt.read::<FontAssoc>()?;

        Ok(FamRec {
            flags,
            fam_id,
            first_char,
            last_char,
            ascent,
            descent,
            leading,
            wid_max,
            w_tab_off,
            kern_off,
            styl_off,
            property,
            intl,
            version,
            font_assoc,
        })
    }
}

impl ReadBinary for FontAssoc<'_> {
    type HostType<'a> = FontAssoc<'a>;

    fn read<'a>(
        ctxt: &mut read_binary::read::ReadCtxt<'a>,
    ) -> Result<Self::HostType<'a>, ParseError> {
        let n_entries = usize::from(ctxt.read_u16be()? + 1); // Value is stored - 1
        let entries = ctxt.read_array::<AsscEntry>(n_entries)?;
        Ok(FontAssoc { entries })
    }
}

impl ReadFrom for AsscEntry {
    type ReadType = (I16Be, I16Be, I16Be);

    fn from((font_size, font_style, font_id): (i16, i16, i16)) -> Self {
        AsscEntry {
            font_size,
            font_style,
            font_id,
        }
    }
}

impl FontRec<'_> {
    /// Retrieve information about the region with the bit image that this glyph occupies.
    ///
    /// An offset and width are returned. The `offset` is the offset in bits into each row of the
    /// bit image where this glyph is located. `width` is the width of the glyph bitmap in bits.
    pub fn glyph_region(&self, glyph_id: u16) -> Option<GlyphRegion> {
        let glyph_id = usize::from(glyph_id);
        (glyph_id < self.bitmap_location_table.len().saturating_sub(1)).then(|| {
            let offset = self.bitmap_location_table.get_item(glyph_id);
            let next = self.bitmap_location_table.get_item(glyph_id + 1);
            let width = next - offset;
            GlyphRegion { offset, width }
        })
    }

    /// Obtain the width and offset of the glyph for layout.
    ///
    /// > For every glyph in the font, this table contains a word with the glyph offset in the
    /// > high-order byte and the glyph’s width, in integer form, in the low-order byte. The value of
    /// > the offset, when added to the maximum kerning value for the font, determines the horizontal
    /// > distance from the glyph origin to the left edge of the bit image of the glyph, in pixels.
    /// > If this sum is negative, the glyph origin is to the right of the glyph image’s left edge,
    /// > meaning the glyph kerns to the left. If the sum is positive, the origin is to the left of
    /// > the image’s left edge. If the sum equals zero, the glyph origin corresponds with the left
    /// > edge of the bit image.
    pub fn width_offset(&self, glyph_id: u16) -> Option<WidthOffset> {
        let glyph_id = usize::from(glyph_id);
        let raw = (glyph_id < self.width_offset_table.len())
            .then(|| self.width_offset_table.get_item(glyph_id))?;

        // For every glyph in the font, this table contains a word with
        // the glyph offset in the high-order byte and the glyph’s width, in integer form, in the
        // low-order byte. Missing glyphs are represented by a word value of –1.
        (raw != -1).then(|| {
            let width = (raw & 0xFF).abs() as u8;
            let offset = (raw >> 8) as i8;
            WidthOffset { width, offset }
        })
    }
}

impl ReadBinary for FontRec<'_> {
    type HostType<'a> = FontRec<'a>;

    fn read<'a>(ctxt: &mut ReadCtxt<'a>) -> Result<Self::HostType<'a>, ParseError> {
        let font_type = ctxt.read_u16be().map(FontType::from_bits_retain)?;
        let first_char = ctxt.read_u16be()?;
        let last_char = ctxt.read_u16be()?;
        let num_glyphs = usize::from(last_char - first_char) + 2; // +1 to get glyph count, +1 for missing glyph at end
        let wid_max = ctxt.read_i16be()?;
        let kern_max = ctxt.read_i16be()?;
        let n_descent = ctxt.read_i16be()?;
        let f_rect_width = ctxt.read_u16be()?;
        let f_rect_height = ctxt.read_u16be()?;
        let ow_t_loc = usize::from(ctxt.read_u16be()?) * 2; // Value is in "words"

        // TODO: The value of nDescent, when positive, is used as the high-order 16 bits in the
        // 32-bit value that is used to store the offset of the width table from the beginning of
        // the resource.
        let offset_width_scope = ctxt.scope().offset_length(ow_t_loc, 2 * num_glyphs)?;

        let ascent = ctxt.read_i16be()?;
        let descent = ctxt.read_i16be()?;
        let leading = ctxt.read_i16be()?;
        let row_words = ctxt.read_u16be()?;
        let bit_image_len = word_align(usize::from(f_rect_height) * usize::from(row_words) * 2); // ×2 because value is in words
        let bit_image_table = ctxt.read_slice(bit_image_len)?;
        let bitmap_location_table = ctxt.read_array::<U16Be>(num_glyphs)?;
        let mut offset_width_ctxt = offset_width_scope.ctxt();
        let width_offset_table = offset_width_ctxt.read_array::<I16Be>(num_glyphs)?;
        let glyph_width_table = font_type
            .intersects(FontType::HAS_GLYPH_WIDTH)
            .then(|| offset_width_ctxt.read_array::<U16Be>(num_glyphs))
            .transpose()?;

        Ok(FontRec {
            font_type,
            first_char,
            last_char,
            wid_max,
            kern_max,
            n_descent,
            f_rect_width,
            f_rect_height,
            ascent,
            descent,
            leading,
            row_words,
            bit_image_table,
            bitmap_location_table,
            width_offset_table,
            glyph_width_table,
        })
    }
}

impl From<ParseError> for Error {
    fn from(error: ParseError) -> Self {
        Error::Parse(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse(err) => err.fmt(f),
            Error::MissingResource(tag) => write!(f, "missing '{}' resource", tag),
        }
    }
}

impl std::error::Error for Error {}

/// Calculate the length required to 16-bit (word) align data of length `len`
pub const fn word_align(len: usize) -> usize {
    (len + 1) / 2 * 2
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::test::read_fixture;

    const PERCENT_REGIONS: &[GlyphRegion] = &[
        // entry for sfnt
        GlyphRegion {
            offset: 0,
            width: 0,
        },
        // 7pt
        GlyphRegion {
            offset: 32,
            width: 5,
        },
        // 9pt
        GlyphRegion {
            offset: 34,
            width: 5,
        },
        // 10pt
        GlyphRegion {
            offset: 34,
            width: 5,
        },
        // 12pt
        GlyphRegion {
            offset: 44,
            width: 7,
        },
        // 14pt
        GlyphRegion {
            offset: 55,
            width: 8,
        },
        // 18pt
        GlyphRegion {
            offset: 71,
            width: 10,
        },
        // 24pt
        GlyphRegion {
            offset: 98,
            width: 14,
        },
    ];

    // These expected values were obtained from ResEdit
    const WIDTH_OFFSETS: &[WidthOffset] = &[
        // entry for sfnt
        WidthOffset {
            offset: 0,
            width: 0,
        },
        // 7pt
        WidthOffset {
            offset: 2,
            width: 5,
        },
        // 9pt
        WidthOffset {
            offset: 1,
            width: 6,
        },
        // 10pt
        WidthOffset {
            offset: 1,
            width: 6,
        },
        // 12pt
        WidthOffset {
            offset: 1,
            width: 7,
        },
        // 14pt
        WidthOffset {
            offset: 1,
            width: 9,
        },
        // 18pt
        WidthOffset {
            offset: 2,
            width: 12,
        },
        // 24pt
        WidthOffset {
            offset: 3,
            width: 16,
        },
    ];

    #[test]
    fn pro_font() -> Result<(), Error> {
        let data = read_fixture("tests/ProFont.bin");
        let mb = macbinary::parse(&data)?;
        let suitcase = mb
            .resource_fork()?
            .map(|fork| Suitcase { data: fork })
            .unwrap();

        let family = suitcase.font_family()?;

        assert_eq!(family.fonts().len(), PERCENT_REGIONS.len());

        for (entry, (expected_region, expected_wo)) in family.fonts().iter().zip(
            PERCENT_REGIONS
                .iter()
                .copied()
                .zip(WIDTH_OFFSETS.iter().copied()),
        ) {
            let font = suitcase.font(entry)?;
            match font {
                Font::Bitmap(font_rec) => {
                    println!(
                        "got bitmap font for {}pt {}x{} stride {}",
                        entry.font_size,
                        font_rec.f_rect_width,
                        font_rec.f_rect_height,
                        font_rec.row_words * 2 * 8
                    );
                    // dbg!(font_rec);
                    let glyph_id = '%' as u16;
                    let region = font_rec.glyph_region(glyph_id).unwrap();
                    assert_eq!(region, expected_region);

                    let wo = font_rec.width_offset(glyph_id).unwrap();
                    assert_eq!(wo, expected_wo);
                }
                Font::Outline(_) => println!("got outline font"),
            }
        }

        Ok(())
    }
}
