extern crate gulp;
extern crate git;
extern crate git_delta;
extern crate git_packfile;
extern crate hexdump;
extern crate flate2;
extern crate sha1;

use std::{io, fs};
use std::io::{Read, BufRead, Write, Seek, SeekFrom};
use std::fmt::Write as FmtWrite;
use gulp::{ReadResult, FromBytes, Parse, ParseResult};

const PACK_PATH: &'static str = "/home/edef/src/github.com/edef1c/libfringe/.git/objects/pack/pack-b452a7d6bcc41ff3e93d12ef285a17c9c04c9804.pack";

fn full_path_for_object_id(object_id: git::ObjectId) -> String {
  format!("/home/edef/src/github.com/edef1c/libfringe-unpacked/.git/objects/{}", path_for_object_id(object_id))
}

fn main() {
  let mut r = fs::File::open(PACK_PATH).map(io::BufReader::new).unwrap();
  let mut file_header: git_packfile::FileHeader = parse_from_buf_reader::<_, git_packfile::FileHeaderParser>(&mut r).unwrap().unwrap();
  writeln!(io::stderr(), "{:?}", file_header).unwrap();
  let mut objects = PackfileIndex::new();
  while let (position, Some(entry_header)) = (r.seek(SeekFrom::Current(0)).unwrap(), parse_from_buf_reader::<_, git_packfile::EntryHeaderParser>(&mut r).unwrap()) {
    writeln!(io::stderr(), "{:?}", entry_header).unwrap();
    let mut body = flate2::bufread::ZlibDecoder::new(&mut r);
    let mut hasher = Sha1Writer(sha1::Sha1::new(), io::sink());
    let mut delta_body;
    let (kind, size, mut body): (git::ObjectKind, u64, &mut Read) = match entry_header {
      git_packfile::EntryHeader::Object(object_header) => {
        (object_header.kind, object_header.size, &mut body)
      }
      git_packfile::EntryHeader::Delta(delta) => {
        let (base_id, kind) = match delta {
          git_packfile::DeltaHeader::Reference { base, .. } => {
            match objects.find_by_id(base) {
              Some(entry) => (base, entry.kind),
              None => panic!("couldn't find base object {}", base)
            }
          },
          git_packfile::DeltaHeader::Offset { base, .. } => {
            let base_position = position - base;
            match objects.find_by_offset(base_position) {
              Some(entry) => (entry.id, entry.kind),
              None => panic!("couldn't find base object at {}", base_position)
            }
          }
        };
        let base_path = full_path_for_object_id(base_id);
        let base = {
          let mut r = fs::File::open(base_path).map(flate2::read::ZlibDecoder::new).map(io::BufReader::new).unwrap();
          r.read_until(0, &mut Vec::new()).unwrap();
          let mut buf = Vec::new();
          r.read_to_end(&mut buf).unwrap();
          io::Cursor::new(buf)
        };
        delta_body = DeltaReader::new(base, io::BufReader::new(body)).unwrap().unwrap();
        let size = delta_body.header().result_len;
        (kind, size, &mut delta_body)
      }
    };
    write!(hasher, "{} {}\u{0}", kind.name(), size).unwrap();
    io::copy(&mut body, &mut hasher).unwrap();
    let Sha1Writer(hasher, _) = hasher;
    let object_id = git::ObjectId(hasher.digest().bytes());
    objects.add(PackfileIndexEntry { id: object_id, offset: position, kind: kind });
    let object_path = full_path_for_object_id(object_id);
    fs::File::open(object_path).unwrap();

    file_header.count -= 1;
    if file_header.count == 0 {
      break;
    }
  }
}

struct PackfileIndex {
  objects: Vec<PackfileIndexEntry>,
  by_id: Vec<usize>,
  by_offset: Vec<usize>
}

#[derive(Copy, Clone, Debug)]
struct PackfileIndexEntry {
  id: git::ObjectId,
  offset: u64,
  kind: git::ObjectKind
}

impl PackfileIndex {
  fn new() -> PackfileIndex {
    PackfileIndex {
      objects: Vec::new(),
      by_id: Vec::new(),
      by_offset: Vec::new()
    }
  }
  fn add(&mut self, entry: PackfileIndexEntry) {
    let id_idx = self.by_id.binary_search_by_key(&entry.id, |&idx| self.objects[idx].id).err().unwrap();
    let offset_idx = self.by_offset.binary_search_by_key(&entry.offset, |&idx| self.objects[idx].offset).err().unwrap();
    let idx = self.objects.len();
    self.by_id.insert(id_idx, idx);
    self.by_offset.insert(offset_idx, idx);
    self.objects.push(entry);
  }
  fn find_by_id(&self, id: git::ObjectId) -> Option<&PackfileIndexEntry> {
    self.by_id.binary_search_by_key(&id, |&idx| self.objects[idx].id).ok().map(|idx| &self.objects[idx])
  }
  fn find_by_offset(&self, offset: u64) -> Option<&PackfileIndexEntry> {
    self.by_offset.binary_search_by_key(&offset, |&idx| self.objects[idx].offset).ok().map(|idx| &self.objects[idx])
  }
}

fn from_buf_reader<R: io::BufRead, T: FromBytes>(mut r: R) -> io::Result<Option<T>> {
  let mut buf = Vec::new();
  let mut peeked_bytes = 0;
  let mut needed_bytes;
  loop {
    match T::from_bytes(&buf) {
      ReadResult::Err(e) => panic!("{:?}", e),
      ReadResult::Ok(value, tail) => {
        r.consume(peeked_bytes - tail.len());
        return Ok(Some(value));
      }
      ReadResult::Incomplete(n) => {
        r.consume(peeked_bytes);
        needed_bytes = n;
      }
    }
    let peek = r.fill_buf().unwrap();
    peeked_bytes = peek.len();
    buf.extend_from_slice(peek);
    if peeked_bytes == 0 {
      return if buf.len() == 0 {
        Ok(None)
      } else {
        hexdump::hexdump(&buf);
        Err(io::Error::new(io::ErrorKind::UnexpectedEof, IncompleteError { needed: needed_bytes, buffered: buf.len() }))
      };
    }
  }
}

#[derive(Debug)]
pub struct IncompleteError {
  buffered: usize,
  needed: usize
}

impl ::std::error::Error for IncompleteError {
  fn description(&self) -> &str { "IncompleteError" }
  fn cause(&self) -> Option<&::std::error::Error> { None }
}

impl ::std::fmt::Display for IncompleteError {
  fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
    write!(f, "IncompleteError: needed {} more bytes after buffering {} bytes, but encountered EOF", self.needed, self.buffered)
  }
}

struct Sha1Writer<W: io::Write>(sha1::Sha1, W);

impl<W: io::Write> io::Write for Sha1Writer<W> {
  fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
    match self.1.write(buf) {
      Ok(n) => {
        self.0.update(&buf[..n]);
        Ok(n)
      }
      Err(e) => Err(e)
    }
  }
  fn flush(&mut self) -> io::Result<()> {
    self.1.flush()
  }
}

fn path_for_object_id(git::ObjectId(bytes): git::ObjectId) -> String {
  let mut result = String::new();
  write!(result, "{:02x}/", bytes[0]).unwrap();
  for &b in &bytes[1..] {
    write!(result, "{:02x}", b).unwrap();
  }
  result
}

struct DeltaReader<Base: Read + Seek, Delta: BufRead> {
  base: Base,
  delta: Delta,
  header: git_delta::Header,
  command: git_delta::Command,
  seek: bool,
}

impl<Base: Read + Seek, Delta: BufRead> DeltaReader<Base, Delta> {
  pub fn new(base: Base, mut delta: Delta) -> io::Result<Option<DeltaReader<Base, Delta>>> {
    parse_from_buf_reader::<_, git_delta::HeaderParser>(&mut delta)
      .map(|header| header
      .map(|header| DeltaReader { base: base, delta: delta, header: header, command: git_delta::Command::Insert { len: 0 }, seek: false }))
  }
  pub fn header(&self) -> git_delta::Header {
    self.header
  }
}

impl<Base: Read + Seek, Delta: BufRead> Read for DeltaReader<Base, Delta> {
  fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
    if self.command.len() == 0 {
      match try!(parse_from_buf_reader::<_, git_delta::CommandParser>(&mut self.delta)) {
      // match try!(from_buf_reader(&mut self.delta)) {
        Some(c) => { self.command = c; self.seek = true },
        None => return Ok(0)
      };
    }
    match self.command {
      git_delta::Command::Insert { ref mut len } => {
        let mut r = (&mut self.delta).take(*len as u64);
        let n = try!(r.read(buf));
        *len -= n as u8;
        Ok(n)
      }
      git_delta::Command::Copy { ref mut len, off } => {
        if self.seek {
          try!(self.base.seek(SeekFrom::Start(off as u64)));
          self.seek = false;
        }
        let mut r = (&mut self.base).take(*len as u64);
        let n = try!(r.read(buf));
        *len -= n as u32;
        Ok(n)
      }
    }
  }
}

fn parse_from_buf_reader<R: io::BufRead, P: Parse + Default>(mut r: R) -> io::Result<Option<P::Output>> {
  let mut acc = vec![];
  loop {
    let buf_len = {
      let buf = try!(r.fill_buf());
      acc.extend_from_slice(buf);
      if buf.len() == 0 {
        if acc.len() == 0 { return Ok(None) };
        panic!("parse_from_buf_reader: unexpected EOF");
      }
      buf.len()
    };
    match P::default().parse(&acc) {
      ParseResult::Incomplete(_) => r.consume(buf_len),
      ParseResult::Err(e) => panic!("parse_from_buf_reader: {:?}", e),
      ParseResult::Done(value, tail) => {
        r.consume(buf_len - tail.len());
        return Ok(Some(value));
      }
    }
  }
}
