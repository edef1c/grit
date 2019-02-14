use std::io::{self, BufRead, Read, Seek, SeekFrom};
use crate::{Header, HeaderParser, Command, CommandParser};

pub struct Reader<Base: Read + Seek, Delta: BufRead> {
    base: Base,
    delta: Delta,
    header: Header,
    command: Command,
    seek: bool
}

impl<Base: Read + Seek, Delta: BufRead> Reader<Base, Delta> {
    pub fn new(base: Base, mut delta: Delta) -> io::Result<Reader<Base, Delta>> {
        let header = gulp::from_reader(&mut delta, HeaderParser::default)?;
        Ok(Reader { base, delta, header, command: Command::Insert { len: 0 }, seek: false })
    }
    pub fn header(&self) -> Header {
        self.header
    }
}

impl<Base: Read + Seek, Delta: BufRead> Read for Reader<Base, Delta> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.command.len() == 0 {
            match gulp::next_from_reader(&mut self.delta, CommandParser::default)? {
                Some(c) => { self.command = c; self.seek = true },
                None => return Ok(0)
            };
        }
        match self.command {
            Command::Insert { ref mut len } => {
                let mut r = (&mut self.delta).take(*len as u64);
                let n = r.read(buf)?;
                *len -= n as u8;
                Ok(n)
            }
            Command::Copy { ref mut len, off } => {
                if self.seek {
                    self.base.seek(SeekFrom::Start(off as u64))?;
                    self.seek = false;
                }
                let mut r = (&mut self.base).take(*len as u64);
                let n = r.read(buf)?;
                *len -= n as u32;
                Ok(n)
            }
        }
    }
    #[cfg(feature = "nightly")]
    unsafe fn initializer(&self) -> io::Initializer {
        let base = self.base.initializer();
        let delta = self.delta.initializer();
        if base.should_initialize() || delta.should_initialize() {
            io::Initializer::zeroing()
        } else {
            io::Initializer::nop()
        }
    }
}
