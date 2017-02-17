#![no_std]
mod std { pub use core::*; }

extern crate gulp;
extern crate safe_shl;

use gulp::{Parse, ParseResult};

#[derive(Copy, Clone, Debug)]
pub struct Header {
  pub base_len:   u64,
  pub result_len: u64
}

 #[derive(Copy, Clone, Debug)]
pub struct InvalidHeader(());

impl From<gulp::Overflow> for InvalidHeader {
  fn from(_: gulp::Overflow) -> InvalidHeader {
    InvalidHeader(())
  }
}

#[derive(Default)]
pub struct HeaderParser(gulp::Pair<gulp::Leb128, gulp::Leb128, gulp::Overflow>);

impl Parse for HeaderParser {
  type Err = InvalidHeader;
  type Output = Header;
  fn parse(self, buf: &[u8]) -> ParseResult<Self> {
    match self.0.parse(buf) {
      ParseResult::Incomplete(p) => ParseResult::Incomplete(HeaderParser(p)),
      ParseResult::Err(gulp::Overflow) => ParseResult::Err(InvalidHeader(())),
      ParseResult::Done((base_len, result_len), tail) => ParseResult::Done(Header { base_len: base_len, result_len: result_len }, tail)
    }
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

#[derive(Copy, Clone, Debug)]
pub struct InvalidCommand(());

impl From<gulp::Overflow> for InvalidCommand {
  fn from(_: gulp::Overflow) -> InvalidCommand {
    InvalidCommand(())
  }
}

pub struct CommandParser(CommandParserState);

enum CommandParserState {
  Fresh,
  CopyOff(VarintParser),
  CopyLen(u64, VarintParser)
}

impl Default for CommandParser {
  fn default() -> CommandParser {
    CommandParser(CommandParserState::Fresh)
  }
}

impl Parse for CommandParser {
  type Output = Command;
  type Err = InvalidCommand;
  fn parse(self, buf: &[u8]) -> gulp::ParseResult<Self> {
    match self.0 {
      CommandParserState::Fresh           => CommandParser::parse_op(buf),
      CommandParserState::CopyOff(p)      => CommandParser::parse_copy_off(p, buf),
      CommandParserState::CopyLen(off, p) => CommandParser::parse_copy_len(off, p, buf),
    }
  }
}

impl CommandParser {
  fn parse_op(buf: &[u8]) -> ParseResult<Self> {
    let mut buf = buf.iter();
    match buf.next() {
      None => ParseResult::Incomplete(CommandParser(CommandParserState::Fresh)),
      Some(&b) => match b {
        0 => ParseResult::Err(InvalidCommand(())),
        len if len&0x80 == 0 => ParseResult::Done(Command::Insert { len: len }, buf.as_slice()),
        bitmap => CommandParser::parse_copy_off(VarintParser::new(bitmap, 4), buf.as_slice())
      }
    }
  }
  fn parse_copy_off(p: VarintParser, buf: &[u8]) -> ParseResult<Self> {
    match p.parse(buf) {
      ParseResult::Incomplete(p) => ParseResult::Incomplete(CommandParser(CommandParserState::CopyOff(p))),
      ParseResult::Err(gulp::Overflow) => ParseResult::Err(InvalidCommand(())),
      ParseResult::Done((off, bitmap), buf) => CommandParser::parse_copy_len(off, VarintParser::new(bitmap, 3), buf),
    }
  }
  fn parse_copy_len(off: u64, p: VarintParser, buf: &[u8]) -> gulp::ParseResult<Self> {
    match p.parse(buf) {
      ParseResult::Incomplete(p) => ParseResult::Incomplete(CommandParser(CommandParserState::CopyLen(off, p))),
      ParseResult::Err(gulp::Overflow) => ParseResult::Err(InvalidCommand(())),
      ParseResult::Done((0,   _), buf) => ParseResult::Done(Command::Copy { off: off as u32, len: 0x10000    }, buf),
      ParseResult::Done((len, _), buf) => ParseResult::Done(Command::Copy { off: off as u32, len: len as u32 }, buf)
    }
  }
}

struct VarintParser {
  bitmap: u8,
  n: u64,
  i: core::ops::Range<u8>
}

impl VarintParser {
  fn new(bitmap: u8, length: u8) -> VarintParser {
    VarintParser {
      bitmap: bitmap,
      n: 0,
      i: 0..length
    }
  }
}

impl Parse for VarintParser {
  type Output = (u64, u8);
  type Err = gulp::Overflow;
  fn parse(mut self, buf: &[u8]) -> ParseResult<Self> {
    let mut iter = buf.iter();
    while let Some(i) = self.i.next() {
      if self.bitmap&1 != 0 {
        match iter.next() {
          Some(&b) => match safe_shl::u64(b as u64, i as u32 * 8) {
            Some(m) => self.n |= m,
            None => return ParseResult::Err(gulp::Overflow)
          },
          None => return ParseResult::Incomplete(self)
        }
      }
      self.bitmap >>= 1;
    }
    ParseResult::Done((self.n, self.bitmap), iter.as_slice())
  }
}
