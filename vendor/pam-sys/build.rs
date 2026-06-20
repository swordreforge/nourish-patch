//! Patched pam-sys build: ship prebuilt bindings instead of running bindgen.
//!
//! Upstream pam-sys runs bindgen 0.69 at build time, which fails under clang 22
//! with clang-sys "a libclang shared library is not loaded on this thread".
//! The PAM bindings are stable/deterministic, so we check them in (`bindings.rs`)
//! and just copy them into OUT_DIR.

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=pam");
    if cfg!(target_os = "linux") {
        println!("cargo:rustc-link-lib=pam_misc");
    }
    println!("cargo:rerun-if-changed=bindings.rs");

    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    fs::copy("bindings.rs", out.join("bindings.rs")).expect("copy prebuilt pam bindings");
}
