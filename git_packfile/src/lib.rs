#![no_std]
mod std { pub use core::*; }

extern crate void;
#[macro_use]
extern crate gulp;
extern crate byteorder;
extern crate git;

use gulp::{ReadResult, FromBytes, read_tag, read_u8, read_u32, read_le_base128_cont};

#[derive(Copy, Clone, Debug)]
pub struct FileHeader {
  pub count: u32
}

#[derive(Copy, Clone, Debug)]
pub struct InvalidFileHeader(());

impl FromBytes for FileHeader {
  type Err = InvalidFileHeader;
  fn from_bytes(buf: &[u8]) -> ReadResult<FileHeader, InvalidFileHeader> {
    let (_, buf) = try_read!(read_tag(buf, b"PACK\x00\x00\x00\x02").map_err(InvalidFileHeader));
    let (count, buf) = try_read_void!(read_u32::<byteorder::NetworkEndian>(buf));
    ReadResult::Ok(FileHeader { count: count }, buf)
  }
}


#[derive(Copy, Clone, Debug)]
pub enum EntryHeader {
  Object(git::ObjectHeader),
  Delta(DeltaHeader)
}

impl From<git::ObjectHeader> for EntryHeader {
  fn from(h: git::ObjectHeader) -> EntryHeader { EntryHeader::Object(h) }
}

impl From<DeltaHeader> for EntryHeader {
  fn from(h: DeltaHeader) -> EntryHeader { EntryHeader::Delta(h) }
}

impl EntryHeader {
  pub fn kind(&self) -> EntryKind {
    match *self {
      EntryHeader::Object(ref h) => From::from(h.kind),
      EntryHeader::Delta(ref h)  => From::from(h.kind())
    }
  }
}

#[derive(Copy, Clone, Debug)]
pub enum EntryKind {
  Object(git::ObjectKind),
  Delta(DeltaKind)
}

impl From<git::ObjectKind> for EntryKind {
  fn from(k: git::ObjectKind) -> EntryKind { EntryKind::Object(k) }
}

impl From<DeltaKind> for EntryKind {
  fn from(k: DeltaKind) -> EntryKind { EntryKind::Delta(k) }
}

impl FromBytes for EntryHeader {
  type Err = InvalidEntryHeader;
  fn from_bytes(buf: &[u8]) -> ReadResult<EntryHeader, InvalidEntryHeader> {
    let (byte, buf) = try_read_void!(read_u8(buf));
    let kind: EntryKind = match (byte>>4) & 7 {
      1 => From::from(git::ObjectKind::Commit),
      2 => From::from(git::ObjectKind::Tree),
      3 => From::from(git::ObjectKind::Blob),
      4 => From::from(git::ObjectKind::Tag),
      6 => From::from(DeltaKind::Offset),
      7 => From::from(DeltaKind::Reference),
      _ => return ReadResult::Err(InvalidEntryHeader(()))
    };
    let size = byte as u64 & 15;
    let (size, buf) = if byte&0x80 != 0 {
      try_read!(read_le_base128_cont(buf, size, 4))
    } else {
      (size, buf)
    };
    match kind {
      EntryKind::Object(kind) => ReadResult::Ok(From::from(git::ObjectHeader { kind: kind, size: size }), buf),
      EntryKind::Delta(kind)  => DeltaHeader::from_bytes(buf, size, kind).map(From::from).map_err(From::from)
    }
  }
}

#[derive(Copy, Clone, Debug)]
pub struct InvalidEntryHeader(());

impl From<gulp::Overflow> for InvalidEntryHeader {
  fn from(_: gulp::Overflow) -> InvalidEntryHeader {
    InvalidEntryHeader(())
  }
}

impl From<InvalidDeltaHeader> for InvalidEntryHeader {
  fn from(_: InvalidDeltaHeader) -> InvalidEntryHeader {
    InvalidEntryHeader(())
  }
}

#[derive(Copy, Clone, Debug)]
pub enum DeltaHeader {
  Offset    { delta_len: u64, base: u64 },
  Reference { delta_len: u64, base: git::ObjectId }
}

#[derive(Copy, Clone, Debug)]
pub enum DeltaKind {
  Offset,
  Reference
}

impl DeltaHeader {
  pub fn kind(&self) -> DeltaKind {
    match *self {
      DeltaHeader::Offset    { .. } => DeltaKind::Offset,
      DeltaHeader::Reference { .. } => DeltaKind::Reference
    }
  }
  pub fn delta_len(&self) -> u64 {
    match *self {
      DeltaHeader::Offset    { delta_len, .. } => delta_len,
      DeltaHeader::Reference { delta_len, .. } => delta_len
    }
  }
}

impl DeltaHeader {
  fn from_bytes(buf: &[u8], delta_len: u64, kind: DeltaKind) -> ReadResult<DeltaHeader, InvalidDeltaHeader> {
    match kind {
      DeltaKind::Offset    => DeltaHeader::offset_from_bytes(buf).map(|base| DeltaHeader::Offset { delta_len: delta_len, base: base }),
      DeltaKind::Reference => FromBytes::from_bytes(buf).map(|base| DeltaHeader::Reference { delta_len: delta_len, base: base }).map_err(|v| match v {})
    }
  }
  fn offset_from_bytes(buf: &[u8]) -> ReadResult<u64, InvalidDeltaHeader> {
    let (b, buf) = try_read_void!(read_u8(buf));
    let mut off = b as u64 & 0x7F;
    if b&0x80 == 0 {
      return ReadResult::Ok(off, buf);
    }
    let mut buf = buf.iter();
    while let Some(&b) = buf.next() {
      off += 1;
      off <<= 7;
      off |= b as u64 & 0x7F;
      if b&0x80 == 0 {
        return ReadResult::Ok(off, buf.as_slice());
      }
    }
    ReadResult::Incomplete(1)
  }
}

#[derive(Copy, Clone, Debug)]
struct InvalidDeltaHeader(());
