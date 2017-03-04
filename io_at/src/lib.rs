#![no_std]
extern crate void;

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
