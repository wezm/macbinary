// #![deny(missing_docs)]

//! Font suitcase parser
//!
//! Read bitmap font data from a font suitecase used on classic Mac OS.
//!
//! ### Specifications:
//!
//! - [Inside Macintosh: Text](https://developer.apple.com/library/archive/documentation/mac/pdf/Text.pdf)
//!
//! #### Other references:
//!
//! - [Detecting MacBinary format](https://entropymine.wordpress.com/2019/02/13/detecting-macbinary-format/)

use core::fmt;

use macbinary::{FourCC, ResourceFork};
use read_binary::{
    ParseError, U16Be,
    read::{ReadArray, ReadBinary, ReadCtxt, ReadFrom, ReadScope},
};

pub use macbinary;

#[cfg(test)]
mod test;

pub struct Suitcase<'a> {
    data: ResourceFork<'a>,
}

type short = u16;
type long = u32;

/// Font Family Record
///
/// Describes the format of the font family (`FOND`) resource.
///
/// Reference: Inside Macintosh: Text pp 4-116
pub struct FamRec<'a> {
    /// flags for family
    flags: short,
    /// family ID number
    fam_id: short,
    /// ASCII code of 1st character
    first_char: short,
    /// ASCII code of last character
    last_char: short,
    /// maximum ascent for 1 pt font
    ascent: short,
    /// maximum descent for 1 pt font
    descent: short,
    /// maximum leading for 1 pt font
    leading: short,
    /// maximum widMax for 1 pt font
    wid_max: short,
    /// offset to width table
    w_tab_off: long,
    /// offset to kerning table
    kern_off: long,
    /// offset to style-mapping table
    styl_off: long,
    /// style property info
    property: [short; 9],
    /// for international use
    intl: [short; 2],
    /// version number
    version: short,
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
#[derive(Debug)]
pub struct AsscEntry {
    /// point size of font
    ///
    /// The font size value for outline font resources is 0.
    pub font_size: short,
    /// style of font
    pub font_style: short,
    /// font resource ID
    ///
    /// This integer value specifies the resource ID of the related `sfnt`, `NFNT`, or `FONT` resource.
    font_id: short,
}

/// Bitmapped Font Record
///
/// Describes the format of the bitmapped font (`NFNT`) resource (and, likewise, the `FONT` resource).
pub struct FontRec {
    /// font type
    font_type: short,
    /// character code of first glyph
    first_char: short,
    /// character code of last glyph
    last_char: short,
    /// maximum glyph width
    wid_max: short,
    /// negative of maximum glyph kern
    kern_max: short,
    /// negative of descent
    n_descent: short,
    /// width of font rectangle
    f_rect_width: short,
    /// height of font rectangle
    f_rect_height: short,
    /// location of width/offset table
    ow_t_loc: short,
    /// ascent
    ascent: short,
    /// descent
    descent: short,
    /// leading
    leading: short,
    /// row width of bit image / 2
    ///
    /// An integer value that specifies the width of the bit image, in words.
    row_words: short,
    /// Bit image table.
    ///
    /// The bit image of the glyphs in the font. The glyph images of every
    /// defined glyph in the font are placed sequentially in order of increasing ASCII code.
    /// The bit image is one pixel image with no undefined stretches that has a height given
    /// by the value of the font rectangle element and a width given by the value of the bit
    /// image row width element. The image is padded at the end with extra pixels to make
    /// its length a multiple of 16.
    bit_image_table: (),
    /// Bitmap location table.
    ///
    /// For every glyph in the font, this table contains a word that
    /// specifies the bit offset to the location of the bitmap for that glyph in the bit image table.
    /// If a glyph is missing from the font, its entry contains the same value for its location as
    /// the entry for the next glyph. The missing glyph is the last glyph of the bit image for
    /// that font. The last word of the table contains the offset to one bit beyond the end of the
    /// bit image. You can determine the image width of each glyph from the bitmap location
    /// table by subtracting the bit offset to that glyph from the bit offset to the next glyph in
    /// the table.
    bitmap_location_table: (),
    /// Width/offset table.
    ///
    /// For every glyph in the font, this table contains a word with
    /// the glyph offset in the high-order byte and the glyph’s width, in integer form, in the
    /// low-order byte. The value of the offset, when added to the maximum kerning
    /// value for the font, determines the horizontal distance from the glyph origin to the left
    /// edge of the bit image of the glyph, in pixels. If this sum is negative, the glyph origin
    /// is to the right of the glyph image’s left edge, meaning the glyph kerns to the left.
    /// If the sum is positive, the origin is to the left of the image’s left edge. If the sum equals
    /// zero, the glyph origin corresponds with the left edge of the bit image. Missing glyphs
    /// are represented by a word value of –1. The last word of this table is also –1,
    /// representing the end.
    width_offset_table: (),
    /// Glyph-width table.
    ///
    /// For every glyph in the font, this table contains a word that specifies
    /// the glyph’s fixed-point glyph width at the given point size and font style, in pixels.
    /// The Font Manager gives precedence to the values in this table over those in the font
    /// family glyph-width table. There is an unsigned integer in the high-order byte and a
    /// fractional part in the low-order byte. This table is optional.
    glyph_width_table: Option<()>,
}

#[derive(Debug)]
pub enum Error {
    Parse(ParseError),
    /// Data is missing `FOND` resource.
    MissingFond,
}

impl<'a> Suitcase<'a> {
    pub fn new(data: &'a [u8]) -> Result<Suitcase<'a>, Error> {
        let data = ResourceFork::new(data)?;
        Ok(Suitcase { data })
    }

    /// Retrieve the `sfnt` resource.
    ///
    /// The outline font ('sfnt') resource, which describes a TrueType outline font.
    pub fn outline_font(&self) -> () {
        todo!("return sfnt resource")
    }

    /// The Font Family Record
    ///
    /// The font family record, describes the format of the font family (`FOND`) resource.
    pub fn font_family(&'a self) -> Result<FamRec<'a>, Error> {
        let fond = FourCC(u32::from_be_bytes(*b"FOND"));
        let type_list = self
            .data
            .resource_types()
            .find(|item| item.resource_type() == fond)
            .ok_or(Error::MissingFond)?;
        // This assumes there can only be one FOND resource, but not sure that's a given.
        let rsrc = self
            .data
            .resources(type_list)
            .next()
            .ok_or(Error::MissingFond)?;
        ReadScope::new(rsrc.data())
            .read::<FamRec<'a>>()
            .map_err(Error::from)
    }
}

impl<'a> FamRec<'a> {
    pub fn fonts(&self) -> &ReadArray<'a, AsscEntry> {
        &self.font_assoc.entries
    }
}

impl ReadBinary for FamRec<'_> {
    type HostType<'a> = FamRec<'a>;

    fn read<'a>(
        ctxt: &mut read_binary::read::ReadCtxt<'a>,
    ) -> Result<Self::HostType<'a>, ParseError> {
        let flags = ctxt.read_u16be()?;
        let fam_id = ctxt.read_u16be()?;
        let first_char = ctxt.read_u16be()?;
        let last_char = ctxt.read_u16be()?;
        let ascent = ctxt.read_u16be()?;
        let descent = ctxt.read_u16be()?;
        let leading = ctxt.read_u16be()?;
        let wid_max = ctxt.read_u16be()?;
        let w_tab_off = ctxt.read_u32be()?;
        let kern_off = ctxt.read_u32be()?;
        let styl_off = ctxt.read_u32be()?;
        let mut property = [0; 9];
        for (i, prop) in ctxt.read_array::<U16Be>(9)?.iter().enumerate() {
            property[i] = prop;
        }
        let intl = [ctxt.read_u16be()?, ctxt.read_u16be()?];
        let version = ctxt.read_u16be()?;
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
    type ReadType = (U16Be, U16Be, U16Be);

    fn from((font_size, font_style, font_id): (u16, u16, u16)) -> Self {
        AsscEntry {
            font_size,
            font_style,
            font_id,
        }
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
            Error::MissingFond => todo!(),
        }
    }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::read_fixture;

    #[test]
    fn pro_font() -> Result<(), Error> {
        let data = read_fixture("tests/ProFont.bin");
        let mb = macbinary::parse(&data)?;
        let suitcase = mb
            .resource_fork()?
            .map(|fork| Suitcase { data: fork })
            .unwrap();

        let family = suitcase.font_family()?;

        for font in family.fonts() {
            dbg!(font);
        }

        Ok(())
    }
}
