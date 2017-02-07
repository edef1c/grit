#![no_std]
extern crate void;
extern crate gulp;

use void::Void;
use gulp::{FromBytes, ReadResult};
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

impl FromBytes for ObjectId {
  type Err = Void;
  fn from_bytes(buf: &[u8]) -> ReadResult<ObjectId, Void> {
    FromBytes::from_bytes(buf).map(ObjectId)
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
