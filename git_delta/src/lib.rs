#![no_std]
mod std { pub use core::*; }

#[macro_use]
extern crate gulp;
extern crate safe_shl;

use gulp::{ReadResult, FromBytes, read_u8, read_le_base128};

#[derive(Copy, Clone, Debug)]
pub struct Header {
  pub base_len:   u64,
  pub result_len: u64
}

impl FromBytes for Header {
  type Err = InvalidHeader;
  fn from_bytes(buf: &[u8]) -> ReadResult<Header, InvalidHeader> {
    let (base_len,   buf) = try_read!(read_le_base128(buf));
    let (result_len, buf) = try_read!(read_le_base128(buf));
    ReadResult::Ok(Header {
      base_len:   base_len,
      result_len: result_len
    }, buf)
  }
}

#[derive(Copy, Clone, Debug)]
pub struct InvalidHeader(());

impl From<gulp::Overflow> for InvalidHeader {
  fn from(_: gulp::Overflow) -> InvalidHeader {
    InvalidHeader(())
  }
}

#[derive(Copy, Clone, Debug)]
pub enum Command {
  Insert { len: u8 },
  Copy { off: u32, len: u32 }
}

impl Command {
  pub fn len(&self) -> u32 {
    match *self {
      Command::Insert { len, .. } => len as u32,
      Command::Copy   { len, .. } => len
    }
  }
}

impl FromBytes for Command {
  type Err = InvalidCommand;
  fn from_bytes(buf: &[u8]) -> ReadResult<Command, InvalidCommand> {
    let (op, buf) = try_read_void!(read_u8(buf));
    match op {
      0u8 => ReadResult::Err(InvalidCommand(())),
      len if len&0x80 == 0 => ReadResult::Ok(Command::Insert { len: len }, buf),
      mut bitmap => {
        let (off, buf) = try_read!(read_varint(buf, &mut bitmap, 4));
        let (len, buf) = try_read!(read_varint(buf, &mut bitmap, 3));
        let len = match len {
          0   => 0x10000,
          len => len
        };
        ReadResult::Ok(Command::Copy { off: off as u32, len: len as u32 }, buf)
      }
    }
  }
}

#[derive(Copy, Clone, Debug)]
pub struct InvalidCommand(());

impl From<gulp::Overflow> for InvalidCommand {
  fn from(_: gulp::Overflow) -> InvalidCommand {
    InvalidCommand(())
  }
}

pub fn read_varint<'a>(buf: &'a [u8], bitmap: &mut u8, length: u8) -> ReadResult<'a, u64, gulp::Overflow> {
  let mut iter = buf.iter();
  let mut n = 0;
  for i in 0..length {
    if *bitmap&1 != 0 {
      match iter.next() {
        Some(&b) => match safe_shl::u64(b as u64, i as u32 * 8) {
          Some(m) => n |= m,
          None => return ReadResult::Err(gulp::Overflow)
        },
        None => return ReadResult::Incomplete(bitmap.count_ones() as usize)
      }
    }
    *bitmap >>= 1;
  }
  ReadResult::Ok(n, iter.as_slice())
}
