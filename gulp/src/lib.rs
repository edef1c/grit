#![no_std]

extern crate void;
extern crate safe_shl;
extern crate byteorder;

pub use incremental::*;

mod incremental;

#[derive(Debug)]
pub struct Overflow;
