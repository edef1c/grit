#![no_std]

extern crate void;
extern crate safe_shl;

use void::Void;
use core::fmt::Debug;
use core::convert::From;
use core::marker::PhantomData;

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
pub struct Pair<P, Q, E>
  where P: Parse, Q: Parse + Default,
        E: From<P::Err> + From<Q::Err> + Debug {
  state: PairState<P, P::Output, Q>,
  _phantom: PhantomData<*const E>
}

#[derive(Debug, Eq, PartialEq)]
enum PairState<P, T, Q> {
  First(P),
  Second(T, Q)
}

impl<P, Q, E> Default for Pair<P, Q, E>
  where P: Parse + Default, Q: Parse + Default,
        E: From<P::Err> + From<Q::Err> + Debug {
  fn default() -> Self {
    Pair {
      state: PairState::First(Default::default()),
      _phantom: PhantomData
    }
  }
}

impl<P, Q, E> Parse for Pair<P, Q, E>
  where P: Parse, Q: Parse + Default,
        E: From<P::Err> + From<Q::Err> + Debug {
  type Output = (P::Output, Q::Output);
  type Err = E;
  fn parse(self, buf: &[u8]) -> ParseResult<Self> {
    match self.state {
      PairState::First(p) => Pair::parse_fst(p, buf),
      PairState::Second(fst, p) => Pair::parse_snd(fst, p, buf)
    }
  }
}

impl<P, Q, E> Pair<P, Q, E>
  where P: Parse, Q: Parse + Default,
        E: From<P::Err> + From<Q::Err> + Debug {
  fn wrap(state: PairState<P, P::Output, Q>) -> Pair<P, Q, E> {
    Pair { state: state, _phantom: PhantomData }
  }
  fn parse_fst(p: P, buf: &[u8]) -> ParseResult<Self> {
    match p.parse(buf) {
      Result::Incomplete(p) => Result::Incomplete(Pair::wrap(PairState::First(p))),
      Result::Err(e) => Result::Err(From::from(e)),
      Result::Ok(fst, tail) => Pair::parse_snd(fst, Default::default(), tail)
    }
  }
  fn parse_snd(fst: P::Output, p: Q, buf: &[u8]) -> ParseResult<Self> {
    match p.parse(buf) {
      Result::Incomplete(p) => Result::Incomplete(Pair::wrap(PairState::Second(fst, p))),
      Result::Err(e) => Result::Err(From::from(e)),
      Result::Ok(snd, tail) => Result::Ok((fst, snd), tail)
    }
  }
}

#[derive(Default, Debug)]
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

#[derive(Debug)]
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
