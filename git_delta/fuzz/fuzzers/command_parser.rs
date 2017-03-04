#![no_main]
extern crate libfuzzer_sys;
extern crate gulp;
extern crate git_delta;

#[export_name="rust_fuzzer_test_input"]
pub extern fn go(data: &[u8]) {
  gulp::split_fuzz::<git_delta::CommandParser>(data)
}