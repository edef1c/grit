#![no_std]

extern crate void;
extern crate safe_shl;
extern crate byteorder;

use void::Void;
use byteorder::ByteOrder;
use core::fmt::Debug;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub enum ReadResult<'a, T, E> {
  Ok(T, &'a [u8]),
  Err(E),
  Incomplete(usize)
}

impl<'a, T, E> ReadResult<'a, T, E> {
  pub fn as_ref(&self) -> ReadResult<'a, &T, &E> {
    match *self {
      ReadResult::Ok(ref val, buf) => ReadResult::Ok(val, buf),
      ReadResult::Incomplete(n)    => ReadResult::Incomplete(n),
      ReadResult::Err(ref e)       => ReadResult::Err(e)
    }
  }
  pub fn as_mut(&mut self) -> ReadResult<'a, &mut T, &mut E> {
    match *self {
      ReadResult::Ok(ref mut val, buf) => ReadResult::Ok(val, buf),
      ReadResult::Incomplete(n)        => ReadResult::Incomplete(n),
      ReadResult::Err(ref mut e)       => ReadResult::Err(e)
    }
  }
  pub fn map<O, U>(self, o: O) -> ReadResult<'a, U, E> where O: FnOnce(T) -> U {
    match self {
      ReadResult::Ok(val, buf)  => ReadResult::Ok(o(val), buf),
      ReadResult::Incomplete(n) => ReadResult::Incomplete(n),
      ReadResult::Err(e)        => ReadResult::Err(e)
    }
  }
  pub fn map_err<O, F>(self, o: O) -> ReadResult<'a, T, F> where O: FnOnce(E) -> F {
    match self {
      ReadResult::Ok(val, buf)  => ReadResult::Ok(val, buf),
      ReadResult::Incomplete(n) => ReadResult::Incomplete(n),
      ReadResult::Err(e)        => ReadResult::Err(o(e))
    }
  }
}

#[macro_export]
macro_rules! try_read {
  ($x:expr) => {
    match $x {
      $crate::ReadResult::Ok(val, buf)  => (val, buf),
      $crate::ReadResult::Incomplete(n) => return $crate::ReadResult::Incomplete(n),
      $crate::ReadResult::Err(e)        => return $crate::ReadResult::Err(::std::convert::From::from(e))
    }
  }
}

#[macro_export]
macro_rules! try_read_void {
  ($x:expr) => {
    match $x {
      $crate::ReadResult::Ok(val, buf)  => (val, buf),
      $crate::ReadResult::Incomplete(n) => return $crate::ReadResult::Incomplete(n),
      $crate::ReadResult::Err(e)        => match e {}
    }
  }
}

pub fn read_u8(buf: &[u8]) -> ReadResult<u8, Void> {
  if buf.len() != 0 {
    ReadResult::Ok(buf[0], &buf[1..])
  } else {
    ReadResult::Incomplete(1)
  }
}

pub fn read_u16<O: ByteOrder>(buf: &[u8]) -> ReadResult<u16, Void> {
  <[u8; 2] as FromBytes>::from_bytes(buf).map(|b| O::read_u16(&b[..]))
}

pub fn read_u32<O: ByteOrder>(buf: &[u8]) -> ReadResult<u32, Void> {
  <[u8; 4] as FromBytes>::from_bytes(buf).map(|b| O::read_u32(&b[..]))
}

pub fn read_u64<O: ByteOrder>(buf: &[u8]) -> ReadResult<u64, Void> {
  <[u8; 8] as FromBytes>::from_bytes(buf).map(|b| O::read_u64(&b[..]))
}

pub fn read_tag<'a>(buf: &'a [u8], tag: &[u8]) -> ReadResult<'a, (), ()> {
  let mut buf = buf.iter();
  let mut tag = tag.iter();
  while let Some((b, t)) = (&mut tag).zip(&mut buf).next() {
    if b != t {
      return ReadResult::Err(());
    }
  }
  if tag.len() == 0 {
    ReadResult::Ok((), buf.as_slice())
  } else {
    ReadResult::Incomplete(tag.len())
  }
}

pub trait FromBytes: Sized {
  type Err: Debug;
  fn from_bytes<'a>(&'a [u8]) -> ReadResult<'a, Self, Self::Err>;
}

impl FromBytes for u8 {
  type Err = Void;
  fn from_bytes(buf: &[u8]) -> ReadResult<u8, Void> {
    read_u8(buf)
  }
}

macro_rules! array_from_bytes {
  ($n:expr) => {
    impl FromBytes for [u8; $n] {
      type Err = Void;
      fn from_bytes(buf: &[u8]) -> ReadResult<[u8; $n], Void> {
        let mut result = [0; $n];
        if buf.len() < result.len() {
          ReadResult::Incomplete(result.len() - buf.len())
        } else {
          let (head, tail) = buf.split_at(result.len());
          result.copy_from_slice(head);
          ReadResult::Ok(result, tail)
        }
      }
    }
  }
}

impl FromBytes for [u8; 0] {
  type Err = Void;
  fn from_bytes(buf: &[u8]) -> ReadResult<[u8; 0], Void> {
    ReadResult::Ok([], buf)
  }
}

array_from_bytes!( 1); array_from_bytes!( 2); array_from_bytes!( 3); array_from_bytes!( 4); array_from_bytes!( 5); array_from_bytes!( 6); array_from_bytes!( 7); array_from_bytes!( 8); array_from_bytes!( 9); array_from_bytes!(10);
array_from_bytes!(11); array_from_bytes!(12); array_from_bytes!(13); array_from_bytes!(14); array_from_bytes!(15); array_from_bytes!(16); array_from_bytes!(17); array_from_bytes!(18); array_from_bytes!(19); array_from_bytes!(20);
array_from_bytes!(21); array_from_bytes!(22); array_from_bytes!(23); array_from_bytes!(24); array_from_bytes!(25); array_from_bytes!(26); array_from_bytes!(27); array_from_bytes!(28); array_from_bytes!(29); array_from_bytes!(30);
array_from_bytes!(31); array_from_bytes!(32); array_from_bytes!(33); array_from_bytes!(34); array_from_bytes!(35); array_from_bytes!(36); array_from_bytes!(37); array_from_bytes!(38); array_from_bytes!(39); array_from_bytes!(40);

pub struct Overflow;

pub fn read_le_base128(buf: &[u8]) -> ReadResult<u64, Overflow> {
  read_le_base128_cont(buf, 0, 0)
}

pub fn read_le_base128_cont(buf: &[u8], mut n: u64, mut shift: u32) -> ReadResult<u64, Overflow> {
  let mut iter = buf.iter();
  while let Some(&b) = iter.next() {
    match safe_shl::u64(b as u64 & 0x7F, shift) {
      Some(m) => n |= m,
      None    => return ReadResult::Err(Overflow)
    };
    shift += 7;
    if b&0x80 == 0 {
      return ReadResult::Ok(n, iter.as_slice());
    }
  }
  ReadResult::Incomplete(0)
}
