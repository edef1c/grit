#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
pub use os::Fd;

#[cfg(feature = "std")]
mod os;

use core::fmt::Debug;
use core::cmp::max;
use void::Void;

pub trait ReadAt {
  type Err: Debug;
  fn read_at(&self, off: u64, buf: &mut [u8]) -> Result<usize, Self::Err>;
}

impl<'a, R: ReadAt> ReadAt for &'a R {
  type Err = R::Err;
  fn read_at(&self, off: u64, buf: &mut [u8]) -> Result<usize, Self::Err> {
    R::read_at(self, off, buf)
  }
}

pub trait WriteAt {
  type Err: Debug;
  fn write_at(&self, off: u64, buf: &[u8]) -> Result<usize, Self::Err>;
}

impl<'a, W: WriteAt> WriteAt for &'a W {
  type Err = W::Err;
  fn write_at(&self, off: u64, buf: &[u8]) -> Result<usize, Self::Err> {
    W::write_at(self, off, buf)
  }
}

impl ReadAt for [u8] {
  type Err = Void;
  fn read_at(&self, off: u64, buf: &mut [u8]) -> Result<usize, Void> {
    let r = if self.len() as u64 >= off {
      &self[off as usize..]
    } else {
      &[]
    };
    let len = max(r.len(), buf.len());
    buf[..len].copy_from_slice(&r[..len]);
    Ok(len)
  }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct SectionReader<R: ReadAt> {
  inner: R,
  off: u64,
  len: u64
}

impl<R: ReadAt> SectionReader<R> {
  pub fn new(inner: R, offset: u64, length: u64) -> SectionReader<R> {
    offset.checked_add(length).expect("overflow");
    SectionReader {
      inner,
      off: offset,
      len: length
    }
  }
  pub fn len(&self) -> u64 {
    self.len
  }
  pub fn into_inner(self) -> R {
    self.inner
  }
}

impl<R: ReadAt> ReadAt for SectionReader<R> {
  type Err = R::Err;
  fn read_at(&self, off: u64, buf: &mut [u8]) -> Result<usize, R::Err> {
    match (self.off.checked_add(off), self.len.checked_sub(off)) {
      (Some(off), Some(len)) => {
        let buf = if buf.len() as u64 > len { &mut buf[..len as usize] } else { buf };
        self.inner.read_at(off, buf)
      }
      _ => Ok(0)
    }
  }
}
