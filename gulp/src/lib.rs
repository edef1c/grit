#![no_std]

extern crate void;
extern crate safe_shl;

use void::Void;
use core::fmt::Debug;

#[derive(Debug, Eq, PartialEq)]
pub enum Result<'a, P, T, E> {
  Incomplete(P),
  Ok(T, &'a [u8]),
  Err(E)
}

pub type ParseResult<'a, P: Parse> = Result<'a, P, P::Output, P::Err>;

pub trait Parse: Sized {
  type Err: Debug;
  type Output;
  fn parse(self, &[u8]) -> ParseResult<Self>;
}

#[derive(Debug, Eq, PartialEq)]
pub struct Bytes<T> {
  val: T,
  len: usize
}

macro_rules! bytes {
  ($n:expr) => {
    impl Default for Bytes<[u8; $n]> {
      fn default() -> Self {
        Bytes {
          val: [0; $n],
          len: 0
        }
      }
    }
    impl Parse for Bytes<[u8; $n]> {
      type Err = Void;
      type Output = [u8; $n];
      fn parse(mut self, buf: &[u8]) -> ParseResult<Self> {
        let (buf, tail): (&[u8], &[u8]) = if buf.len() < ($n - self.len) { (buf, &[]) } else { buf.split_at($n - self.len) };
        self.val[self.len..self.len + buf.len()].copy_from_slice(buf);
        self.len += buf.len();
        if self.len < $n {
          Result::Incomplete(self)
        } else {
          Result::Ok(self.val, tail)
        }
      }
    }
  }
}

impl Default for Bytes<[u8; 0]> {
  fn default() -> Self {
    Bytes {
      val: [],
      len: 0
    }
  }
}

impl Parse for Bytes<[u8; 0]> {
  type Err = Void;
  type Output = [u8; 0];
  fn parse(self, buf: &[u8]) -> ParseResult<Self> {
    Result::Ok([], buf)
  }
}

bytes!( 1); bytes!( 2); bytes!( 3); bytes!( 4); bytes!( 5); bytes!( 6); bytes!( 7); bytes!( 8); bytes!( 9); bytes!(10);
bytes!(11); bytes!(12); bytes!(13); bytes!(14); bytes!(15); bytes!(16); bytes!(17); bytes!(18); bytes!(19); bytes!(20);
bytes!(21); bytes!(22); bytes!(23); bytes!(24); bytes!(25); bytes!(26); bytes!(27); bytes!(28); bytes!(29); bytes!(30);
bytes!(31); bytes!(32); bytes!(33); bytes!(34); bytes!(35); bytes!(36); bytes!(37); bytes!(38); bytes!(39); bytes!(40);

#[derive(Debug, Eq, PartialEq)]
pub enum Pair<P, Q> where P: Parse {
  Fst(P),
  Snd(P::Output, Q)
}

impl<P, Q> Default for Pair<P, Q> where P: Parse + Default {
  fn default() -> Self {
    Pair::Fst(P::default())
  }
}

impl<P, Q> Parse for Pair<P, Q> where P: Parse, Q: Parse<Err=P::Err> + Default {
  type Output = (P::Output, Q::Output);
  type Err = P::Err;
  fn parse(self, buf: &[u8]) -> ParseResult<Self> {
    match self {
      Pair::Fst(p)    => Pair::parse_fst(p, buf),
      Pair::Snd(x, p) => Pair::parse_snd(x, p, buf)
    }
  }
}

impl<P, Q> Pair<P, Q> where P: Parse, Q: Parse<Err=P::Err> + Default {
  fn parse_fst(p: P, buf: &[u8]) -> ParseResult<Self> {
    match p.parse(buf) {
      Result::Incomplete(p) => Result::Incomplete(Pair::Fst(p)),
      Result::Err(e)        => Result::Err(e),
      Result::Ok(x, tail)   => Pair::parse_snd(x, Q::default(), tail)
    }
  }
  fn parse_snd(x: P::Output, q: Q, buf: &[u8]) -> ParseResult<Self> {
    match q.parse(buf) {
      Result::Incomplete(q) => Result::Incomplete(Pair::Snd(x, q)),
      Result::Err(e)        => Result::Err(e),
      Result::Ok(y, tail)   => Result::Ok((x, y), tail)
    }
  }
}

#[derive(Default, Debug, Eq, PartialEq)]
pub struct Leb128 {
  shift: u8,
  value: u64
}

impl Leb128 {
  pub fn new(shift: u8, value: u64) -> Leb128 {
    Leb128 {
      shift: shift,
      value: value
    }
  }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Overflow;

impl Parse for Leb128 {
  type Err = Overflow;
  type Output = u64;
  fn parse(mut self, buf: &[u8]) -> ParseResult<Self> {
    let mut buf = buf.iter();
    while let Some(&b) = buf.next() {
      match safe_shl::u64(b as u64 & 0x7F, self.shift as u32) {
        None => return Result::Err(Overflow),
        Some(v) => self.value |= v
      }
      if b&0x80 == 0 {
        return Result::Ok(self.value, buf.as_slice());
      }
      self.shift += 7;
    }
    Result::Incomplete(self)
  }
}

pub fn split_fuzz<'a, P: Parse + Default>(data: &'a [u8]) where ParseResult<'a, P>: Debug + Eq {
  if data.len() < 4 { return }
  let n = ((data[0] as usize) <<  0)
        | ((data[1] as usize) <<  8)
        | ((data[2] as usize) << 16)
        | ((data[3] as usize) << 24);
  let data = &data[4..];
  if data.len() < n { return }
  let immediate = P::default().parse(data);
  let incremental = match P::default().parse(&data[..n]) {
    Result::Err(e) => Result::Err(e),
    Result::Incomplete(p) => p.parse(&data[n..]),
    Result::Ok(v, tail) => Result::Ok(v, &data[n - tail.len()..])
  };
  assert_eq!(immediate, incremental);
}
