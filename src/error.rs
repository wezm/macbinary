//! Error types

use core::fmt;

use crate::binary::read::ReadEof;

/// Errors that originate when parsing binary data
#[derive(Clone, Eq, PartialEq, Debug)]
pub enum ParseError {
    /// EOF was reached unexpectedly
    BadEof,
    /// A value was outside the expected range
    BadValue,
    /// A version field contained an unsupported version
    BadVersion,
    /// An offset was outside allowed bounds
    BadOffset,
    /// An index was outside the valid range
    BadIndex,
    /// A value overflowed its storage type
    Overflow,
    /// CRC did not match expected value
    CrcMismatch,
}

impl From<ReadEof> for ParseError {
    fn from(_error: ReadEof) -> Self {
        ParseError::BadEof
    }
}

impl From<core::num::TryFromIntError> for ParseError {
    fn from(_error: core::num::TryFromIntError) -> Self {
        ParseError::BadValue
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::BadEof => write!(f, "end of data reached unexpectedly"),
            ParseError::BadValue => write!(f, "invalid value"),
            ParseError::BadVersion => write!(f, "unexpected data version"),
            ParseError::BadOffset => write!(f, "invalid data offset"),
            ParseError::BadIndex => write!(f, "invalid data index"),
            ParseError::Overflow => write!(f, "a value overflowed its range"),
            ParseError::CrcMismatch => write!(f, "CRC mismatch"),
        }
    }
}

// FIXME: Enable on no_std when https://github.com/rust-lang/rust/issues/103765 is stable
#[cfg(not(feature = "no_std"))]
impl std::error::Error for ParseError {}
