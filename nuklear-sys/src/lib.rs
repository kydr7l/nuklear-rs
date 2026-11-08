//! Raw FFI bindings to Nuklear.
//!
//! The crate compiles `nuklear.h` from the git submodule `nuklear` and generates the Rust declarations with `bindgen` at build time.
//! Every `nk_*` function, type and `NK_*` constant of the header is public here.
//! The submodule pins the upstream commit.
//!
//! The functions are `unsafe`, and Nuklear checks its preconditions with `NK_ASSERT`.
//! A failed assertion calls [`nk_rs_assert_fail`], which prints the expression and aborts the process.
//! Use the `nuklear` crate for a safe interface.

pub use bindings::*;

// The lints do not fit generated code, so the allows cover only this module.
#[allow(
    non_upper_case_globals,
    non_camel_case_types,
    non_snake_case,
    improper_ctypes,
    missing_docs,
    clippy::all,
    clippy::approx_constant,
    rustdoc::broken_intra_doc_links
)]
mod bindings {
    include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
}

use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

/// Nuklear calls this function when an `NK_ASSERT` fails. The function aborts the process.
///
/// # Safety
/// `expr` and `file` must each be null or point to a valid NUL-terminated string.
#[no_mangle]
pub unsafe extern "C" fn nk_rs_assert_fail(expr: *const c_char, file: *const c_char, line: c_int) -> ! {
    let text = |p: *const c_char| {
        if p.is_null() {
            "?".into()
        } else {
            // SAFETY: the caller guarantees a valid NUL-terminated string.
            unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned()
        }
    };
    eprintln!("nuklear assertion failed: {} ({}:{})", text(expr), text(file), line);
    std::process::abort()
}
