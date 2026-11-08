//! Fonts. Implement the `Font` trait and wrap the value in a `UserFont`.

use crate::{sys, AtlasFormat, Handle, Vec2};
use std::ffi::{c_char, c_int, c_void};
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ops::RangeInclusive;
use std::ptr::NonNull;
use std::sync::Mutex;

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
    /// The maximum height of the font in pixels. `UserFont` reads this value once, in `UserFont::new`.
    fn height(&self) -> f32;
    /// The width of `text` in pixels at the font height `height`.
    fn width(&self, height: f32, text: &str) -> f32;
    /// The drawing data for `codepoint`. The character `next` follows `codepoint`.
    fn glyph(&self, height: f32, codepoint: char, next: char) -> Glyph;
    /// The handle of the font texture. `UserFont` reads this value once, in `UserFont::new`.
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
    /// This pointer comes from `Box::into_raw`. The heap allocation never moves until `Drop`.
    inner: NonNull<UserFontInner>,
}

impl UserFont {
    /// Make a user font from a `Font` value.
    /// The function reads `Font::height` and `Font::texture` once.
    pub fn new<F: Font>(font: F) -> UserFont {
        let height = font.height();
        let texture = font.texture().0;
        let p = Box::into_raw(Box::new(UserFontInner {
            // SAFETY: `nk_user_font` has only integers, floats, optional function pointers and unions. All zero bytes are valid.
            raw: unsafe { std::mem::zeroed() },
            font: Box::new(font),
        }));
        // SAFETY: `p` comes from `Box::into_raw` and no other reference to it exists yet.
        unsafe {
            (*p).raw.userdata.ptr = p as *mut c_void;
            (*p).raw.height = height;
            (*p).raw.width = Some(width_cb);
            (*p).raw.query = Some(query_cb);
            (*p).raw.texture = texture;
        }
        // SAFETY: `Box::into_raw` never returns a null pointer.
        UserFont {
            inner: unsafe { NonNull::new_unchecked(p) },
        }
    }
}

impl Drop for UserFont {
    fn drop(&mut self) {
        // SAFETY: `inner` comes from `Box::into_raw` in `new`, and `drop` runs once.
        drop(unsafe { Box::from_raw(self.inner.as_ptr()) });
    }
}

impl private::Sealed for UserFont {}

impl FontHandle for UserFont {
    fn raw(&self) -> &sys::nk_user_font {
        // SAFETY: `inner` points to a live allocation until `drop`. The allocation does not change after `new`.
        unsafe { &(*self.inner.as_ptr()).raw }
    }
}

unsafe extern "C" fn width_cb(
    h: sys::nk_handle,
    height: f32,
    text: *const c_char,
    len: c_int,
) -> f32 {
    // SAFETY: `UserFont::new` stores the address of a live `UserFontInner` in `h.ptr`. The `UserFont` outlives every call.
    let font = unsafe { &(*(h.ptr as *const UserFontInner)).font };
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
    // SAFETY: `UserFont::new` stores the address of a live `UserFontInner` in `h.ptr`. The `UserFont` outlives every call.
    let font = unsafe { &(*(h.ptr as *const UserFontInner)).font };
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

/// The largest font height in pixels that the atlas accepts.
/// This bound keeps the C integer math safe. stb_truetype turns the height into integer sizes and array indices.
/// The bound does not promise that the glyphs fit in the atlas image.
const MAX_FONT_HEIGHT: f32 = 512.0;

fn height_ok(height: f32) -> bool {
    height.is_finite() && height > 0.0 && height <= MAX_FONT_HEIGHT
}

/// Free an atlas allocation.
///
/// # Safety
/// `atlas` must come from `Box::into_raw` in `FontAtlasBuilder::new`. The caller must free `pixel` first.
/// Nobody may use the pointer after this call.
unsafe fn free_atlas(atlas: NonNull<sys::nk_font_atlas>) {
    // SAFETY: the caller gives a live atlas. `clear` frees the fonts, the glyphs and the font configs, which include the TTF copies.
    // `clear` does not free `pixel`. It zeroes the struct, so no pointer stays in it.
    unsafe { sys::nk_font_atlas_clear(atlas.as_ptr()) };
    // SAFETY: the pointer comes from `Box::into_raw`, and the C code keeps no pointer into the struct after `clear`.
    drop(unsafe { Box::from_raw(atlas.as_ptr()) });
}

/// The identity of a font in one atlas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontId(usize);

/// The white pixel that Nuklear uses to draw shapes without a texture.
#[derive(Clone, Copy, Debug)]
pub struct NullTexture {
    /// The handle of the atlas texture.
    pub texture: Handle,
    /// The texture coordinates of the white pixel.
    pub uv: Vec2,
}

/// The most glyphs that one font bakes. Nuklear counts the glyphs of a font in an `int`.
const MAX_GLYPHS: u64 = 0x110000;

/// The settings of one font in the atlas. `Default` gives the Nuklear defaults.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FontConfig {
    /// Round the advance of every glyph to a whole pixel.
    pub pixel_snap: bool,
    /// Put each glyph on whole pixels at the size of its atlas texels after the bake.
    /// The linear sampler then does not blur small text.
    pub snap_glyphs: bool,
    /// The code points to bake. An empty list bakes `'\u{20}'..='\u{ff}'`.
    /// A character that is not in a range draws as `?`.
    pub ranges: Vec<RangeInclusive<char>>,
}

impl FontConfig {
    /// Make the flat, zero-terminated range list that Nuklear reads. The function returns `None` when a range is empty,
    /// when a range starts at `'\0'` (Nuklear reads 0 as the end of the list), or when the ranges have more than 0x110000 glyphs in total.
    fn flat_ranges(&self) -> Option<Option<Box<[u32]>>> {
        if self.ranges.is_empty() {
            return Some(None);
        }
        let mut flat = Vec::with_capacity(self.ranges.len() * 2 + 1);
        let mut total: u64 = 0;
        for r in &self.ranges {
            let (lo, hi) = (*r.start() as u32, *r.end() as u32);
            if lo == 0 || lo > hi {
                return None;
            }
            total += u64::from(hi - lo) + 1;
            flat.extend([lo, hi]);
        }
        if total > MAX_GLYPHS {
            return None;
        }
        flat.push(0);
        Some(Some(flat.into_boxed_slice()))
    }
}

/// The first step of the font pipeline. Add fonts, then call `bake`.
pub struct FontAtlasBuilder {
    /// This pointer comes from `Box::into_raw`. Nuklear stores pointers into the allocation, so it must not move.
    atlas: NonNull<sys::nk_font_atlas>,
    /// The fonts in the order of `add_default` and `add_ttf`. Nuklear owns them.
    fonts: Vec<*mut sys::nk_font>,
    /// The glyph ranges of the fonts. Nuklear keeps a pointer to each list and reads it when it looks up a glyph.
    ranges: Vec<Box<[u32]>>,
    /// For each font in `fonts`: `true` when `BakedAtlas::end` snaps the glyphs of the font.
    snaps: Vec<bool>,
}

impl Default for FontAtlasBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FontAtlasBuilder {
    /// Make an empty builder.
    pub fn new() -> FontAtlasBuilder {
        let p = Box::into_raw(Box::<sys::nk_font_atlas>::default());
        // SAFETY: `p` comes from `Box::into_raw`, and no other reference to it exists. `init_default` sets the C allocators.
        unsafe {
            sys::nk_font_atlas_init_default(p);
            sys::nk_font_atlas_begin(p);
        }
        FontAtlasBuilder {
            // SAFETY: `Box::into_raw` never returns a null pointer.
            atlas: unsafe { NonNull::new_unchecked(p) },
            fonts: Vec::new(),
            ranges: Vec::new(),
            snaps: Vec::new(),
        }
    }

    /// Make the C config for one font. The function returns `None` when `height` or `config` is not valid.
    fn c_config(&mut self, height: f32, config: &FontConfig) -> Option<sys::nk_font_config> {
        if !height_ok(height) {
            return None;
        }
        let flat = config.flat_ranges()?;
        // SAFETY: `nk_font_config` only fills a struct with constants and the default range pointer.
        let mut cfg = unsafe { sys::nk_font_config(height) };
        cfg.pixel_snap = config.pixel_snap.into();
        if let Some(flat) = flat {
            cfg.range = flat.as_ptr();
            // The box never moves, and `ranges` lives as long as the atlas, so the pointer stays valid.
            self.ranges.push(flat);
        }
        Some(cfg)
    }

    fn push(&mut self, font: *mut sys::nk_font, snap: bool) -> Option<FontId> {
        if font.is_null() {
            return None;
        }
        self.fonts.push(font);
        self.snaps.push(snap);
        Some(FontId(self.fonts.len() - 1))
    }

    /// Add the built-in font. The function returns `None` when `height` is not finite or not in `0 < height <= 512`.
    /// The height range protects the integer math in stb_truetype.
    /// The atlas image is 512 pixels wide, or 1024 pixels wide when the atlas has more than 1000 glyphs.
    /// The height of the image is at most 32768 pixels.
    /// Nuklear omits the glyphs that do not fit, and it gives no error.
    /// All fonts in one atlas share this space.
    pub fn add_default(&mut self, height: f32) -> Option<FontId> {
        self.add_default_with(height, &FontConfig::default())
    }

    /// Add the built-in font with a `FontConfig`. See `add_default` for the height rule and the image size.
    /// The function also returns `None` when a range in `config` is empty, starts at `'\0'`, or when the ranges have more than 0x110000 glyphs.
    pub fn add_default_with(&mut self, height: f32, config: &FontConfig) -> Option<FontId> {
        let cfg = self.c_config(height, config)?;
        // Nuklear decompresses the built-in font with C globals (`nk__barrier`, `nk__dout`). Two threads would race on them.
        static DECOMPRESS: Mutex<()> = Mutex::new(());
        let _guard = DECOMPRESS.lock().unwrap_or_else(|e| e.into_inner());
        // SAFETY: `atlas` is live and in the state after `begin`. Nuklear copies `cfg`, and `ranges` keeps the range list alive. The lock above serializes the C globals.
        let font = unsafe { sys::nk_font_atlas_add_default(self.atlas.as_ptr(), height, &cfg) };
        self.push(font, config.snap_glyphs)
    }

    /// Add a font from TrueType or OpenType data. The atlas copies the bytes before the function returns.
    /// The function returns `None` when `height` is not finite or not in `0 < height <= 512`, or when `ttf` is empty.
    /// The height range protects the integer math in stb_truetype.
    /// The atlas image is 512 pixels wide, or 1024 pixels wide when the atlas has more than 1000 glyphs.
    /// The height of the image is at most 32768 pixels.
    /// Nuklear omits the glyphs that do not fit, and it gives no error.
    /// All fonts in one atlas share this space.
    ///
    /// `ttf` must be one complete, well-formed TrueType or OpenType file, or a collection. The atlas uses font 0 of a collection.
    /// stb_truetype trusts the offsets in the data and does not check them against the length.
    /// Bad data causes out-of-bounds reads when `bake` runs.
    /// Font data from `include_bytes!` of a font file meets this contract.
    pub fn add_ttf(&mut self, ttf: &[u8], height: f32) -> Option<FontId> {
        self.add_ttf_with(ttf, height, &FontConfig::default())
    }

    /// Add a font from TrueType or OpenType data with a `FontConfig`. See `add_ttf` for the height rule and the image size.
    /// The function also returns `None` when a range in `config` is empty, starts at `'\0'`, or when the ranges have more than 0x110000 glyphs.
    ///
    /// The same contract as `add_ttf` applies to `ttf`.
    pub fn add_ttf_with(&mut self, ttf: &[u8], height: f32, config: &FontConfig) -> Option<FontId> {
        // An empty `ttf` would reach `NK_ASSERT(size)` in Nuklear, which aborts.
        if ttf.is_empty() {
            return None;
        }
        let cfg = self.c_config(height, config)?;
        // SAFETY: `atlas` is live. `ttf` is readable for its length, and Nuklear only reads it. The contract of this function makes `ttf` well-formed font data.
        // Nuklear copies `cfg`, and `ranges` keeps the range list alive.
        let font = unsafe {
            sys::nk_font_atlas_add_from_memory(
                self.atlas.as_ptr(),
                ttf.as_ptr() as *mut c_void,
                ttf.len(),
                height,
                &cfg,
            )
        };
        self.push(font, config.snap_glyphs)
    }

    /// Draw the glyphs into an image. The function returns `None` when no font was added or when Nuklear fails.
    /// The atlas image is 512 pixels wide, or 1024 pixels wide when the atlas has more than 1000 glyphs.
    /// The height of the image is at most 32768 pixels.
    /// Nuklear omits the glyphs that do not fit, and it gives no error.
    /// All fonts in one atlas share this space.
    pub fn bake(self, format: AtlasFormat) -> Option<BakedAtlas> {
        // Without a font, Nuklear adds a default font on its own. This crate wants an explicit font.
        if self.fonts.is_empty() {
            return None;
        }
        let (mut w, mut h) = (0, 0);
        // SAFETY: `atlas` is live. `w` and `h` are writable. On failure Nuklear frees its temporary memory and sets `pixel` to null.
        // stb_truetype reads the font data here. The contract of `add_ttf` makes that data well formed.
        let px = unsafe {
            sys::nk_font_atlas_bake(self.atlas.as_ptr(), &mut w, &mut h, format.raw() as _)
        };
        if px.is_null() || w <= 0 || h <= 0 {
            return None; // `self` drops here and clears the atlas.
        }
        let me = ManuallyDrop::new(self); // The new value takes the atlas. The builder `Drop` must not run.
        Some(BakedAtlas {
            atlas: me.atlas,
            // SAFETY: `me` is never used or dropped again, so this read moves the `Vec` out once.
            fonts: unsafe { std::ptr::read(&me.fonts) },
            // SAFETY: the same single move as `fonts`.
            ranges: unsafe { std::ptr::read(&me.ranges) },
            // SAFETY: the same single move as `fonts`.
            snaps: unsafe { std::ptr::read(&me.snaps) },
            width: w as u32,
            height: h as u32,
            bytes_per_pixel: if matches!(format, AtlasFormat::Rgba32) {
                4
            } else {
                1
            },
        })
    }
}

impl Drop for FontAtlasBuilder {
    fn drop(&mut self) {
        // SAFETY: `atlas` comes from `Box::into_raw`, and `drop` runs once. `pixel` is null here: `begin` and a failed `bake` leave it null.
        unsafe { free_atlas(self.atlas) };
    }
}

/// An atlas with an image. Upload the pixels to a texture, then call `end`.
pub struct BakedAtlas {
    /// This pointer comes from `Box::into_raw`. The fonts point back into the allocation.
    atlas: NonNull<sys::nk_font_atlas>,
    fonts: Vec<*mut sys::nk_font>,
    ranges: Vec<Box<[u32]>>,
    /// The snap flags of the fonts, as in `FontAtlasBuilder`.
    snaps: Vec<bool>,
    width: u32,
    height: u32,
    bytes_per_pixel: usize,
}

impl BakedAtlas {
    /// The width of the image in pixels.
    pub fn width(&self) -> u32 {
        self.width
    }
    /// The height of the image in pixels.
    pub fn height(&self) -> u32 {
        self.height
    }
    /// The pixels of the image. The format is Alpha8 (1 byte per pixel) or Rgba32 (4 bytes per pixel).
    pub fn pixels(&self) -> &[u8] {
        let len = self.width as usize * self.height as usize * self.bytes_per_pixel;
        // SAFETY: `bake` allocated `pixel` with `width * height * bytes_per_pixel` bytes. It stays valid until `end` or `drop`, and both consume `self`.
        unsafe { std::slice::from_raw_parts((*self.atlas.as_ptr()).pixel as *const u8, len) }
    }
    /// Finish the atlas. `texture` is the handle of the texture that holds the pixels.
    /// The result has the fonts and the white pixel of the texture.
    pub fn end(self, texture: Handle) -> (FontAtlas, NullTexture) {
        let me = ManuallyDrop::new(self); // The new value takes the atlas. The `Drop` of `BakedAtlas` must not run.
        let mut null = sys::nk_draw_null_texture::default();
        // SAFETY: `atlas` is live and baked. `end` frees `pixel` through the temporary allocator and sets it to null.
        // `cleanup` frees the TTF copies. The fonts, the glyphs and the configs stay until `FontAtlas::drop`.
        unsafe {
            sys::nk_font_atlas_end(me.atlas.as_ptr(), texture.0, &mut null);
            sys::nk_font_atlas_cleanup(me.atlas.as_ptr());
        }
        // SAFETY: `me` is never used or dropped again, so this read moves the `Vec` out once.
        let snaps = unsafe { std::ptr::read(&me.snaps) };
        for (&font, _) in me.fonts.iter().zip(&snaps).filter(|(_, &snap)| snap) {
            // SAFETY: `font` is a baked font of the live atlas. Its glyphs are `glyph_count` values in the glyph array of the atlas.
            // Nothing else holds a reference to them, because no `BakedFont` exists before `end` returns.
            let glyphs = unsafe {
                std::slice::from_raw_parts_mut((*font).glyphs, (*font).info.glyph_count as usize)
            };
            for g in glyphs {
                let (w, h) = (g.x1 - g.x0, g.y1 - g.y0);
                g.x0 = g.x0.round();
                g.y0 = g.y0.round();
                g.x1 = g.x0 + w;
                g.y1 = g.y0 + h;
                g.w = w;
                g.h = h;
                g.xadvance = g.xadvance.round();
            }
        }
        let atlas = FontAtlas {
            atlas: me.atlas,
            // SAFETY: `me` is never used or dropped again, so this read moves the `Vec` out once.
            fonts: unsafe { std::ptr::read(&me.fonts) },
            // SAFETY: the same single move as `fonts`.
            ranges: unsafe { std::ptr::read(&me.ranges) },
        };
        (
            atlas,
            NullTexture {
                texture: Handle(null.texture),
                uv: null.uv.into(),
            },
        )
    }
}

impl Drop for BakedAtlas {
    fn drop(&mut self) {
        let a = self.atlas.as_ptr();
        // SAFETY: `atlas` is live. `bake` allocated `pixel` with the temporary allocator, and `clear` does not free it.
        // This call frees it once, with the allocator that made it. `end` frees it too, but `end` consumes `self` without this `drop`.
        unsafe {
            let pixel = (*a).pixel;
            if !pixel.is_null() {
                if let Some(free) = (*a).temporary.free {
                    free((*a).temporary.userdata, pixel);
                }
            }
        }
        // SAFETY: `atlas` comes from `Box::into_raw`, and `drop` runs once. `pixel` is freed above.
        unsafe { free_atlas(self.atlas) };
    }
}

/// A finished atlas. It owns the fonts. `BakedFont` borrows from it.
///
/// The atlas must live longer than every context that uses one of its fonts.
/// This code does not compile, because it drops the atlas before the last use of the context:
///
/// ```compile_fail,E0505
/// use nuklear::*;
/// let mut b = FontAtlasBuilder::new();
/// let id = b.add_default(13.0).unwrap();
/// let (atlas, _) = b.bake(AtlasFormat::Alpha8).unwrap().end(Handle::id(1));
/// let font = atlas.font(id).unwrap();
/// let mut ctx = Context::new(&font).unwrap();
/// drop(atlas);
/// ctx.clear();
/// ```
pub struct FontAtlas {
    /// This pointer comes from `Box::into_raw`. The fonts point back into the allocation.
    atlas: NonNull<sys::nk_font_atlas>,
    fonts: Vec<*mut sys::nk_font>,
    /// The glyph ranges. `nk_font_find_glyph` reads them on every text draw, so they live as long as the fonts.
    #[allow(dead_code)]
    ranges: Vec<Box<[u32]>>,
}

impl FontAtlas {
    /// Get a font. The function returns `None` when `id` is not a valid index of the fonts in this atlas.
    pub fn font(&self, id: FontId) -> Option<BakedFont<'_>> {
        let font = NonNull::new(*self.fonts.get(id.0)?)?;
        Some(BakedFont {
            font,
            _atlas: PhantomData,
        })
    }
    /// The number of fonts.
    pub fn len(&self) -> usize {
        self.fonts.len()
    }
    /// Check if the atlas has no font.
    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }
}

impl Drop for FontAtlas {
    fn drop(&mut self) {
        // SAFETY: `atlas` comes from `Box::into_raw`, and `drop` runs once. `end` freed `pixel` and set it to null.
        // Each `BakedFont` borrows the atlas, so none is alive now.
        unsafe { free_atlas(self.atlas) };
    }
}

/// A font in a `FontAtlas`. Use it with `Context::new`.
pub struct BakedFont<'a> {
    /// A raw pointer, not a reference. It keeps the provenance of the allocation that C made, and `'a` limits its life.
    font: NonNull<sys::nk_font>,
    _atlas: PhantomData<&'a FontAtlas>,
}

impl<'a> BakedFont<'a> {
    /// The height of the font in pixels.
    pub fn height(&self) -> f32 {
        self.raw().height
    }
    /// A copy of the font that Nuklear draws at `height` pixels. The copy uses the same baked glyphs.
    /// The copy borrows the atlas like `BakedFont`.
    pub fn at_height(&self, height: f32) -> ScaledFont<'a> {
        let mut raw = *self.raw();
        raw.height = height;
        ScaledFont {
            raw,
            _atlas: PhantomData,
        }
    }
    /// The baked glyph for `c`. The function returns `None` when the font has no glyph for `c`, so Nuklear would draw the fallback glyph.
    pub fn glyph(&self, c: char) -> Option<Glyph> {
        // SAFETY: `font` points into the fonts of a live atlas, because `'a` borrows the `FontAtlas`. The function only reads.
        let g = unsafe { sys::nk_font_find_glyph(self.font.as_ptr(), c as u32).as_ref()? };
        // The function returns the fallback glyph for a missing code point. It carries the code point of the fallback.
        (g.codepoint == c as u32).then_some(Glyph {
            uv: [Vec2 { x: g.u0, y: g.v0 }, Vec2 { x: g.u1, y: g.v1 }],
            offset: Vec2 { x: g.x0, y: g.y0 },
            width: g.w,
            height: g.h,
            xadvance: g.xadvance,
        })
    }
}

impl private::Sealed for BakedFont<'_> {}

impl FontHandle for BakedFont<'_> {
    fn raw(&self) -> &sys::nk_user_font {
        // SAFETY: `font` points into the fonts of a live atlas, because `'a` borrows the `FontAtlas`.
        // `end` set `handle` for each font, and nothing changes it after that.
        unsafe { &(*self.font.as_ptr()).handle }
    }
}

/// A `BakedFont` that Nuklear draws at another height. Make it with `BakedFont::at_height`.
///
/// The width and glyph callbacks of Nuklear take the height as a parameter.
/// They scale the baked glyphs by the ratio of that height to the baked height.
/// So the copy needs no new bake. Like `BakedFont`, the value borrows the `FontAtlas`.
pub struct ScaledFont<'a> {
    raw: sys::nk_user_font,
    _atlas: PhantomData<&'a FontAtlas>,
}

impl ScaledFont<'_> {
    /// The height of the font in pixels.
    pub fn height(&self) -> f32 {
        self.raw.height
    }
}

impl private::Sealed for ScaledFont<'_> {}

impl FontHandle for ScaledFont<'_> {
    fn raw(&self) -> &sys::nk_user_font {
        &self.raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixed;
    impl Font for Fixed {
        fn height(&self) -> f32 {
            12.0
        }
        fn width(&self, _h: f32, t: &str) -> f32 {
            6.0 * t.chars().count() as f32
        }
        fn glyph(&self, _h: f32, _c: char, _n: char) -> Glyph {
            Glyph {
                width: 6.0,
                height: 12.0,
                xadvance: 6.0,
                ..Default::default()
            }
        }
        fn texture(&self) -> Handle {
            Handle::id(3)
        }
    }

    #[test]
    fn width_callback_reaches_trait() {
        let f = UserFont::new(Fixed);
        let raw = f.raw();
        // SAFETY: the callback and the userdata come from the same `UserFont`, and the text is valid for 3 bytes.
        let w = unsafe { (raw.width.unwrap())(raw.userdata, 12.0, c"abc".as_ptr(), 3) };
        assert_eq!(w, 18.0);
        assert_eq!(raw.height, 12.0);
        assert_eq!(Handle(raw.texture).as_id(), 3);
    }
    #[test]
    fn query_callback_fills_glyph() {
        let f = UserFont::new(Fixed);
        let raw = f.raw();
        let mut g = sys::nk_user_font_glyph::default();
        // SAFETY: the callback and the userdata come from the same `UserFont`, and `g` is writable.
        unsafe { (raw.query.unwrap())(raw.userdata, 12.0, &mut g, 'a' as u32, 0) };
        assert_eq!(g.xadvance, 6.0);
    }
    #[test]
    fn null_text_is_empty() {
        let f = UserFont::new(Fixed);
        let raw = f.raw();
        // SAFETY: the callback and the userdata come from the same `UserFont`. A null text is allowed.
        assert_eq!(
            unsafe { (raw.width.unwrap())(raw.userdata, 12.0, std::ptr::null(), 5) },
            0.0
        );
    }

    use std::cell::RefCell;
    thread_local! { static SEEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) }; }
    struct Rec;
    impl Font for Rec {
        fn height(&self) -> f32 {
            1.0
        }
        fn width(&self, _h: f32, t: &str) -> f32 {
            SEEN.with(|s| s.borrow_mut().push(t.to_string()));
            t.len() as f32
        }
        fn glyph(&self, _h: f32, c: char, n: char) -> Glyph {
            SEEN.with(|s| s.borrow_mut().push(format!("{c}{n}")));
            Glyph::default()
        }
        fn texture(&self) -> Handle {
            Handle::id(0)
        }
    }
    fn make() -> UserFont {
        UserFont::new(Fixed)
    }

    #[test]
    fn moved_font_still_works() {
        let v = [make(), make()];
        let raw = v[0].raw();
        // SAFETY: the callback and the userdata come from the same `UserFont`, and the text is valid for 3 bytes.
        assert_eq!(
            unsafe { (raw.width.unwrap())(raw.userdata, 12.0, c"abc".as_ptr(), 3) },
            18.0
        );
    }
    #[test]
    fn null_glyph_returns() {
        let f = make();
        let raw = f.raw();
        // SAFETY: the callback and the userdata come from the same `UserFont`. A null glyph is allowed.
        unsafe { (raw.query.unwrap())(raw.userdata, 12.0, std::ptr::null_mut(), 'a' as u32, 0) };
    }
    #[test]
    fn zero_and_negative_len_are_empty() {
        let f = UserFont::new(Rec);
        let raw = f.raw();
        SEEN.with(|s| s.borrow_mut().clear());
        for len in [0, -4] {
            // SAFETY: the callback and the userdata come from the same `UserFont`. The callback reads no byte when `len <= 0`.
            assert_eq!(
                unsafe { (raw.width.unwrap())(raw.userdata, 1.0, c"abc".as_ptr(), len) },
                0.0
            );
        }
        SEEN.with(|s| assert_eq!(*s.borrow(), ["", ""]));
    }
    #[test]
    fn invalid_utf8_is_lossy() {
        let f = UserFont::new(Rec);
        let raw = f.raw();
        SEEN.with(|s| s.borrow_mut().clear());
        let bytes = [b'a', 0xFF, b'b'];
        // SAFETY: the callback and the userdata come from the same `UserFont`, and `bytes` has 3 readable bytes.
        unsafe { (raw.width.unwrap())(raw.userdata, 1.0, bytes.as_ptr() as *const c_char, 3) };
        SEEN.with(|s| assert_eq!(*s.borrow(), ["a\u{FFFD}b"]));
    }
    #[test]
    fn invalid_codepoint_is_replacement() {
        let f = UserFont::new(Rec);
        let raw = f.raw();
        SEEN.with(|s| s.borrow_mut().clear());
        let mut g = sys::nk_user_font_glyph::default();
        // SAFETY: the callback and the userdata come from the same `UserFont`, and `g` is writable.
        unsafe { (raw.query.unwrap())(raw.userdata, 1.0, &mut g, 0xD800, 0x110000) };
        SEEN.with(|s| assert_eq!(*s.borrow(), ["\u{FFFD}\u{FFFD}"]));
    }
}
