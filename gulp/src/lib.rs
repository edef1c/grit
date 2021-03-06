#![cfg_attr(not(feature = "std"), no_std)]

use core::fmt::Debug;
use failure::Fail;
pub use parsers::*;
#[cfg(feature = "std")] pub use io::*;

mod parsers;
#[cfg(feature = "std")] mod io;

#[derive(Debug, Eq, PartialEq)]
pub enum Result<'a, P, T, E> {
    Incomplete(P),
    Ok(T, &'a [u8]),
    Err(E)
}

pub type ParseResult<'a, P> = Result<'a, P, <P as Parse>::Output, <P as Parse>::Err>;

pub trait Parse: Sized {
    type Err: Fail;
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
        Result::Ok(v, tail) => {
            let m = n - tail.len();
            assert_eq!(tail, &data[m..n]);
            Result::Ok(v, &data[m..])
        }
    };
    assert_eq!(immediate, incremental);
}

#[macro_export]
macro_rules! split_fuzz {
    ($p:ty) => {
        extern crate libfuzzer_sys;

        #[export_name = "rust_fuzzer_test_input"]
        pub extern "C" fn go(data: &[u8]) {
            $crate::split_fuzz::<$p>(data)
        }
    }
}
