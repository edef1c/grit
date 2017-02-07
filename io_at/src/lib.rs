#![no_std]

#[cfg(feature = "std")]
pub use os::Fd;

#[cfg(feature = "std")]
mod os;

use core::fmt::Debug;

pub trait ReadAt {
  type Err: Debug;
  fn read_at(&self, off: u64, buf: &mut [u8]) -> Result<usize, Self::Err>;
}

pub trait WriteAt {
  type Err: Debug;
  fn write_at(&self, off: u64, buf: &[u8]) -> Result<usize, Self::Err>;
}
