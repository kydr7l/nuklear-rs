//! Raw FFI bindings to Nuklear.
#![allow(non_upper_case_globals, non_camel_case_types, non_snake_case, improper_ctypes)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

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
