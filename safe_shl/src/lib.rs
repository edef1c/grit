#![no_std]

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
