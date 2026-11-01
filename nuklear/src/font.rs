//! Fonts. Implement the `Font` trait and wrap the value in a `UserFont`.

use crate::{sys, Handle, Vec2};
use std::ffi::{c_char, c_int, c_void};

/// The drawing data of one glyph.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Glyph {
    /// The texture coordinates of the glyph. The values are the top left and the bottom right corner.
    pub uv: [Vec2; 2],
    /// The offset between the top left corner of the cell and the glyph.
    pub offset: Vec2,
    /// The width of the glyph.
    pub width: f32,
    /// The height of the glyph.
    pub height: f32,
    /// The distance to the next glyph.
    pub xadvance: f32,
}

/// A font that Nuklear can use. Nuklear calls these methods when it measures and draws text.
pub trait Font: 'static {
    /// The maximum height of the font in pixels.
    fn height(&self) -> f32;
    /// The width of `text` in pixels at the font height `height`.
    fn width(&self, height: f32, text: &str) -> f32;
    /// The drawing data for `codepoint`. The character `next` follows `codepoint`.
    fn glyph(&self, height: f32, codepoint: char, next: char) -> Glyph;
    /// The handle of the font texture.
    fn texture(&self) -> Handle;
}

mod private {
    pub trait Sealed {}
}

/// A value that gives Nuklear a font. Only types of this crate implement this trait.
pub trait FontHandle: private::Sealed {
    /// Get the C font. This method is not part of the stable API.
    #[doc(hidden)]
    fn raw(&self) -> &sys::nk_user_font;
}

struct UserFontInner {
    raw: sys::nk_user_font,
    font: Box<dyn Font>,
}

/// A font that calls a Rust `Font` value. The C font data stays at a fixed address.
pub struct UserFont {
    inner: Box<UserFontInner>,
}

impl UserFont {
    /// Make a user font from a `Font` value.
    pub fn new<F: Font>(font: F) -> UserFont {
        let height = font.height();
        let texture = font.texture().0;
        let mut inner = Box::new(UserFontInner {
            // SAFETY: `nk_user_font` has only integers, floats, optional function pointers and unions. All zero bytes are valid.
            raw: unsafe { std::mem::zeroed() },
            font: Box::new(font),
        });
        // The inner box does not move after this line, so the address stays valid.
        let font_ptr: *const Box<dyn Font> = &inner.font;
        inner.raw.userdata.ptr = font_ptr as *mut c_void;
        inner.raw.height = height;
        inner.raw.width = Some(width_cb);
        inner.raw.query = Some(query_cb);
        inner.raw.texture = texture;
        UserFont { inner }
    }
}

impl private::Sealed for UserFont {}

impl FontHandle for UserFont {
    fn raw(&self) -> &sys::nk_user_font {
        &self.inner.raw
    }
}

unsafe extern "C" fn width_cb(h: sys::nk_handle, height: f32, text: *const c_char, len: c_int) -> f32 {
    // SAFETY: `UserFont::new` stores the address of a live `Box<dyn Font>` in `h.ptr`. The `UserFont` outlives every call.
    let font = unsafe { &*(h.ptr as *const Box<dyn Font>) };
    if text.is_null() || len <= 0 {
        return font.width(height, "");
    }
    // SAFETY: the caller gives a pointer to `len` readable bytes, and `len` is positive.
    let bytes = unsafe { std::slice::from_raw_parts(text as *const u8, len as usize) };
    font.width(height, &String::from_utf8_lossy(bytes))
}

unsafe extern "C" fn query_cb(
    h: sys::nk_handle,
    height: f32,
    glyph: *mut sys::nk_user_font_glyph,
    codepoint: sys::nk_rune,
    next: sys::nk_rune,
) {
    if glyph.is_null() {
        return;
    }
    // SAFETY: `UserFont::new` stores the address of a live `Box<dyn Font>` in `h.ptr`. The `UserFont` outlives every call.
    let font = unsafe { &*(h.ptr as *const Box<dyn Font>) };
    let c = char::from_u32(codepoint).unwrap_or(char::REPLACEMENT_CHARACTER);
    let n = char::from_u32(next).unwrap_or(char::REPLACEMENT_CHARACTER);
    let g = font.glyph(height, c, n);
    // SAFETY: `glyph` is not null, and the caller gives a pointer to a writable glyph.
    unsafe {
        *glyph = sys::nk_user_font_glyph {
            uv: [g.uv[0].into(), g.uv[1].into()],
            offset: g.offset.into(),
            width: g.width,
            height: g.height,
            xadvance: g.xadvance,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed;
    impl Font for Fixed {
        fn height(&self) -> f32 { 12.0 }
        fn width(&self, _h: f32, t: &str) -> f32 { 6.0 * t.chars().count() as f32 }
        fn glyph(&self, _h: f32, _c: char, _n: char) -> Glyph { Glyph { width: 6.0, height: 12.0, xadvance: 6.0, ..Default::default() } }
        fn texture(&self) -> Handle { Handle::id(3) }
    }

    #[test] fn width_callback_reaches_trait() {
        let f = UserFont::new(Fixed);
        let raw = f.raw();
        // SAFETY: the callback and the userdata come from the same `UserFont`, and the text is valid for 3 bytes.
        let w = unsafe { (raw.width.unwrap())(raw.userdata, 12.0, c"abc".as_ptr(), 3) };
        assert_eq!(w, 18.0);
        assert_eq!(raw.height, 12.0);
        assert_eq!(Handle(raw.texture).as_id(), 3);
    }
    #[test] fn query_callback_fills_glyph() {
        let f = UserFont::new(Fixed);
        let raw = f.raw();
        let mut g = sys::nk_user_font_glyph::default();
        // SAFETY: the callback and the userdata come from the same `UserFont`, and `g` is writable.
        unsafe { (raw.query.unwrap())(raw.userdata, 12.0, &mut g, 'a' as u32, 0) };
        assert_eq!(g.xadvance, 6.0);
    }
    #[test] fn null_text_is_empty() {
        let f = UserFont::new(Fixed);
        let raw = f.raw();
        // SAFETY: the callback and the userdata come from the same `UserFont`. A null text is allowed.
        assert_eq!(unsafe { (raw.width.unwrap())(raw.userdata, 12.0, std::ptr::null(), 5) }, 0.0);
    }
}
