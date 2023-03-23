#![deny(missing_docs)]

//! Reading and writing of binary data.

pub(crate) mod read;
// pub mod write;

/// Unsigned 8-bit binary type.
#[derive(Copy, Clone)]
pub enum U8 {}

/// Signed 8-bit binary type.
#[derive(Copy, Clone)]
pub enum I8 {}

/// Unsigned 16-bit big endian binary type.
#[derive(Copy, Clone)]
pub enum U16Be {}

/// Signed 16-bit big endian binary type.
#[derive(Copy, Clone)]
pub enum I16Be {}

/// Unsigned 24-bit (3 bytes) big endian binary type.
#[derive(Copy, Clone)]
pub enum U24Be {}

/// Unsigned 32-bit big endian binary type.
#[derive(Copy, Clone)]
pub enum U32Be {}

/// Signed 32-bit big endian binary type.
#[derive(Copy, Clone)]
pub enum I32Be {}

/// Signed 64-bit binary type.
#[derive(Copy, Clone)]
pub enum I64Be {}

/// A safe u32 to usize casting.
///
/// Rust doesn't implement `From<u32> for usize`,
/// because it has to support 16 bit targets.
/// We don't, so we can allow this.
pub trait NumFrom<T>: Sized {
    /// Converts u32 into usize.
    fn num_from(_: T) -> Self;
}

impl NumFrom<u32> for usize {
    #[inline]
    fn num_from(v: u32) -> Self {
        #[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
        {
            v as usize
        }

        // compilation error on 16 bit targets
    }
}

mod size {
    //! Definitions of the sizes of binary types.

    use core::mem;

    pub const U8: usize = mem::size_of::<u8>();
    pub const I8: usize = mem::size_of::<i8>();
    pub const U16: usize = mem::size_of::<u16>();
    pub const I16: usize = mem::size_of::<i16>();
    pub const U24: usize = 3;
    pub const U32: usize = mem::size_of::<u32>();
    pub const I32: usize = mem::size_of::<i32>();
    pub const I64: usize = mem::size_of::<i64>();
}
