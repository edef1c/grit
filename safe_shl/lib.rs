#![no_std]
//! Shift `lhs` left by `rhs` bits, returning `None` instead
//! if that drops any nonzero bits.

macro_rules! ty {
  ($ty:ident) => {
    pub fn $ty(lhs: $ty, rhs: u32) -> Option<$ty> {
      if rhs <= lhs.leading_zeros() {
        lhs.checked_shl(rhs)
      } else {
        None
      }
    }
  }
}

ty!(u8);
ty!(u16);
ty!(u32);
ty!(u64);
ty!(i8);
ty!(i16);
ty!(i32);
ty!(i64);
