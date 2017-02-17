#![no_std]

extern crate void;
extern crate safe_shl;
extern crate byteorder;

pub use stateless::*;
pub use incremental::*;

mod stateless;
mod incremental;
