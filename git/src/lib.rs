#![no_std]
extern crate void;
extern crate gulp;

use void::Void;
use gulp::{Parse, ParseResult};
use core::fmt;

#[derive(Copy, Clone, Debug)]
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

#[derive(Copy, Clone, Debug)]
pub struct ObjectHeader {
  pub kind: ObjectKind,
  pub size: u64
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct ObjectId(pub [u8; 20]);

#[derive(Default)]
pub struct ObjectIdParser(gulp::Bytes<[u8; 20]>);

impl Parse for ObjectIdParser {
  type Output = ObjectId;
  type Err = Void;
  fn parse(self, buf: &[u8]) -> ParseResult<Self> {
    match self.0.parse(buf) {
      ParseResult::Incomplete(p) => ParseResult::Incomplete(ObjectIdParser(p)),
      ParseResult::Err(e) => match e {},
      ParseResult::Done(buf, tail) => ParseResult::Done(ObjectId(buf), tail)
    }
  }
}

impl fmt::Display for ObjectId {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let &ObjectId(ref bytes) = self;
    for &b in bytes.iter() {
      try!(write!(f, "{:02x}", b));
    }
    Ok(())
  }
}

impl fmt::Debug for ObjectId {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "ObjectId({})", self)
  }
}
