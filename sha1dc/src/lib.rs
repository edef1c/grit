#![no_std]

use core::{mem, fmt};

pub struct Hasher {
    lower: ffi::SHA1_CTX
}

impl Hasher {
    pub fn new() -> Hasher {
        unsafe {
            let mut lower = mem::uninitialized();
            ffi::SHA1DCInit(&mut lower);
            Hasher { lower }
        }
    }
    pub fn update(&mut self, buffer: &[u8]) {
        unsafe {
            ffi::SHA1DCUpdate(&mut self.lower, buffer.as_ptr(), buffer.len())
        }
    }
    pub fn digest(mut self) -> [u8; 20] {
        unsafe {
            let mut buffer: [u8; 20] = mem::uninitialized();
            ffi::SHA1DCFinal(buffer.as_mut_ptr(), &mut self.lower);
            buffer
        }
    }
}

impl fmt::Write for Hasher {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        Ok(self.update(s.as_bytes()))
    }
}

#[test]
fn hello_world() {
    use core::fmt::Write;

    let mut h = Hasher::new();
    write!(h, "hello, world").unwrap();
    let d = [0xb7, 0xe2, 0x3e, 0xc2, 0x9a, 0xf2, 0x2b, 0x0b, 0x4e, 0x41,
             0xda, 0x31, 0xe8, 0x68, 0xd5, 0x72, 0x26, 0x12, 0x1c, 0x84];
    assert_eq!(d, h.digest());
}

#[allow(non_camel_case_types)]
mod ffi {
    type c_int = i32;
    type collision_block_callback = extern "C" fn(byte_offset: u64, ihvin1: *const u32, ihvin2: *const u32, m1: *const u32, m2: *const u32);

    #[repr(C)]
    pub struct SHA1_CTX {
        total: u64,
        ihv: [u32; 5],
        buffer: [u8; 64],
        found_collision: c_int,
        safe_hash: c_int,
        detect_coll: c_int,
        ubc_check: c_int,
        reduced_round_coll: c_int,
        callback: collision_block_callback,

        ihv1: [u32; 5],
        ihv2: [u32; 5],
        m1: [u32; 80],
        m2: [u32; 80],
        states: [[u32; 5]; 80],
    }

    extern "C" {
        pub fn SHA1DCInit(ctx: *mut SHA1_CTX);
        pub fn SHA1DCUpdate(ctx: *mut SHA1_CTX, buf: *const u8, len: usize);
        pub fn SHA1DCFinal(digest: *mut u8, ctx: *mut SHA1_CTX);
    }
}
