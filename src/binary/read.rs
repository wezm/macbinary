//! Parse binary data
//!
//! The is module was extracted from Allsorts. The parsing approach is inspired by the paper,
//! [The next 700 data description languages](https://collaborate.princeton.edu/en/publications/the-next-700-data-description-languages) by Kathleen Fisher, Yitzhak Mandelbaum, David P. Walker.

use core::cmp;
use core::fmt;
use core::marker::PhantomData;

use super::size;
use crate::binary::{I16Be, I32Be, I64Be, U16Be, U24Be, U32Be, I8, U8};
use crate::error::ParseError;

#[derive(Debug, Copy, Clone)]
pub struct ReadEof {}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct ReadScope<'a> {
    base: usize,
    data: &'a [u8],
}

#[derive(Clone)]
pub struct ReadCtxt<'a> {
    scope: ReadScope<'a>,
    offset: usize,
}

pub trait ReadBinary {
    type HostType<'a>: Sized; // default = Self

    fn read<'a>(ctxt: &mut ReadCtxt<'a>) -> Result<Self::HostType<'a>, ParseError>;
}

pub trait ReadBinaryDep {
    type Args<'a>: Copy;
    type HostType<'a>: Sized; // default = Self

    fn read_dep<'a>(
        ctxt: &mut ReadCtxt<'a>,
        args: Self::Args<'a>,
    ) -> Result<Self::HostType<'a>, ParseError>;
}

pub trait ReadFixedSizeDep: ReadBinaryDep {
    /// The number of bytes consumed by `ReadBinaryDep::read`.
    fn size(args: Self::Args<'_>) -> usize;
}

/// Read will always succeed if sufficient bytes are available.
pub trait ReadUnchecked {
    type HostType: Sized; // default = Self

    /// The number of bytes consumed by `read_unchecked`.
    const SIZE: usize;

    /// Must read exactly `SIZE` bytes.
    /// Unsafe as it avoids prohibitively expensive per-byte bounds checking.
    unsafe fn read_unchecked<'a>(ctxt: &mut ReadCtxt<'a>) -> Self::HostType;
}

pub trait ReadFrom {
    type ReadType: ReadUnchecked;
    fn from(value: <Self::ReadType as ReadUnchecked>::HostType) -> Self;
}

impl<T> ReadUnchecked for T
where
    T: ReadFrom,
{
    type HostType = T;

    const SIZE: usize = T::ReadType::SIZE;

    unsafe fn read_unchecked<'a>(ctxt: &mut ReadCtxt<'a>) -> Self::HostType {
        let t = T::ReadType::read_unchecked(ctxt);
        T::from(t)
    }
}

impl<T> ReadBinary for T
where
    T: ReadUnchecked,
{
    type HostType<'a> = T::HostType;

    fn read<'a>(ctxt: &mut ReadCtxt<'a>) -> Result<Self::HostType<'a>, ParseError> {
        ctxt.check_avail(T::SIZE)?;
        Ok(unsafe { T::read_unchecked(ctxt) })
        // Safe because we have `SIZE` bytes available.
    }
}

impl<T> ReadBinaryDep for T
where
    T: ReadBinary,
{
    type Args<'a> = ();
    type HostType<'a> = T::HostType<'a>;

    fn read_dep<'a>(
        ctxt: &mut ReadCtxt<'a>,
        (): Self::Args<'_>,
    ) -> Result<Self::HostType<'a>, ParseError> {
        T::read(ctxt)
    }
}

impl<T> ReadFixedSizeDep for T
where
    T: ReadUnchecked,
{
    fn size((): ()) -> usize {
        T::SIZE
    }
}

pub trait CheckIndex {
    fn check_index(&self, index: usize) -> Result<(), ParseError>;
}

#[derive(Clone)]
pub struct ReadArray<'a, T: ReadFixedSizeDep> {
    scope: ReadScope<'a>,
    length: usize,
    args: T::Args<'a>,
}

pub struct ReadArrayIter<'a, T: ReadUnchecked> {
    ctxt: ReadCtxt<'a>,
    length: usize,
    phantom: PhantomData<T>,
}

pub struct ReadArrayDepIter<'a, 'b, T: ReadFixedSizeDep> {
    array: &'b ReadArray<'a, T>,
    index: usize,
}

impl<'a> ReadScope<'a> {
    pub fn new(data: &'a [u8]) -> ReadScope<'a> {
        let base = 0;
        ReadScope { base, data }
    }

    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    pub fn offset(&self, offset: usize) -> ReadScope<'a> {
        let base = self.base + offset;
        let data = self.data.get(offset..).unwrap_or(&[]);
        ReadScope { base, data }
    }

    pub fn offset_length(&self, offset: usize, length: usize) -> Result<ReadScope<'a>, ParseError> {
        if offset < self.data.len() || length == 0 {
            let data = self.data.get(offset..).unwrap_or(&[]);
            if length <= data.len() {
                let base = self.base + offset;
                let data = &data[0..length];
                Ok(ReadScope { base, data })
            } else {
                Err(ParseError::BadEof)
            }
        } else {
            Err(ParseError::BadOffset)
        }
    }

    pub fn ctxt(&self) -> ReadCtxt<'a> {
        ReadCtxt::new(self.clone())
    }

    pub fn read<T: ReadBinaryDep<Args<'a> = ()>>(&self) -> Result<T::HostType<'a>, ParseError> {
        self.ctxt().read::<T>()
    }

    pub fn read_dep<T: ReadBinaryDep>(
        &self,
        args: T::Args<'a>,
    ) -> Result<T::HostType<'a>, ParseError> {
        self.ctxt().read_dep::<T>(args)
    }
}

impl<'a> ReadCtxt<'a> {
    /// ReadCtxt is constructed by calling `ReadScope::ctxt`.
    fn new(scope: ReadScope<'a>) -> ReadCtxt<'a> {
        ReadCtxt { scope, offset: 0 }
    }

    pub fn check(&self, cond: bool) -> Result<(), ParseError> {
        match cond {
            true => Ok(()),
            false => Err(ParseError::BadValue),
        }
    }

    /// Check a condition, returning `ParseError::BadVersion` if `false`.
    ///
    /// Intended for use in checking versions read from data. Example:
    pub fn check_version(&self, cond: bool) -> Result<(), ParseError> {
        match cond {
            true => Ok(()),
            false => Err(ParseError::BadVersion),
        }
    }

    pub fn scope(&self) -> ReadScope<'a> {
        self.scope.offset(self.offset)
    }

    pub fn read<T: ReadBinaryDep<Args<'a> = ()>>(&mut self) -> Result<T::HostType<'a>, ParseError> {
        T::read_dep(self, ())
    }

    pub fn read_dep<T: ReadBinaryDep>(
        &mut self,
        args: T::Args<'a>,
    ) -> Result<T::HostType<'a>, ParseError> {
        T::read_dep(self, args)
    }

    pub fn bytes_available(&self) -> bool {
        self.offset < self.scope.data.len()
    }

    fn check_avail(&self, length: usize) -> Result<(), ReadEof> {
        match self.offset.checked_add(length) {
            Some(endpos) if endpos <= self.scope.data.len() => Ok(()),
            _ => Err(ReadEof {}),
        }
    }

    unsafe fn read_unchecked_u8(&mut self) -> u8 {
        let byte = *self.scope.data.get_unchecked(self.offset);
        self.offset += 1;
        byte
    }

    unsafe fn read_unchecked_i8(&mut self) -> i8 {
        self.read_unchecked_u8() as i8
    }

    unsafe fn read_unchecked_u16be(&mut self) -> u16 {
        let hi = u16::from(*self.scope.data.get_unchecked(self.offset));
        let lo = u16::from(*self.scope.data.get_unchecked(self.offset + 1));
        self.offset += 2;
        (hi << 8) | lo
    }

    unsafe fn read_unchecked_i16be(&mut self) -> i16 {
        self.read_unchecked_u16be() as i16
    }

    unsafe fn read_unchecked_u24be(&mut self) -> u32 {
        let b0 = u32::from(*self.scope.data.get_unchecked(self.offset));
        let b1 = u32::from(*self.scope.data.get_unchecked(self.offset + 1));
        let b2 = u32::from(*self.scope.data.get_unchecked(self.offset + 2));
        self.offset += 3;
        (b0 << 16) | (b1 << 8) | b2
    }

    unsafe fn read_unchecked_u32be(&mut self) -> u32 {
        let b0 = u32::from(*self.scope.data.get_unchecked(self.offset));
        let b1 = u32::from(*self.scope.data.get_unchecked(self.offset + 1));
        let b2 = u32::from(*self.scope.data.get_unchecked(self.offset + 2));
        let b3 = u32::from(*self.scope.data.get_unchecked(self.offset + 3));
        self.offset += 4;
        (b0 << 24) | (b1 << 16) | (b2 << 8) | b3
    }

    unsafe fn read_unchecked_i32be(&mut self) -> i32 {
        self.read_unchecked_u32be() as i32
    }

    unsafe fn read_unchecked_u64be(&mut self) -> u64 {
        let hi = u64::from(self.read_unchecked_u32be());
        let lo = u64::from(self.read_unchecked_u32be());
        (hi << 32) | lo
    }

    unsafe fn read_unchecked_i64be(&mut self) -> i64 {
        self.read_unchecked_u64be() as i64
    }

    pub fn read_u8(&mut self) -> Result<u8, ReadEof> {
        self.check_avail(1)?;
        Ok(unsafe { self.read_unchecked_u8() })
        // Safe because we have 1 byte available.
    }

    pub fn read_i8(&mut self) -> Result<i8, ReadEof> {
        self.check_avail(1)?;
        Ok(unsafe { self.read_unchecked_i8() })
        // Safe because we have 1 byte available.
    }

    pub fn read_u16be(&mut self) -> Result<u16, ReadEof> {
        self.check_avail(2)?;
        Ok(unsafe { self.read_unchecked_u16be() })
        // Safe because we have 2 bytes available.
    }

    pub fn read_i16be(&mut self) -> Result<i16, ReadEof> {
        self.check_avail(2)?;
        Ok(unsafe { self.read_unchecked_i16be() })
        // Safe because we have 2 bytes available.
    }

    pub fn read_u32be(&mut self) -> Result<u32, ReadEof> {
        self.check_avail(4)?;
        Ok(unsafe { self.read_unchecked_u32be() })
        // Safe because we have 4 bytes available.
    }

    pub fn read_i32be(&mut self) -> Result<i32, ReadEof> {
        self.check_avail(4)?;
        Ok(unsafe { self.read_unchecked_i32be() })
        // Safe because we have 4 bytes available.
    }

    pub fn read_u64be(&mut self) -> Result<u64, ReadEof> {
        self.check_avail(8)?;
        Ok(unsafe { self.read_unchecked_u64be() })
        // Safe because we have 8 bytes available.
    }

    pub fn read_i64be(&mut self) -> Result<i64, ReadEof> {
        self.check_avail(8)?;
        Ok(unsafe { self.read_unchecked_i64be() })
        // Safe because we have 8 bytes available.
    }

    pub fn read_array<T: ReadUnchecked>(
        &mut self,
        length: usize,
    ) -> Result<ReadArray<'a, T>, ParseError> {
        let scope = self.read_scope(length * T::SIZE)?;
        let args = ();
        Ok(ReadArray {
            scope,
            length,
            args,
        })
    }

    pub fn read_array_upto_hack<T: ReadUnchecked>(
        &mut self,
        length: usize,
    ) -> Result<ReadArray<'a, T>, ParseError> {
        let start_pos = self.offset;
        let buf_size = self.scope.data.len();
        let avail_bytes = cmp::max(0, buf_size - start_pos);
        let max_length = avail_bytes / T::SIZE;
        let length = cmp::min(length, max_length);
        self.read_array(length)
    }

    /// Read up to and including the supplied nibble.
    pub fn read_until_nibble(&mut self, nibble: u8) -> Result<&'a [u8], ReadEof> {
        let end = self.scope.data[self.offset..]
            .iter()
            .position(|&b| (b >> 4) == nibble || (b & 0xF) == nibble)
            .ok_or(ReadEof {})?;
        self.read_slice(end + 1)
    }

    pub fn read_array_dep<T: ReadFixedSizeDep>(
        &mut self,
        length: usize,
        args: T::Args<'a>,
    ) -> Result<ReadArray<'a, T>, ParseError> {
        let scope = self.read_scope(length * T::size(args))?;
        Ok(ReadArray {
            scope,
            length,
            args: args,
        })
    }

    pub fn read_scope(&mut self, length: usize) -> Result<ReadScope<'a>, ReadEof> {
        if let Ok(scope) = self.scope.offset_length(self.offset, length) {
            self.offset += length;
            Ok(scope)
        } else {
            Err(ReadEof {})
        }
    }

    pub fn read_slice(&mut self, length: usize) -> Result<&'a [u8], ReadEof> {
        let scope = self.read_scope(length)?;
        Ok(scope.data)
    }
}

impl<'a, T: ReadFixedSizeDep> ReadArray<'a, T> {
    pub fn len(&self) -> usize {
        self.length
    }

    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub fn read_item(&self, index: usize) -> Result<T::HostType<'a>, ParseError> {
        if index < self.length {
            let size = T::size(self.args);
            let offset = index * size;
            let scope = self.scope.offset_length(offset, size).unwrap();
            let mut ctxt = scope.ctxt();
            T::read_dep(&mut ctxt, self.args)
        } else {
            panic!("ReadArray::read_item: index out of bounds");
        }
    }

    pub fn get_item(&self, index: usize) -> <T as ReadUnchecked>::HostType
    where
        T: ReadUnchecked,
    {
        if index < self.length {
            let offset = index * T::SIZE;
            let scope = self.scope.offset_length(offset, T::SIZE).unwrap();
            let mut ctxt = scope.ctxt();
            unsafe { T::read_unchecked(&mut ctxt) } // Safe because we have `SIZE` bytes available.
        } else {
            panic!("ReadArray::get_item: index out of bounds");
        }
    }

    pub fn subarray(&self, index: usize) -> Self {
        if index < self.length {
            let offset = index * T::size(self.args);
            ReadArray {
                scope: self.scope.offset(offset),
                length: self.length - index,
                args: self.args,
            }
        } else {
            ReadArray {
                scope: ReadScope::new(&[]),
                length: 0,
                args: self.args,
            }
        }
    }

    pub fn iter(&self) -> ReadArrayIter<'a, T>
    where
        T: ReadUnchecked,
    {
        ReadArrayIter {
            ctxt: self.scope.ctxt(),
            length: self.length,
            phantom: PhantomData,
        }
    }

    pub fn iter_res<'b>(&'b self) -> ReadArrayDepIter<'a, 'b, T> {
        ReadArrayDepIter {
            array: self,
            index: 0,
        }
    }
}

impl<'a, T: ReadFixedSizeDep> CheckIndex for ReadArray<'a, T> {
    fn check_index(&self, index: usize) -> Result<(), ParseError> {
        if index < self.len() {
            Ok(())
        } else {
            Err(ParseError::BadIndex)
        }
    }
}

impl<'a, 'b, T: ReadUnchecked> IntoIterator for &'b ReadArray<'a, T> {
    type Item = T::HostType;
    type IntoIter = ReadArrayIter<'a, T>;
    fn into_iter(self) -> ReadArrayIter<'a, T> {
        self.iter()
    }
}

impl<'a, T: ReadUnchecked> Iterator for ReadArrayIter<'a, T> {
    type Item = T::HostType;

    fn next(&mut self) -> Option<T::HostType> {
        if self.length > 0 {
            self.length -= 1;
            // Safe because we have (at least) `SIZE` bytes available.
            Some(unsafe { T::read_unchecked(&mut self.ctxt) })
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.length, Some(self.length))
    }
}

impl<'a, T: ReadUnchecked> ExactSizeIterator for ReadArrayIter<'a, T> {}

impl<'a, 'b, T: ReadFixedSizeDep> Iterator for ReadArrayDepIter<'a, 'b, T> {
    type Item = Result<T::HostType<'a>, ParseError>;

    fn next(&mut self) -> Option<Result<T::HostType<'a>, ParseError>> {
        if self.index < self.array.len() {
            let result = self.array.read_item(self.index);
            self.index += 1;
            Some(result)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.index < self.array.len() {
            let length = self.array.len() - self.index;
            (length, Some(length))
        } else {
            (0, Some(0))
        }
    }
}

impl<'a, T: ReadUnchecked> ReadArray<'a, T> {
    pub fn empty() -> ReadArray<'a, T> {
        ReadArray {
            scope: ReadScope::new(&[]),
            length: 0,
            args: (),
        }
    }
}

impl ReadUnchecked for U8 {
    type HostType = u8;

    const SIZE: usize = size::U8;

    unsafe fn read_unchecked<'a>(ctxt: &mut ReadCtxt<'a>) -> u8 {
        ctxt.read_unchecked_u8()
    }
}

impl ReadUnchecked for I8 {
    type HostType = i8;

    const SIZE: usize = size::I8;

    unsafe fn read_unchecked<'a>(ctxt: &mut ReadCtxt<'a>) -> i8 {
        ctxt.read_unchecked_i8()
    }
}

impl ReadUnchecked for U16Be {
    type HostType = u16;

    const SIZE: usize = size::U16;

    unsafe fn read_unchecked<'a>(ctxt: &mut ReadCtxt<'a>) -> u16 {
        ctxt.read_unchecked_u16be()
    }
}

impl ReadUnchecked for I16Be {
    type HostType = i16;

    const SIZE: usize = size::I16;

    unsafe fn read_unchecked<'a>(ctxt: &mut ReadCtxt<'a>) -> i16 {
        ctxt.read_unchecked_i16be()
    }
}

impl ReadUnchecked for U24Be {
    type HostType = u32;

    const SIZE: usize = size::U24;

    unsafe fn read_unchecked<'a>(ctxt: &mut ReadCtxt<'a>) -> u32 {
        ctxt.read_unchecked_u24be()
    }
}

impl ReadUnchecked for U32Be {
    type HostType = u32;

    const SIZE: usize = size::U32;

    unsafe fn read_unchecked<'a>(ctxt: &mut ReadCtxt<'a>) -> u32 {
        ctxt.read_unchecked_u32be()
    }
}

impl ReadUnchecked for I32Be {
    type HostType = i32;

    const SIZE: usize = size::I32;

    unsafe fn read_unchecked<'a>(ctxt: &mut ReadCtxt<'a>) -> i32 {
        ctxt.read_unchecked_i32be()
    }
}

impl ReadUnchecked for I64Be {
    type HostType = i64;

    const SIZE: usize = size::I64;

    unsafe fn read_unchecked<'a>(ctxt: &mut ReadCtxt<'a>) -> i64 {
        ctxt.read_unchecked_i64be()
    }
}

impl<T1, T2> ReadUnchecked for (T1, T2)
where
    T1: ReadUnchecked,
    T2: ReadUnchecked,
{
    type HostType = (T1::HostType, T2::HostType);

    const SIZE: usize = T1::SIZE + T2::SIZE;

    unsafe fn read_unchecked<'a>(ctxt: &mut ReadCtxt<'a>) -> Self::HostType {
        let t1 = T1::read_unchecked(ctxt);
        let t2 = T2::read_unchecked(ctxt);
        (t1, t2)
    }
}

impl<T1, T2, T3> ReadUnchecked for (T1, T2, T3)
where
    T1: ReadUnchecked,
    T2: ReadUnchecked,
    T3: ReadUnchecked,
{
    type HostType = (T1::HostType, T2::HostType, T3::HostType);

    const SIZE: usize = T1::SIZE + T2::SIZE + T3::SIZE;

    unsafe fn read_unchecked<'a>(ctxt: &mut ReadCtxt<'a>) -> Self::HostType {
        let t1 = T1::read_unchecked(ctxt);
        let t2 = T2::read_unchecked(ctxt);
        let t3 = T3::read_unchecked(ctxt);
        (t1, t2, t3)
    }
}

impl<'a, T> fmt::Debug for ReadArray<'a, T>
where
    T: ReadUnchecked,
    <T as ReadUnchecked>::HostType: Copy + fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        f.debug_list().entries(self.iter()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_u24be() {
        let scope = ReadScope::new(&[1, 2, 3]);
        assert_eq!(scope.read::<U24Be>().unwrap(), 0x10203);
    }

    // Tests that offset_length does not panic when length is 0 but offset is out-of-bounds
    #[test]
    fn test_offset_length_oob() {
        let scope = ReadScope::new(&[1, 2, 3]);
        assert!(scope.offset_length(99, 0).is_ok());
    }
}
