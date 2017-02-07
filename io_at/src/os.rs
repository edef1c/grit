extern crate libc;
extern crate std;

use {ReadAt, WriteAt};
use core::i64;
use self::std::os::unix::io::AsRawFd;
use self::std::io;
use self::libc::{pread, pwrite, EINVAL};

pub struct Fd<T: AsRawFd> {
  inner: T
}

impl<T: AsRawFd> Fd<T> {
  pub fn new(inner: T) -> Fd<T> {
    Fd { inner: inner }
  }
  pub fn unwrap(self) -> T {
    self.inner
  }
}

impl<T: AsRawFd> ReadAt for Fd<T> {
  type Err = io::Error;
  fn read_at(&self, off: u64, buf: &mut [u8]) -> io::Result<usize> {
    if off > i64::MAX as u64 {
      return Err(io::Error::from_raw_os_error(EINVAL));
    }
    let ret = unsafe { pread(self.inner.as_raw_fd(), buf.as_mut_ptr() as *mut _, buf.len(), off as i64) };
    if ret < 0 {
      Err(io::Error::last_os_error())
    } else {
      Ok(ret as usize)
    }
  }
}

impl<T: AsRawFd> WriteAt for Fd<T> {
  type Err = io::Error;
  fn write_at(&self, off: u64, buf: &[u8]) -> io::Result<usize> {
    if off > i64::MAX as u64 {
      return Err(io::Error::from_raw_os_error(EINVAL));
    }
    let ret = unsafe { pwrite(self.inner.as_raw_fd(), buf.as_ptr() as *mut _, buf.len(), off as i64) };
    if ret < 0 {
      Err(io::Error::last_os_error())
    } else {
      Ok(ret as usize)
    }
  }
}
