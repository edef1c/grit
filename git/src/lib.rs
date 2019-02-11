#![cfg_attr(not(feature = "std"), no_std)]

use void::Void;
use gulp::{Parse, ParseResult};
use core::fmt::{self, Write};

#[cfg(feature = "std")] pub use io::*;
#[cfg(feature = "std")] mod io;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ObjectKind {
  Commit,
  Tree,
  Blob,
  Tag
}

impl ObjectKind {
  pub fn name(&self) -> &'static str {
    match *self {
      ObjectKind::Commit => "commit",
      ObjectKind::Tree   => "tree",
      ObjectKind::Blob   => "blob",
      ObjectKind::Tag    => "tag"
    }
  }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct ObjectHeader {
  pub kind: ObjectKind,
  pub size: u64
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct ObjectId(pub [u8; 20]);

#[derive(Default, Debug, Eq, PartialEq)]
pub struct ObjectIdParser(gulp::Bytes<[u8; 20]>);

impl Parse for ObjectIdParser {
  type Output = ObjectId;
  type Err = Void;
  fn parse(self, buf: &[u8]) -> ParseResult<Self> {
    match self.0.parse(buf) {
      gulp::Result::Incomplete(p) => gulp::Result::Incomplete(ObjectIdParser(p)),
      gulp::Result::Err(e) => match e {},
      gulp::Result::Ok(buf, tail) => gulp::Result::Ok(ObjectId(buf), tail)
    }
  }
}

impl fmt::Display for ObjectId {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let &ObjectId(ref bytes) = self;
    for &b in bytes.iter() {
      write!(f, "{:02x}", b)?;
    }
    Ok(())
  }
}

impl fmt::Debug for ObjectId {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "ObjectId({})", self)
  }
}

pub struct ObjectHasher(sha1dc::Hasher);

impl ObjectHasher {
    pub fn new(header: ObjectHeader) -> ObjectHasher {
        let mut h = sha1dc::Hasher::new();
        write!(h, "{} {}\u{0}", header.kind.name(), header.size).unwrap();
        ObjectHasher(h)
    }
    pub fn update(&mut self, buffer: &[u8]) {
        self.0.update(buffer)
    }
    pub fn digest(self) -> ObjectId {
        ObjectId(self.0.digest())
    }
}
