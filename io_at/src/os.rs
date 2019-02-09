use crate::{ReadAt, WriteAt};
use core::i64;
use std::os::unix::io::AsRawFd;
use std::io;
use libc::{pread, pwrite, EINVAL, c_void};

pub struct Fd<T: AsRawFd> {
  inner: T
}

impl<T: AsRawFd> Fd<T> {
  pub fn new(inner: T) -> Fd<T> {
    Fd { inner }
  }
  pub fn unwrap(self) -> T {
    self.inner
  }
}

impl<T: AsRawFd> ReadAt for Fd<T> {
  type Err = io::Error;
  fn read_at(&self, off: u64, buf: &mut [u8]) -> io::Result<usize> {
    if off >= i64::MAX as u64 {
      return Ok(0);
    }
    let ret = unsafe { pread(self.inner.as_raw_fd(), buf.as_mut_ptr() as *mut c_void, buf.len(), off as i64) };
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
    let ret = unsafe { pwrite(self.inner.as_raw_fd(), buf.as_ptr() as *const c_void, buf.len(), off as i64) };
    if ret < 0 {
      Err(io::Error::last_os_error())
    } else {
      Ok(ret as usize)
    }
  }
}
