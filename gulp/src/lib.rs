#![no_std]

use core::fmt::Debug;
pub use parsers::*;

mod parsers;

#[derive(Debug, Eq, PartialEq)]
pub enum Result<'a, P, T, E> {
  Incomplete(P),
  Ok(T, &'a [u8]),
  Err(E)
}

pub type ParseResult<'a, P> = Result<'a, P, <P as Parse>::Output, <P as Parse>::Err>;

pub trait Parse: Sized {
  type Err: Debug;
  type Output;
  fn parse(self, buffer: &[u8]) -> ParseResult<Self>;
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
