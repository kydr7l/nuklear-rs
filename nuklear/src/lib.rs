//! Safe Rust wrapper for the Nuklear immediate mode GUI library.
#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

pub use nuklear_sys as sys;

pub mod types;
pub use types::*;

pub mod font;
pub use font::*;

/// Make a C string from `s`. The function stops at the first NUL byte.
#[allow(dead_code)] // Later tasks call this function.
pub(crate) fn cstr(s: &str) -> std::ffi::CString {
    let end = s.find('\0').unwrap_or(s.len());
    std::ffi::CString::new(&s.as_bytes()[..end]).unwrap_or_default()
}
