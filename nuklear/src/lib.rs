//! Safe Rust wrapper for the Nuklear immediate mode GUI library.
#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

pub use nuklear_sys as sys;

pub mod types;
pub use types::*;

pub mod font;
pub use font::*;

pub mod context;
pub use context::*;

pub mod window;
pub use window::*;

pub mod command;
pub use command::*;

mod widgets;

/// Make a C string from `s`. The function stops at the first NUL byte.
pub(crate) fn cstr(s: &str) -> std::ffi::CString {
    let end = s.find('\0').unwrap_or(s.len());
    std::ffi::CString::new(&s.as_bytes()[..end]).unwrap_or_default()
}

/// Get the pointer and the length of `s` for a Nuklear `_text` function. The length is at most `i32::MAX`.
/// An empty string gets a pointer to a static empty C string.
pub(crate) fn text_arg(s: &str) -> (*const std::ffi::c_char, std::ffi::c_int) {
    if s.is_empty() {
        return (c"".as_ptr(), 0);
    }
    (s.as_ptr().cast(), s.len().min(i32::MAX as usize) as std::ffi::c_int)
}
