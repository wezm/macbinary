//! MacBinary Parser
//!
//! ### Specifications:
//!
//! - [MacBinary I](https://web.archive.org/web/20050307030202/http://www.lazerware.com/formats/macbinary/macbinary.html)
//! - [MacBinary II](https://web.archive.org/web/20050305042909/http://www.lazerware.com/formats/macbinary/macbinary_ii.html)
//! - [MacBinary III](https://web.archive.org/web/20050305044255/http://www.lazerware.com/formats/macbinary/macbinary_iii.html)
//!
//! #### Other references:
//!
//! - [Detecting MacBinary format](https://entropymine.wordpress.com/2019/02/13/detecting-macbinary-format/)

// TODO
// - no_std/WASM
// - zero-copy, ttf-parser style

use core::fmt::{self, Display, Formatter};
use crc::{Crc, CRC_16_XMODEM};

use crate::binary::read::{ReadBinary, ReadBinaryDep, ReadCtxt, ReadFrom, ReadScope};
use crate::binary::{NumFrom, U32Be};
use crate::macroman::FromMacRoman;

pub(crate) mod binary;
pub(crate) mod error;
mod macroman;
mod resource;
#[cfg(test)]
mod test;

const MBIN_SIG: u32 = u32::from_be_bytes(*b"mBIN");

pub use crate::error::ParseError;
pub use crate::resource::ResourceFork;

/// A four-character code
///
/// A 32-bit number that typically holds 4 8-bit ASCII characters, used for type and creator
/// codes, and resource types. Eg. 'mBIN' 'SIZE' 'ICON' 'APPL'.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct FourCC(pub u32);

/// A parsed MacBinary file containing metadata, data fork (if present), and resource fork (if present)
pub struct MacBinary<'a> {
    version: Version,
    header: Header<'a>,
    data_fork: &'a [u8],
    rsrc_fork: &'a [u8],
}

/// MacBinary header
struct Header<'a> {
    filename: &'a [u8],
    secondary_header_len: u16,
    data_fork_len: u32,
    rsrc_fork_len: u32,
    file_type: FourCC,
    file_creator: FourCC,
    finder_flags: u8,
    vpos: u16,
    hpos: u16,
    window_or_folder_id: u16,
    protected: bool,
    created: u32,
    modified: u32,
    comment_len: u16,
    finder_flags2: u8,
    signature: FourCC,
    /// Script of file name (from the `fdScript` field of an `fxInfo` record). since: MacBinary III
    ///
    /// > The script system for displaying the file’s name. Ordinarily, the
    /// > Finder (and the Standard File Package) displays the names of all
    /// > desktop objects in the system script, which depends on the
    /// > region-specific configuration of the system. The high bit of the byte
    /// > in the `fdScript` field is set by default to 0, which causes the Finder
    /// > to display the filename in the current system script. If the high bit is
    /// > set to 1, the Finder (and the Standard File Package) displays the
    /// > filename and directory name in the script whose code is recorded in
    /// > the remaining 7 bits.
    ///
    /// https://developer.apple.com/library/archive/documentation/mac/pdf/MacintoshToolboxEssentials.pdf
    script: u8,
    extended_finder_flags: u8,
    version: u8,
    min_version: u8,
    crc: u16,
}

/// MacBinary version.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone)]
pub enum Version {
    I = 1,
    II = 2,
    III = 3,
}

/// Determine if the supplied data looks like MacBinary data.
pub fn detect(data: &[u8]) -> Option<Version> {
    // All MacBinary files start with a 128-byte header and the first byte is zero
    (data.len() >= 128 && data[0] == 0).then_some(())?;

    // To determine if a header is a valid MacBinary header, first take advantage of the new MacBinary III signature located at offset 102
    if ReadScope::new(&data[102..][..4]).read::<FourCC>() == Ok(FourCC(MBIN_SIG)) {
        return Some(Version::III);
    }

    // If it is not a MacBinary III header, start by checking bytes 0 and 74 - they should both be zero. If they are both zero, either (a) the CRC should match, which means it is a MacBinary II file, or (b) byte 82 is zero, which means it may be a MacBinary I file.
    if data[74] != 0 || data[82] != 0 {
        return None;
    }

    let crc = u16::from_be_bytes(data[124..][..2].try_into().unwrap());
    if crc == calc_crc(&data[..124]) {
        return Some(Version::II);
    }

    // Check for MacBinary I
    // Offsets 101-125, Byte, should all be 0.
    // Offset 2, Byte, (the length of the file name) should be in the range of 1-63.
    // Offsets 83 and 87, Long Word, (the length of the forks) should be in the range of 0-$007F FFFF.
    let data_fork_len = u32::from_be_bytes(data[83..][..4].try_into().unwrap());
    let rsrc_fork_len = u32::from_be_bytes(data[87..][..4].try_into().unwrap());
    let macbinary1 = data[101..=125].iter().all(|byte| *byte == 0)
        && (1..=63).contains(&data[2])
        && data_fork_len <= 0x007F_FFFF
        && rsrc_fork_len <= 0x007F_FFFF;

    if macbinary1 {
        Some(Version::I)
    } else {
        None
    }
}

/// Parse a MacBinary encoded file.
pub fn parse(data: &[u8]) -> Result<MacBinary<'_>, ParseError> {
    let Some(version) = detect(data) else {
        return Err(ParseError::BadVersion) // FIXME: Better error type
    };
    ReadScope::new(data).read_dep::<MacBinary<'_>>(version)
}

impl ReadBinary for Header<'_> {
    type HostType<'a> = Header<'a>;

    fn read<'a>(ctxt: &mut ReadCtxt<'a>) -> Result<Self::HostType<'a>, ParseError> {
        // old version number, must be kept at zero for compatibility
        let _ = ctxt.read_u8()?;
        // Length of filename (must be in the range 1-31)
        let filename_len = ctxt.read_u8()?;
        ctxt.check((1..=31).contains(&filename_len))?; // TODO: 1-63?
                                                       // filename (only "length" bytes are significant).
        let filename_data = ctxt.read_slice(63)?;
        // file type (normally expressed as four characters)
        let file_type = ctxt.read::<FourCC>()?;
        // file creator (normally expressed as four characters)
        let file_creator = ctxt.read::<FourCC>()?;
        // original Finder flags Bit 7 - isAlias. Bit 6 - isInvisible. Bit 5 - hasBundle. Bit 4 - nameLocked. Bit 3 - isStationery. Bit 2 - hasCustomIcon. Bit 1 - reserved. Bit 0 - hasBeenInited.
        let finder_flags = ctxt.read_u8()?;
        // zero fill, must be zero for compatibility
        let _ = ctxt.read_u8()?;
        // file's vertical position within its window.
        let vpos = ctxt.read_u16be()?;
        // file's horizontal position within its window.
        let hpos = ctxt.read_u16be()?;
        // file's window or folder ID.
        let window_or_folder_id = ctxt.read_u16be()?;
        // "Protected" flag (in low order bit).
        let protected = ctxt.read_u8()?;
        // zero fill, must be zero for compatibility
        let _ = ctxt.read_u8()?;
        // Data Fork length (bytes, zero if no Data Fork).
        let data_fork_len = ctxt.read_u32be()?;
        // Resource Fork length (bytes, zero if no R.F.).
        let rsrc_fork_len = ctxt.read_u32be()?;
        // File's creation date
        let created = ctxt.read_u32be()?;
        // File's "last modified" date.
        let modified = ctxt.read_u32be()?;
        // length of Get Info comment to be sent after the resource fork (if implemented, see below).
        let comment_len = ctxt.read_u16be()?;
        // Finder Flags, bits 0-7. (Bits 8-15 are already in byte 73) Bit 7 - hasNoInits Bit 6 - isShared Bit 5 - requiresSwitchLaunch Bit 4 - ColorReserved Bits 1-3 - color Bit 0 - isOnDesk
        let finder_flags2 = ctxt.read_u8()?;
        // signature for identification purposes ('mBIN')
        let signature = ctxt.read::<FourCC>()?;
        // script of file name (from the fdScript field of an fxInfo record)
        let script = ctxt.read_u8()?;
        // extended Finder flags (from the fdXFlags field of an fxInfo record)
        let extended_finder_flags = ctxt.read_u8()?;
        // Bytes 108-115 unused (must be zeroed by creators, must be ignored by readers)
        let _ = ctxt.read_slice(8)?;
        // Length of total files when packed files are unpacked. As of the writing of this document, this field has never been used.
        let _ = ctxt.read_u32be()?;
        // Length of a secondary header. If this is non-zero, skip this many bytes (rounded up to the next multiple of 128). This is for future expansion only, when sending files with MacBinary, this word should be zero.
        let secondary_header_len = ctxt.read_u16be()?;
        // Version number of MacBinary III that the uploading program is written for (the version is 130 for MacBinary III)
        let version = ctxt.read_u8()?;
        // Minimum MacBinary version needed to read this file (set this value at 129 for backwards compatibility with MacBinary II)
        // field: u8,
        let min_version = ctxt.read_u8()?;
        // CRC of previous 124 bytes
        let crc = ctxt.read_u16be()?;
        // Reserved for computer type and OS ID (this field will be zero for the current Macintosh).
        let _ = ctxt.read_u16be()?;

        Ok(Header {
            filename: &filename_data[..usize::from(filename_len)],
            file_type,
            file_creator,
            finder_flags,
            vpos,
            hpos,
            window_or_folder_id,
            protected: protected != 0,
            data_fork_len,
            rsrc_fork_len,
            created,
            modified,
            comment_len,
            finder_flags2,
            signature,
            script,
            extended_finder_flags,
            secondary_header_len,
            version,
            min_version,
            crc,
        })
    }
}

impl ReadBinaryDep for MacBinary<'_> {
    type Args<'a> = Version;
    type HostType<'a> = MacBinary<'a>;

    fn read_dep<'a>(
        ctxt: &mut ReadCtxt<'a>,
        version: Version,
    ) -> Result<Self::HostType<'a>, ParseError> {
        let crc_data = ctxt.scope().data().get(..124).ok_or(ParseError::BadEof)?;

        // The binary format consists of a 128-byte header containing all the information necessary
        // to reproduce the document's directory entry on the receiving Macintosh; followed by the
        // document's Data Fork (if it has one), padded with nulls to a multiple of 128 bytes (if
        // necessary); followed by the document's Resource Fork (again, padded if necessary). The
        // lengths of these forks (either or both of which may be zero) are contained in the
        // header.
        let header = ctxt.read::<Header<'_>>()?;

        // Check the CRC
        let crc = calc_crc(crc_data);
        if version >= Version::II && crc != header.crc {
            return Err(ParseError::CrcMismatch);
        }

        // Skip secondary header if present, rounding up to next multiple of 128
        let _ = ctxt.read_slice(usize::from(next_u16_multiple_of_128(
            header.secondary_header_len,
        )?))?;

        // Read the data fork
        let data_fork = ctxt.read_slice(usize::num_from(header.data_fork_len))?;

        // Skip padding
        let padding = next_u32_multiple_of_128(header.data_fork_len)? - header.data_fork_len;
        let _ = ctxt.read_slice(usize::num_from(padding))?;

        // Read the resource fork
        let rsrc_fork = ctxt.read_slice(usize::num_from(header.rsrc_fork_len))?;

        Ok(MacBinary {
            version,
            header,
            data_fork,
            rsrc_fork,
        })
    }
}

impl MacBinary<'_> {
    pub fn version(&self) -> Version {
        self.version
    }

    pub fn filename(&self) -> String {
        // For the purposes of this library we consider the system script to be Mac Roman.
        // The script field can indicate a different script if the high-bit is set though.
        // If the high-bit is set but the remaining 7-bits are zero that means it's still
        // MacRoman.
        if self.header.script & 0x80 == 0x80 && self.header.script & !0x80 != 0 {
            todo!("Handle non-macroman script")
        } else {
            String::from_macroman(self.header.filename)
        }
    }

    /// The file's creator code
    pub fn file_creator(&self) -> FourCC {
        self.header.file_creator
    }

    /// The file's type code
    pub fn file_type(&self) -> FourCC {
        self.header.file_type
    }

    /// File creation date (UNIX timestamp)
    pub fn created(&self) -> u32 {
        mactime(self.header.created)
    }

    /// File last modified date (UNIX timestamp)
    pub fn modified(&self) -> u32 {
        mactime(self.header.modified)
    }

    /// Data fork data
    pub fn data_fork(&self) -> &[u8] {
        self.data_fork
    }

    /// Resource fork data
    pub fn resource_fork_raw(&self) -> &[u8] {
        self.rsrc_fork
    }

    /// Parsed resource fork
    pub fn resource_fork(&self) -> Result<ResourceFork<'_>, ParseError> {
        ResourceFork::new(self.rsrc_fork)
    }
}

impl ReadFrom for FourCC {
    type ReadType = U32Be;

    fn from(value: u32) -> Self {
        FourCC(value)
    }
}

impl Display for FourCC {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let tag = self.0;
        let bytes = tag.to_be_bytes();
        if bytes.iter().all(|c| c.is_ascii() && !c.is_ascii_control()) {
            let s = core::str::from_utf8(&bytes).unwrap(); // unwrap safe due to above check
            s.fmt(f)
        } else {
            write!(f, "0x{:08x}", tag)
        }
    }
}

fn next_u16_multiple_of_128(value: u16) -> Result<u16, ParseError> {
    let rem = value % 128;
    if rem == 0 {
        Ok(value)
    } else {
        value.checked_add(128 - rem).ok_or(ParseError::Overflow)
    }
}

fn next_u32_multiple_of_128(value: u32) -> Result<u32, ParseError> {
    let rem = value % 128;
    if rem == 0 {
        Ok(value)
    } else {
        value.checked_add(128 - rem).ok_or(ParseError::Overflow)
    }
}

/// Convert Mac OS timestamp to UNIX timestamp
///
/// The Mac OS epoch is 1 January 1904, UNIX epoch is 1 Jan 1970.
fn mactime(timestamp: u32) -> u32 {
    // 66 years from 1904 to 1970, 18 leap years, 86400 seconds in a day
    const OFFSET: u32 = 66 * 365 * 86400 + (18 * 86400);
    timestamp.wrapping_add(OFFSET)
}

fn calc_crc(data: &[u8]) -> u16 {
    let crc: Crc<u16> = Crc::<u16>::new(&CRC_16_XMODEM);
    crc.checksum(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::read_fixture;

    #[test]
    fn test_next_multiple() {
        assert_eq!(next_u16_multiple_of_128(0), Ok(0));
        assert_eq!(next_u16_multiple_of_128(3), Ok(128));
        assert_eq!(next_u16_multiple_of_128(128), Ok(128));
        assert_eq!(next_u16_multiple_of_128(129), Ok(256));

        assert_eq!(next_u32_multiple_of_128(0), Ok(0));
        assert_eq!(next_u32_multiple_of_128(3), Ok(128));
        assert_eq!(next_u32_multiple_of_128(128), Ok(128));
        assert_eq!(next_u32_multiple_of_128(129), Ok(256));
    }

    #[test]
    fn test_next_multiple_overflow() {
        assert_eq!(
            next_u16_multiple_of_128(u16::MAX - 3),
            Err(ParseError::Overflow)
        );
        assert_eq!(
            next_u32_multiple_of_128(u32::MAX - 3),
            Err(ParseError::Overflow)
        );
    }

    #[test]
    fn test_macbinary_3() {
        let data = read_fixture("tests/Text File.bin");
        let file = parse(&data).unwrap();

        assert_eq!(file.filename(), "Text File");
        assert_eq!(file.file_type(), FourCC(u32::from_be_bytes(*b"TEXT")));
        assert_eq!(file.file_creator(), FourCC(u32::from_be_bytes(*b"R*ch"))); // BBEdit
        assert_eq!(file.data_fork(), b"This is a test file.\r");
        assert_eq!(file.resource_fork_raw().len(), 1454);
    }
}
