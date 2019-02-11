extern crate cc;

fn main() {
    cc::Build::new()
        .file("lib/sha1.c")
        .file("lib/ubc_check.c")
        .compile("sha1dc_c");
}
