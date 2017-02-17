use void::Void;
use core::fmt::Debug;
use core::convert::From;
use core::marker::PhantomData;

pub enum ParseResult<'a, P: Parse> {
  Incomplete(P),
  Done(P::Output, &'a [u8]),
  Err(P::Err)
}

pub trait Parse: Sized {
  type Err: Debug;
  type Output;
  fn parse(self, &[u8]) -> ParseResult<Self>;
}

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
        self.val[self.len..].copy_from_slice(buf);
        self.len += buf.len();
        if self.len < $n {
          ParseResult::Incomplete(self)
        } else {
          ParseResult::Done(self.val, tail)
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
    ParseResult::Done([], buf)
  }
}

bytes!( 1); bytes!( 2); bytes!( 3); bytes!( 4); bytes!( 5); bytes!( 6); bytes!( 7); bytes!( 8); bytes!( 9); bytes!(10);
bytes!(11); bytes!(12); bytes!(13); bytes!(14); bytes!(15); bytes!(16); bytes!(17); bytes!(18); bytes!(19); bytes!(20);
bytes!(21); bytes!(22); bytes!(23); bytes!(24); bytes!(25); bytes!(26); bytes!(27); bytes!(28); bytes!(29); bytes!(30);
bytes!(31); bytes!(32); bytes!(33); bytes!(34); bytes!(35); bytes!(36); bytes!(37); bytes!(38); bytes!(39); bytes!(40);

pub struct Pair<T, U, E>
  where T: Parse, U: Parse + Default,
        E: From<T::Err> + From<U::Err> + Debug {
  state: PairState<T, U>,
  _phantom: PhantomData<*const E>
}

enum PairState<T: Parse, U> {
  First(T),
  Second(T::Output, U)
}


impl<T, U, E> Default for Pair<T, U, E>
  where T: Parse + Default, U: Parse + Default,
        E: From<T::Err> + From<U::Err> + Debug {
  fn default() -> Self {
    Pair {
      state: PairState::First(Default::default()),
      _phantom: PhantomData
    }
  }
}

impl<T, U, E> Parse for Pair<T, U, E>
  where T: Parse, U: Parse + Default,
        E: From<T::Err> + From<U::Err> + Debug {
  type Output = (T::Output, U::Output);
  type Err = E;
  fn parse(self, buf: &[u8]) -> ParseResult<Self> {
    match self.state {
      PairState::First(p) => Pair::parse_fst(p, buf),
      PairState::Second(fst, p) => Pair::parse_snd(fst, p, buf)
    }
  }
}

impl<T, U, E> Pair<T, U, E>
  where T: Parse, U: Parse + Default,
        E: From<T::Err> + From<U::Err> + Debug {
  fn wrap(state: PairState<T, U>) -> Pair<T, U, E> {
    Pair { state: state, _phantom: PhantomData }
  }
  fn parse_fst(p: T, buf: &[u8]) -> ParseResult<Self> {
    match p.parse(buf) {
      ParseResult::Incomplete(p) => ParseResult::Incomplete(Pair::wrap(PairState::First(p))),
      ParseResult::Err(e) => ParseResult::Err(From::from(e)),
      ParseResult::Done(fst, tail) => Pair::parse_snd(fst, Default::default(), tail)
    }
  }
  fn parse_snd(fst: T::Output, p: U, buf: &[u8]) -> ParseResult<Self> {
    match p.parse(buf) {
      ParseResult::Incomplete(p) => ParseResult::Incomplete(Pair::wrap(PairState::Second(fst, p))),
      ParseResult::Err(e) => ParseResult::Err(From::from(e)),
      ParseResult::Done(snd, tail) => ParseResult::Done((fst, snd), tail)
    }
  }
}

#[derive(Default, Debug)]
pub struct Leb128 {
  shift: u8,
  value: u64
}

impl Parse for Leb128 {
  type Err = super::Overflow;
  type Output = u64;
  fn parse(mut self, buf: &[u8]) -> ParseResult<Self> {
    let mut buf = buf.iter();
    while let Some(&b) = buf.next() {
      self.value |= (b as u64&0x7F) << self.shift;
      if b&0x80 == 0 {
        return ParseResult::Done(self.value, buf.as_slice());
      }
      self.shift += 7;
    }
    ParseResult::Incomplete(self)
  }
}
