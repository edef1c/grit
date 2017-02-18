#![no_std]
mod std { pub use core::*; }

extern crate void;
extern crate safe_shl;
extern crate gulp;
extern crate byteorder;
extern crate git;

use gulp::{Parse, ParseResult};

#[derive(Copy, Clone, Debug)]
pub struct FileHeader {
  pub count: u32
}

#[derive(Copy, Clone, Debug)]
pub struct InvalidFileHeader(());

pub struct FileHeaderParser(FileHeaderParserState);

impl Default for FileHeaderParser {
  fn default() -> FileHeaderParser {
    FileHeaderParser(FileHeaderParserState::Tag(0))
  }
}

enum FileHeaderParserState {
  Tag(usize),
  Count(gulp::Bytes<[u8; 4]>)
}

impl Parse for FileHeaderParser {
  type Output = FileHeader;
  type Err = InvalidFileHeader;
  fn parse(self, buf: &[u8]) -> ParseResult<Self> {
    match self.0 {
      FileHeaderParserState::Tag(n)   => FileHeaderParser::parse_tag(n, buf),
      FileHeaderParserState::Count(p) => FileHeaderParser::parse_count(p, buf)
    }
  }
}

impl FileHeaderParser {
  fn parse_tag(n: usize, buf: &[u8]) -> ParseResult<Self> {
    const TAG: &'static [u8] = b"PACK\x00\x00\x00\x02";
    let mut buf = buf.iter();
    let mut tag = TAG[n..].iter();
    while let Some((b, t)) = (&mut tag).zip(&mut buf).next() {
      if b != t {
        return ParseResult::Err(InvalidFileHeader(()));
      }
    }
    if tag.len() != 0 {
      ParseResult::Incomplete(FileHeaderParser(FileHeaderParserState::Tag(TAG.len() - tag.len())))
    } else {
      FileHeaderParser::parse_count(gulp::Bytes::default(), buf.as_slice())
    }
  }
  fn parse_count(p: gulp::Bytes<[u8; 4]>, buf: &[u8]) -> ParseResult<Self> {
    match p.parse(buf) {
      ParseResult::Incomplete(p) => ParseResult::Incomplete(FileHeaderParser(FileHeaderParserState::Count(p))),
      ParseResult::Err(e) => match e {},
      ParseResult::Done(count, tail) => {
        use byteorder::ByteOrder;
        let count = byteorder::NetworkEndian::read_u32(&count);
        ParseResult::Done(FileHeader { count: count }, tail)
      }
    }
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

impl Default for EntryHeaderParser {
  fn default() -> EntryHeaderParser {
    EntryHeaderParser(EntryHeaderParserState::Fresh)
  }
}

pub struct EntryHeaderParser(EntryHeaderParserState);

enum EntryHeaderParserState {
  Fresh,
  Size(EntryKind, gulp::Leb128),
  Delta(DeltaHeaderParser)
}

impl Parse for EntryHeaderParser {
  type Output = EntryHeader;
  type Err = InvalidEntryHeader;
  fn parse(self, buf: &[u8]) -> ParseResult<Self> {
    match self.0 {
      EntryHeaderParserState::Fresh         => EntryHeaderParser::parse_fresh(buf),
      EntryHeaderParserState::Size(kind, p) => EntryHeaderParser::parse_size(kind, p, buf),
      EntryHeaderParserState::Delta(p)      => EntryHeaderParser::parse_delta(p, buf)
    }
  }
}

impl EntryHeaderParser {
  fn parse_fresh(buf: &[u8]) -> ParseResult<Self> {
    let mut buf = buf.iter();
    let byte = match buf.next() {
      None => return ParseResult::Incomplete(EntryHeaderParser(EntryHeaderParserState::Fresh)),
      Some(&b) => b
    };
    let kind: EntryKind = match (byte>>4) & 7 {
      1 => From::from(git::ObjectKind::Commit),
      2 => From::from(git::ObjectKind::Tree),
      3 => From::from(git::ObjectKind::Blob),
      4 => From::from(git::ObjectKind::Tag),
      6 => From::from(DeltaKind::Offset),
      7 => From::from(DeltaKind::Reference),
      _ => return ParseResult::Err(InvalidEntryHeader(()))
    };
    let size = byte as u64 & 15;
    if byte&0x80 != 0 {
      EntryHeaderParser::parse_size(kind, gulp::Leb128::new(4, size), buf.as_slice())
    } else {
      EntryHeaderParser::parse_tail(kind, size, buf.as_slice())
    }
  }
  fn parse_size(kind: EntryKind, p: gulp::Leb128, buf: &[u8]) -> ParseResult<Self> {
    match p.parse(buf) {
      ParseResult::Incomplete(p) => ParseResult::Incomplete(EntryHeaderParser(EntryHeaderParserState::Size(kind, p))),
      ParseResult::Err(e) => ParseResult::Err(From::from(e)),
      ParseResult::Done(size, tail) => EntryHeaderParser::parse_tail(kind, size, tail)
    }
  }
  fn parse_tail(kind: EntryKind, size: u64, buf: &[u8]) -> ParseResult<Self> {
    match kind {
      EntryKind::Object(kind) => ParseResult::Done(From::from(git::ObjectHeader { kind: kind, size: size }), buf),
      EntryKind::Delta(kind)  => EntryHeaderParser::parse_delta(DeltaHeaderParser::new(size, kind), buf)
    }
  }
  fn parse_delta(p: DeltaHeaderParser, buf: &[u8]) -> ParseResult<Self> {
    match p.parse(buf) {
      ParseResult::Incomplete(p) => ParseResult::Incomplete(EntryHeaderParser(EntryHeaderParserState::Delta(p))),
      ParseResult::Err(e) => ParseResult::Err(From::from(e)),
      ParseResult::Done(header, tail) => ParseResult::Done(From::from(header), tail)
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

enum DeltaHeaderParser {
  Offset(u64, DeltaOffsetParser),
  Reference(u64, git::ObjectIdParser)
}

impl DeltaHeaderParser {
  fn new(delta_len: u64, kind: DeltaKind) -> DeltaHeaderParser {
    match kind {
      DeltaKind::Offset    => DeltaHeaderParser::Offset(delta_len, DeltaOffsetParser::Fresh),
      DeltaKind::Reference => DeltaHeaderParser::Reference(delta_len, git::ObjectIdParser::default())
    }
  }
}

impl Parse for DeltaHeaderParser {
  type Output = DeltaHeader;
  type Err = InvalidDeltaHeader;
  fn parse(self, buf: &[u8]) -> ParseResult<Self> {
    match self {
      DeltaHeaderParser::Offset(delta_len, p) => match p.parse(buf) {
        ParseResult::Incomplete(p) => ParseResult::Incomplete(DeltaHeaderParser::Offset(delta_len, p)),
        ParseResult::Err(e) => ParseResult::Err(e),
        ParseResult::Done(base, tail) => ParseResult::Done(DeltaHeader::Offset { delta_len: delta_len, base: base }, tail)
      },
      DeltaHeaderParser::Reference(delta_len, p) => match p.parse(buf) {
        ParseResult::Incomplete(p) => ParseResult::Incomplete(DeltaHeaderParser::Reference(delta_len, p)),
        ParseResult::Err(e) => match e {},
        ParseResult::Done(base, tail) => ParseResult::Done(DeltaHeader::Reference { delta_len: delta_len, base: base }, tail)
      }
    }
  }
}

enum DeltaOffsetParser {
  Fresh,
  Offset(u64)
}

impl Parse for DeltaOffsetParser {
  type Output = u64;
  type Err = InvalidDeltaHeader;
  fn parse(self, buf: &[u8]) -> ParseResult<Self> {
    match self {
      DeltaOffsetParser::Fresh => Self::parse_fresh(buf),
      DeltaOffsetParser::Offset(off) => Self::parse_off(off, buf)
    }
  }
}

impl DeltaOffsetParser {
  fn parse_fresh(buf: &[u8]) -> ParseResult<Self> {
    let mut buf = buf.iter();
    let b = match buf.next() {
      None => return ParseResult::Incomplete(DeltaOffsetParser::Fresh),
      Some(&b) => b
    };
    let off = b as u64 & 0x7F;
    if b&0x80 == 0 {
      ParseResult::Done(off, buf.as_slice())
    } else {
      Self::parse_off(off, buf.as_slice())
    }
  }
  fn parse_off(mut off: u64, buf: &[u8]) -> ParseResult<Self> {
    let mut buf = buf.iter();
    while let Some(&b) = buf.next() {
      off += 1;
      off = match safe_shl::u64(off, 7) {
        None => return ParseResult::Err(InvalidDeltaHeader(())),
        Some(off) => off
      };
      off |= b as u64 & 0x7F;
      if b&0x80 == 0 {
        return ParseResult::Done(off, buf.as_slice());
      }
    }
    ParseResult::Incomplete(DeltaOffsetParser::Offset(off))
  }
}

#[derive(Copy, Clone, Debug)]
struct InvalidDeltaHeader(());
