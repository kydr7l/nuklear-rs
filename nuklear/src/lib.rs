//! Safe Rust wrapper for the Nuklear immediate mode GUI library.
//!
//! The crate wraps Nuklear (pinned to the commit in `nuklear-sys/nuklear/VERSION`) and has no other runtime dependency than `nuklear-sys`.
//! It has no render backend and no window code. A program runs a full frame without a display,
//! and it reads the result as draw commands (`Context::commands`) or as vertex data (`Context::convert`).
//! The popups, combo boxes, contextual menus, menus and charts are in the module `containers`, not in `window`. The crate root re-exports every public type.
//!
//! # The frame protocol
//!
//! Make one `UserFont` and one `Context` that borrows the font. Then repeat these steps for each frame.
//! Send the input with `Context::input`. Build the windows with `Context::window`. Read the result with `Context::commands`.
//! Call `Context::clear` at the end of the frame.
//!
//! ```
//! use nuklear::*;
//!
//! // A font where every character is 6 pixels wide and 12 pixels high.
//! struct Fixed;
//! impl Font for Fixed {
//!     fn height(&self) -> f32 { 12.0 }
//!     fn width(&self, _height: f32, text: &str) -> f32 { 6.0 * text.chars().count() as f32 }
//!     fn glyph(&self, _height: f32, _codepoint: char, _next: char) -> Glyph {
//!         Glyph { width: 6.0, height: 12.0, xadvance: 6.0, ..Default::default() }
//!     }
//!     fn texture(&self) -> Handle { Handle::id(1) }
//! }
//!
//! let font = UserFont::new(Fixed);
//! let mut ctx = Context::new(&font).unwrap();
//! let mut clicked = false;
//! // Frame 0 moves the mouse and presses the button. Frame 1 releases the button.
//! for down in [true, false] {
//!     ctx.input(|i| {
//!         i.motion(50, 60);
//!         i.button(Button::Left, 50, 60, down);
//!     });
//!     let bounds = Rect { x: 0.0, y: 0.0, w: 300.0, h: 200.0 };
//!     ctx.window("Demo", bounds, PanelFlags::TITLE | PanelFlags::BORDER, |w| {
//!         w.layout_row_dynamic(30.0, 1);
//!         if w.button_label("Click") {
//!             clicked = true;
//!         }
//!     });
//!     let count = ctx.commands().count();
//!     assert!(count > 0);
//!     ctx.clear();
//! }
//! assert!(clicked);
//! ```
//!
//! # The safety model
//!
//! The goal is that no safe code causes undefined behaviour. The wrapper removes each hazard of the C API with one rule.
//!
//! - **Fonts.** Nuklear keeps a pointer to the font. `Context<'f>` borrows the font for `'f`, so the font lives longer than the context.
//!   `UserFont` boxes the `Font` value and passes a thin pointer to the C callbacks.
//! - **Scopes.** Nuklear needs `_begin` and `_end` calls in pairs. Every scope is a closure, for example `Context::window`.
//!   When the closure returns, the `_end` call runs. A panic in the closure aborts the process, so no unwind skips the `_end` call.
//!   A widget method exists only on `Window`, so no widget call happens outside a window.
//! - **Strings.** The wrapper uses the `_text` variants with a length whenever Nuklear has one. Other strings become C strings
//!   that stop at the first NUL byte. A `_text` variant gets at most the first 2^30 bytes of a string.
//! - **Command memory.** `Context::commands` and `Context::convert` return iterators that borrow the context.
//!   The commands cannot outlive the next `Context::clear`. `DrawCommands` borrows the same `Buffer` that `convert` filled.
//! - **Vertex layout.** `VertexLayout::new` checks every element, so Nuklear writes only inside each vertex.
//! - **Buffers.** `Buffer` allocates zeroed memory, so `Buffer::as_bytes` never reads uninitialized bytes.
//! - **Font atlas.** The types `FontAtlasBuilder`, `BakedAtlas` and `FontAtlas` follow the order of the C calls.
//!   `BakedFont` borrows the `FontAtlas`.
//! - **Text edit.** `TextEdit` owns its `nk_text_edit` in a box, so the struct does not move.
//! - **Layout and popups.** The context tracks the state of the layout row and the number of open popups.
//!   The wrapper returns early instead of a call that would fail a Nuklear assertion.
//! - **Numbers.** Sliders, knobs, progress bars and properties return `false` and do not call C when a parameter is not finite
//!   or when a step or a range is not valid.
//! - **Threads.** `Context`, `Window`, `Buffer`, `TextEdit` and the atlas types hold raw pointers. They are not `Send` and not `Sync`.
//!
//! # Aborts
//!
//! These events abort the process. The process stops before Nuklear uses memory that is not valid.
//!
//! - Two windows with the same name in one frame fail a Nuklear assertion.
//! - Every Nuklear assertion calls `nk_rs_assert_fail`. This function prints the expression, the file and the line to stderr, and it aborts.
//!   Assertions stay on in every build profile. The stb_truetype and stb_rect_pack assertions use the same function.
//! - A panic in the body of a scope aborts the process. The scopes are `Context::input`, `Context::window`, and every
//!   `Window` method that takes a closure. `std::panic::catch_unwind` cannot catch this panic.
//!   After an unwind, Nuklear would keep pointers into freed Rust memory, for example the scroll offset of `Window::group_scrolled`.
//! - A panic inside a `Font` method aborts, because the panic cannot unwind through the C code.
//!
//! # The unsafe function
//!
//! The crate has exactly one public `unsafe fn`: `FontAtlasBuilder::add_ttf`. The stb_truetype code in Nuklear reads out of
//! bounds on a malformed font file. The caller must give a well-formed TrueType or OpenType file.
//! `FontAtlasBuilder::add_default` uses the built-in font and is safe.
//!
//! # Known limitations
//!
//! - Nuklear casts `float` to `short` or `int` when it records commands and drags widgets.
//!   For values outside the integer range, this is undefined behaviour by the letter of the C standard.
//!   The wrapper rejects non-finite floats and zero steps where they create such values directly. It does not clamp every geometry input.
//! - Nuklear has signed integer overflow in its int code (the parse and the step of `Window::property_i32`).
//!   The build uses `-fwrapv` on gcc and clang. MSVC has no `-fwrapv`, so the overflow stays undefined there.
//! - After `Context::commands` or `Context::convert`, `Context::window` returns `None` until `Context::clear` runs.
//! - `FontHandle` is sealed. Only the types of this crate implement it.
//! - Float properties (`property_f32`, `property_f64`) return `false` unless `|v| < 2^31` for the minimum, the maximum and the value.
//! - Font heights must be in the range `(0, 512]`. The font functions return `None` for other heights.
//! - The atlas image is 512 or 1024 pixels wide and at most 32768 pixels high. All fonts in one atlas share this space.
//!   Nuklear omits the glyphs that do not fit, and it gives no error.
//! - `Context::convert` rejects a segment count above 65535.
//! - Two windows with the same name in one frame abort the process.
//! - A Nuklear assertion aborts the process and prints the expression.
//! - A panic in a scope body aborts the process.
//! - The `_text` functions get at most the first 2^30 bytes of a string. The cut can split a UTF-8 character.
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

pub mod text_edit;
pub use text_edit::*;

pub mod containers;
pub use containers::*;

pub mod convert;
pub use convert::*;

/// Make a C string from `s`. The function stops at the first NUL byte.
pub(crate) fn cstr(s: &str) -> std::ffi::CString {
    let end = s.find('\0').unwrap_or(s.len());
    std::ffi::CString::new(&s.as_bytes()[..end]).unwrap_or_default()
}

/// The most bytes of a string that the wrapper gives to Nuklear.
/// `nk_draw_text` adds `length + 1` in `int`, and `nk_buffer_alloc` rounds the size up in 32 bits. Both overflow above 2^31 bytes.
const TEXT_LIMIT: usize = 1 << 30;

/// Get the C length of a string of `len` bytes. The length is at most `TEXT_LIMIT`.
fn text_len(len: usize) -> std::ffi::c_int {
    len.min(TEXT_LIMIT) as std::ffi::c_int
}

/// Get the pointer and the length of `s` for a Nuklear `_text` function. The length is at most `TEXT_LIMIT`.
/// An empty string gets a pointer to a static empty C string.
pub(crate) fn text_arg(s: &str) -> (*const std::ffi::c_char, std::ffi::c_int) {
    if s.is_empty() {
        return (c"".as_ptr(), 0);
    }
    (s.as_ptr().cast(), text_len(s.len()))
}

/// A guard that aborts the process when a panic unwinds through it.
///
/// Between a C begin call and its end call, Nuklear holds pointers into Rust memory. An unwind would skip the end call.
/// Then Nuklear keeps the pointers into freed memory, so an abort is the only sound result.
pub(crate) struct AbortOnUnwind;

impl Drop for AbortOnUnwind {
    fn drop(&mut self) {
        if std::thread::panicking() {
            std::process::abort();
        }
    }
}

/// Run `f`, the body of a C scope. A panic in `f` aborts the process.
pub(crate) fn no_unwind<R>(f: impl FnOnce() -> R) -> R {
    let guard = AbortOnUnwind;
    let r = f();
    // The body returned, so the guard has nothing to do. A guard made during an unwind must not abort after a normal return.
    std::mem::forget(guard);
    r
}

#[cfg(test)]
mod tests {
    #[test]
    fn text_len_stops_at_the_limit() {
        assert_eq!(super::text_len(5), 5);
        assert_eq!(super::text_len(super::TEXT_LIMIT), 1 << 30);
        assert_eq!(super::text_len(i32::MAX as usize), 1 << 30);
        assert_eq!(super::text_len(usize::MAX), 1 << 30);
    }
}
