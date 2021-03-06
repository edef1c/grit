#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(feature = "nightly", feature(read_initializer))]

use failure::Fail;
use safe_shl::SafeShl;
use gulp::{Parse, ParseResult};

#[cfg(feature = "std")] pub use io::*;
#[cfg(feature = "std")] mod io;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Header {
    pub base_len:   u64,
    pub result_len: u64
}

#[derive(Copy, Clone, Debug, Fail, Eq, PartialEq)]
#[fail(display = "invalid delta header")]
pub struct InvalidHeader(());

#[derive(Default, Debug, Eq, PartialEq)]
pub struct HeaderParser(gulp::Pair<gulp::Leb128, gulp::Leb128>);

impl Parse for HeaderParser {
    type Err = InvalidHeader;
    type Output = Header;
    fn parse(self, buf: &[u8]) -> ParseResult<Self> {
        match self.0.parse(buf) {
            gulp::Result::Incomplete(p) => gulp::Result::Incomplete(HeaderParser(p)),
            gulp::Result::Err(gulp::Overflow) => gulp::Result::Err(InvalidHeader(())),
            gulp::Result::Ok((base_len, result_len), tail) => gulp::Result::Ok(Header { base_len, result_len }, tail)
        }
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
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

#[derive(Copy, Clone, Debug, Fail, Eq, PartialEq)]
#[fail(display = "invalid delta command")]
pub struct InvalidCommand(());

#[derive(Debug, Eq, PartialEq)]
pub struct CommandParser(CommandParserState);

#[derive(Debug, Eq, PartialEq)]
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
            None => gulp::Result::Incomplete(CommandParser(CommandParserState::Fresh)),
            Some(&b) => match b {
                0 => gulp::Result::Err(InvalidCommand(())),
                len if len&0x80 == 0 => gulp::Result::Ok(Command::Insert { len }, buf.as_slice()),
                bitmap => CommandParser::parse_copy_off(VarintParser::new(bitmap, 4), buf.as_slice())
            }
        }
    }
    fn parse_copy_off(p: VarintParser, buf: &[u8]) -> ParseResult<Self> {
        match p.parse(buf) {
            gulp::Result::Incomplete(p) => gulp::Result::Incomplete(CommandParser(CommandParserState::CopyOff(p))),
            gulp::Result::Err(gulp::Overflow) => gulp::Result::Err(InvalidCommand(())),
            gulp::Result::Ok((off, bitmap), buf) => CommandParser::parse_copy_len(off, VarintParser::new(bitmap, 3), buf),
        }
    }
    fn parse_copy_len(off: u64, p: VarintParser, buf: &[u8]) -> gulp::ParseResult<Self> {
        match p.parse(buf) {
            gulp::Result::Incomplete(p) => gulp::Result::Incomplete(CommandParser(CommandParserState::CopyLen(off, p))),
            gulp::Result::Err(gulp::Overflow) => gulp::Result::Err(InvalidCommand(())),
            gulp::Result::Ok((0,   _), buf) => gulp::Result::Ok(Command::Copy { off: off as u32, len: 0x10000    }, buf),
            gulp::Result::Ok((len, _), buf) => gulp::Result::Ok(Command::Copy { off: off as u32, len: len as u32 }, buf)
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct VarintParser {
    bitmap: u8,
    n: u64,
    i: u8,
    len: u8
}

impl VarintParser {
    fn new(bitmap: u8, length: u8) -> VarintParser {
        VarintParser {
            bitmap,
            n: 0,
            i: 0,
            len: length
        }
    }
}

impl Parse for VarintParser {
    type Output = (u64, u8);
    type Err = gulp::Overflow;
    fn parse(mut self, buf: &[u8]) -> ParseResult<Self> {
        let mut iter = buf.iter();
        while self.i < self.len {
            if self.bitmap&1 != 0 {
                match iter.next() {
                    Some(&b) => match (b as u64).safe_shl(self.i as u32 * 8) {
                        Some(m) => self.n |= m,
                        None => return gulp::Result::Err(gulp::Overflow)
                    },
                    None => return gulp::Result::Incomplete(self)
                }
            }
            self.bitmap >>= 1;
            self.i += 1;
        }
        gulp::Result::Ok((self.n, self.bitmap), iter.as_slice())
    }
}
