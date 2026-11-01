//! The draw commands of a finished frame.

use crate::{sys, Color, Context, Handle, Image, Vec2i};
use std::borrow::Cow;
use std::marker::PhantomData;
use std::ptr::addr_of;

/// One draw command. A renderer reads these commands after the frame is built.
///
/// All positions and sizes use the types of the C structs. Nuklear gives them as integers in pixels.
/// A command borrows the context, so the context cannot change while a command lives.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)] // The fields have the names and the meaning of the fields of the C structs.
pub enum Command<'a> {
    /// Set the clip rectangle.
    Scissor { x: i16, y: i16, w: u16, h: u16 },
    /// Draw a line.
    Line { thickness: u16, begin: Vec2i, end: Vec2i, color: Color },
    /// Draw a curve.
    Curve { thickness: u16, begin: Vec2i, end: Vec2i, ctrl: [Vec2i; 2], color: Color },
    /// Draw a rectangle outline.
    Rect { rounding: u16, thickness: u16, x: i16, y: i16, w: u16, h: u16, color: Color },
    /// Draw a filled rectangle.
    RectFilled { rounding: u16, x: i16, y: i16, w: u16, h: u16, color: Color },
    /// Draw a filled rectangle with one color for each side.
    RectMultiColor { x: i16, y: i16, w: u16, h: u16, left: Color, top: Color, bottom: Color, right: Color },
    /// Draw a circle outline.
    Circle { thickness: u16, x: i16, y: i16, w: u16, h: u16, color: Color },
    /// Draw a filled circle.
    CircleFilled { x: i16, y: i16, w: u16, h: u16, color: Color },
    /// Draw an arc outline.
    Arc { cx: i16, cy: i16, r: u16, thickness: u16, a: [f32; 2], color: Color },
    /// Draw a filled arc.
    ArcFilled { cx: i16, cy: i16, r: u16, a: [f32; 2], color: Color },
    /// Draw a triangle outline.
    Triangle { thickness: u16, a: Vec2i, b: Vec2i, c: Vec2i, color: Color },
    /// Draw a filled triangle.
    TriangleFilled { a: Vec2i, b: Vec2i, c: Vec2i, color: Color },
    /// Draw a polygon outline.
    Polygon { color: Color, thickness: u16, points: &'a [Vec2i] },
    /// Draw a filled polygon.
    PolygonFilled { color: Color, points: &'a [Vec2i] },
    /// Draw a line through the points.
    Polyline { color: Color, thickness: u16, points: &'a [Vec2i] },
    /// Draw text. Each invalid UTF-8 byte sequence of the text becomes U+FFFD.
    Text { font: Handle, font_height: f32, background: Color, foreground: Color, x: i16, y: i16, w: u16, h: u16, text: Cow<'a, str> },
    /// Draw an image.
    Image { x: i16, y: i16, w: u16, h: u16, img: Image, color: Color },
    /// A custom command or a command of an unknown type. The wrapper has no data for it.
    Custom,
}

/// The iterator over the draw commands of the context. Get it from `Context::commands`.
///
/// Nuklear stores the commands in one buffer, and each command has a different size.
/// Only `nk__begin` and `nk__next` can find the start of the next command.
/// So this iterator is the only safe way to get a command pointer.
pub struct Commands<'a> {
    ctx: *mut sys::nk_context,
    next: *const sys::nk_command,
    _ctx: PhantomData<&'a mut ()>,
}

impl<'f> Context<'f> {
    /// Get the draw commands of the frame. Call this method after all windows of the frame and before `clear`.
    ///
    /// The method ends the frame build. After it, `window` and `window_titled` return `None` until `clear` runs.
    /// A command borrows the context, so this code does not compile:
    ///
    /// ```compile_fail,E0502
    /// use nuklear::*;
    /// struct F;
    /// impl Font for F {
    ///     fn height(&self) -> f32 { 12.0 }
    ///     fn width(&self, _h: f32, _t: &str) -> f32 { 0.0 }
    ///     fn glyph(&self, _h: f32, _c: char, _n: char) -> Glyph { Glyph::default() }
    ///     fn texture(&self) -> Handle { Handle::id(1) }
    /// }
    /// let font = UserFont::new(F);
    /// let mut ctx = Context::new(&font).unwrap();
    /// let first = ctx.commands().next();
    /// ctx.clear();
    /// drop(first);
    /// ```
    pub fn commands(&mut self) -> Commands<'_> {
        self.begin_build();
        let ctx = self.raw();
        // SAFETY: the context is valid. `nk__begin` returns null or a pointer to the first command.
        let next = unsafe { sys::nk__begin(ctx) };
        Commands { ctx, next, _ctx: PhantomData }
    }
}

impl<'a> Iterator for Commands<'a> {
    type Item = Command<'a>;

    fn next(&mut self) -> Option<Command<'a>> {
        let cmd = self.next;
        if cmd.is_null() {
            return None;
        }
        // SAFETY: `cmd` comes from `nk__begin` or `nk__next` of this context. The context lives for `'a`.
        self.next = unsafe { sys::nk__next(self.ctx, cmd) };
        // SAFETY: same pointer as above. The frame is built, so the command memory does not change for `'a`.
        Some(unsafe { read(cmd) })
    }
}

/// Copy the data of the command `cmd` into a `Command`.
///
/// # Safety
/// `cmd` must point to a command that the context built. The command memory must stay valid and unchanged for `'a`.
unsafe fn read<'a>(cmd: *const sys::nk_command) -> Command<'a> {
    // SAFETY: the header is the first field of every command struct. The caller guarantees that `cmd` is valid.
    let kind = unsafe { (*cmd).type_ };
    // Each cast gives a pointer to the full command struct that `kind` names. All reads go through raw pointers.
    macro_rules! c {
        ($t:ident) => {
            cmd.cast::<sys::$t>()
        };
    }
    // SAFETY (all reads below): `kind` names the struct type of the cast, so the struct is complete and aligned.
    // The trailing data of `points` and `string` runs past the declared array, so only `addr_of!` reads it.
    unsafe {
        match kind {
            sys::NK_COMMAND_SCISSOR => {
                let p = c!(nk_command_scissor);
                Command::Scissor { x: (*p).x, y: (*p).y, w: (*p).w, h: (*p).h }
            }
            sys::NK_COMMAND_LINE => {
                let p = c!(nk_command_line);
                Command::Line { thickness: (*p).line_thickness, begin: (*p).begin.into(), end: (*p).end.into(), color: (*p).color.into() }
            }
            sys::NK_COMMAND_CURVE => {
                let p = c!(nk_command_curve);
                let ctrl = (*p).ctrl;
                Command::Curve {
                    thickness: (*p).line_thickness,
                    begin: (*p).begin.into(),
                    end: (*p).end.into(),
                    ctrl: [ctrl[0].into(), ctrl[1].into()],
                    color: (*p).color.into(),
                }
            }
            sys::NK_COMMAND_RECT => {
                let p = c!(nk_command_rect);
                Command::Rect {
                    rounding: (*p).rounding,
                    thickness: (*p).line_thickness,
                    x: (*p).x,
                    y: (*p).y,
                    w: (*p).w,
                    h: (*p).h,
                    color: (*p).color.into(),
                }
            }
            sys::NK_COMMAND_RECT_FILLED => {
                let p = c!(nk_command_rect_filled);
                Command::RectFilled { rounding: (*p).rounding, x: (*p).x, y: (*p).y, w: (*p).w, h: (*p).h, color: (*p).color.into() }
            }
            sys::NK_COMMAND_RECT_MULTI_COLOR => {
                let p = c!(nk_command_rect_multi_color);
                Command::RectMultiColor {
                    x: (*p).x,
                    y: (*p).y,
                    w: (*p).w,
                    h: (*p).h,
                    left: (*p).left.into(),
                    top: (*p).top.into(),
                    bottom: (*p).bottom.into(),
                    right: (*p).right.into(),
                }
            }
            sys::NK_COMMAND_CIRCLE => {
                let p = c!(nk_command_circle);
                Command::Circle { thickness: (*p).line_thickness, x: (*p).x, y: (*p).y, w: (*p).w, h: (*p).h, color: (*p).color.into() }
            }
            sys::NK_COMMAND_CIRCLE_FILLED => {
                let p = c!(nk_command_circle_filled);
                Command::CircleFilled { x: (*p).x, y: (*p).y, w: (*p).w, h: (*p).h, color: (*p).color.into() }
            }
            sys::NK_COMMAND_ARC => {
                let p = c!(nk_command_arc);
                Command::Arc { cx: (*p).cx, cy: (*p).cy, r: (*p).r, thickness: (*p).line_thickness, a: (*p).a, color: (*p).color.into() }
            }
            sys::NK_COMMAND_ARC_FILLED => {
                let p = c!(nk_command_arc_filled);
                Command::ArcFilled { cx: (*p).cx, cy: (*p).cy, r: (*p).r, a: (*p).a, color: (*p).color.into() }
            }
            sys::NK_COMMAND_TRIANGLE => {
                let p = c!(nk_command_triangle);
                Command::Triangle {
                    thickness: (*p).line_thickness,
                    a: (*p).a.into(),
                    b: (*p).b.into(),
                    c: (*p).c.into(),
                    color: (*p).color.into(),
                }
            }
            sys::NK_COMMAND_TRIANGLE_FILLED => {
                let p = c!(nk_command_triangle_filled);
                Command::TriangleFilled { a: (*p).a.into(), b: (*p).b.into(), c: (*p).c.into(), color: (*p).color.into() }
            }
            sys::NK_COMMAND_POLYGON => {
                let p = c!(nk_command_polygon);
                let points = std::slice::from_raw_parts(addr_of!((*p).points).cast::<Vec2i>(), (*p).point_count as usize);
                Command::Polygon { color: (*p).color.into(), thickness: (*p).line_thickness, points }
            }
            sys::NK_COMMAND_POLYGON_FILLED => {
                let p = c!(nk_command_polygon_filled);
                let points = std::slice::from_raw_parts(addr_of!((*p).points).cast::<Vec2i>(), (*p).point_count as usize);
                Command::PolygonFilled { color: (*p).color.into(), points }
            }
            sys::NK_COMMAND_POLYLINE => {
                let p = c!(nk_command_polyline);
                let points = std::slice::from_raw_parts(addr_of!((*p).points).cast::<Vec2i>(), (*p).point_count as usize);
                Command::Polyline { color: (*p).color.into(), thickness: (*p).line_thickness, points }
            }
            sys::NK_COMMAND_TEXT => {
                let p = c!(nk_command_text);
                let font = (*p).font;
                // `userdata` is the pointer of our trampoline, so the renderer needs the `texture` handle.
                let font = if font.is_null() { Handle::id(0) } else { Handle(addr_of!((*font).texture).read()) };
                // A negative length counts as an empty text.
                let len = usize::try_from((*p).length).unwrap_or(0);
                let bytes = std::slice::from_raw_parts(addr_of!((*p).string).cast::<u8>(), len);
                Command::Text {
                    font,
                    font_height: (*p).height,
                    background: (*p).background.into(),
                    foreground: (*p).foreground.into(),
                    x: (*p).x,
                    y: (*p).y,
                    w: (*p).w,
                    h: (*p).h,
                    text: String::from_utf8_lossy(bytes),
                }
            }
            sys::NK_COMMAND_IMAGE => {
                let p = c!(nk_command_image);
                Command::Image { x: (*p).x, y: (*p).y, w: (*p).w, h: (*p).h, img: Image::from_sys((*p).img), color: (*p).col.into() }
            }
            _ => Command::Custom,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;

    struct Fixed;
    impl Font for Fixed {
        fn height(&self) -> f32 { 12.0 }
        fn width(&self, _h: f32, t: &str) -> f32 { 6.0 * t.chars().count() as f32 }
        fn glyph(&self, _h: f32, _c: char, _n: char) -> Glyph { Glyph { width: 6.0, height: 12.0, xadvance: 6.0, ..Default::default() } }
        fn texture(&self) -> Handle { Handle::id(1) }
    }

    const BOUNDS: Rect = Rect { x: 0.0, y: 0.0, w: 200.0, h: 200.0 };

    /// Draw a window "a" with a static popup. The popup code uses `sys`, because the wrapper has no popup method yet.
    fn draw_a_with_popup(ctx: &mut Context<'_>) {
        ctx.window("a", BOUNDS, PanelFlags::BORDER, |w| {
            w.layout_row_dynamic(30.0, 1);
            w.label("x", TextAlign::LEFT);
            let c = w.ctx.raw();
            let r = sys::nk_rect { x: 10.0, y: 10.0, w: 100.0, h: 100.0 };
            // SAFETY: the context is valid and a window is open. Every successful `nk_popup_begin` gets one `nk_popup_end`.
            unsafe {
                if sys::nk_popup_begin(c, sys::NK_POPUP_STATIC as _, c"p".as_ptr(), 0, r) {
                    sys::nk_layout_row_dynamic(c, 30.0, 1);
                    sys::nk_label(c, c"popup".as_ptr(), 0);
                    sys::nk_popup_end(c);
                }
            }
        });
    }

    fn draw_b(ctx: &mut Context<'_>) {
        ctx.window("b", BOUNDS, PanelFlags::BORDER, |w| {
            w.layout_row_dynamic(30.0, 1);
            (0..4).for_each(|_| w.label("x", TextAlign::LEFT));
        });
    }

    /// A popup flag that `commands` did not reset in frame N must not reach the build of frame N+1.
    #[test] fn old_popup_flag_does_not_reach_next_build() {
        let font = UserFont::new(Fixed);
        let mut ctx = Context::new(&font).unwrap();
        draw_a_with_popup(&mut ctx);
        ctx.clear();
        draw_b(&mut ctx);
        let walk: Vec<String> = ctx.commands().map(|c| format!("{c:?}")).collect();
        ctx.clear();

        let mut fresh = Context::new(&font).unwrap();
        draw_b(&mut fresh);
        let expected: Vec<String> = fresh.commands().map(|c| format!("{c:?}")).collect();
        assert_eq!(walk, expected);
    }

    /// A window that is hidden after its popup must not get its popup linked. With no other window, `nk_build` would write through null.
    #[test] fn popup_of_window_hidden_after_popup_is_not_linked() {
        let font = UserFont::new(Fixed);
        let mut ctx = Context::new(&font).unwrap();
        draw_a_with_popup(&mut ctx);
        ctx.window_show("a", ShowState::Hidden);
        assert_eq!(ctx.commands().count(), 0);
        ctx.clear();
    }
}
