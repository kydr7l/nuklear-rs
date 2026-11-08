//! Custom drawing into the command buffer of the current window. `Window::canvas` gives a `Canvas`.
//!
//! Nuklear casts each coordinate to `short`, and each size, radius, rounding and thickness to `unsigned short`.
//! A cast of a value outside the range of the target type is undefined behaviour in C.
//! So each method checks its numbers first. It draws nothing when a number is not finite or does not fit.

use crate::{sys, text_arg, Color, Image, Rect, Vec2, Window};
use std::marker::PhantomData;

/// Check that the C cast of `v` to `short` is defined. The cast truncates toward zero, so the range is open.
fn coord(v: f32) -> bool {
    v > -32769.0 && v < 32768.0
}

/// Check that the C cast of `v` to `unsigned short` is defined. The cast truncates toward zero, so the range is open.
fn extent(v: f32) -> bool {
    v > -1.0 && v < 65536.0
}

/// Check that the C casts of the position and the size of `r` are defined.
fn rect_ok(r: Rect) -> bool {
    coord(r.x) && coord(r.y) && extent(r.w) && extent(r.h)
}

/// Check that the C casts of both values of `p` to `short` are defined.
fn point_ok(p: Vec2) -> bool {
    coord(p.x) && coord(p.y)
}

/// Check that a point list fits in a command. Nuklear stores the count as `unsigned short`.
fn points_ok(points: &[Vec2]) -> bool {
    points.len() <= u16::MAX as usize && points.iter().all(|&p| point_ok(p))
}

/// The drawing surface of the current window. Get it from `Window::canvas`.
///
/// The canvas borrows the window, so no widget runs while the canvas lives. Nuklear clips each shape to the last scissor rectangle.
/// The shapes appear in the draw commands in the order of the calls.
pub struct Canvas<'a> {
    /// The command buffer of the current window. It lives in the window, and the window lives for `'a`.
    buf: *mut sys::nk_command_buffer,
    /// The font of the context. `Context` borrows the font for longer than `'a`.
    font: *const sys::nk_user_font,
    _window: PhantomData<&'a mut ()>,
}

impl Window<'_, '_> {
    /// Get the canvas of the current window. In a group, the canvas is the canvas of the parent window. In a popup, it is the canvas of the popup.
    pub fn canvas(&mut self) -> Canvas<'_> {
        let ctx = self.raw();
        // SAFETY: the context is valid, and a window with a panel is current. The function returns the buffer of that window.
        let buf = unsafe { sys::nk_window_get_canvas(ctx) };
        // SAFETY: the context is valid. `Context::new` and `Context::set_font` set the font from a reference, so it is not null.
        let font = unsafe { (*ctx).style.font };
        Canvas {
            buf,
            font,
            _window: PhantomData,
        }
    }
}

impl Canvas<'_> {
    /// Set the clip rectangle for the shapes that follow, and for the widgets that follow in this window.
    pub fn push_scissor(&mut self, rect: Rect) {
        if rect_ok(rect) {
            // SAFETY: the buffer is valid for the life of the canvas, and the casts of `rect` are defined.
            unsafe { sys::nk_push_scissor(self.buf, rect.into()) }
        }
    }

    /// Draw a line from `begin` to `end`. Nuklear truncates `thickness` to whole pixels.
    pub fn stroke_line(&mut self, begin: Vec2, end: Vec2, thickness: f32, color: Color) {
        if point_ok(begin) && point_ok(end) && extent(thickness) {
            // SAFETY: the buffer is valid for the life of the canvas, and every cast is defined.
            unsafe {
                sys::nk_stroke_line(
                    self.buf,
                    begin.x,
                    begin.y,
                    end.x,
                    end.y,
                    thickness,
                    color.into(),
                )
            }
        }
    }

    /// Draw a cubic Bézier curve from `begin` to `end` with the control points `ctrl`.
    pub fn stroke_curve(
        &mut self,
        begin: Vec2,
        ctrl: [Vec2; 2],
        end: Vec2,
        thickness: f32,
        color: Color,
    ) {
        if point_ok(begin)
            && point_ok(ctrl[0])
            && point_ok(ctrl[1])
            && point_ok(end)
            && extent(thickness)
        {
            let [c0, c1] = ctrl;
            // SAFETY: the buffer is valid for the life of the canvas, and every cast is defined.
            unsafe {
                sys::nk_stroke_curve(
                    self.buf,
                    begin.x,
                    begin.y,
                    c0.x,
                    c0.y,
                    c1.x,
                    c1.y,
                    end.x,
                    end.y,
                    thickness,
                    color.into(),
                )
            }
        }
    }

    /// Draw a rectangle outline with corners of radius `rounding`.
    pub fn stroke_rect(&mut self, rect: Rect, rounding: f32, thickness: f32, color: Color) {
        if rect_ok(rect) && extent(rounding) && extent(thickness) {
            // SAFETY: the buffer is valid for the life of the canvas, and every cast is defined.
            unsafe { sys::nk_stroke_rect(self.buf, rect.into(), rounding, thickness, color.into()) }
        }
    }

    /// Draw a filled rectangle with corners of radius `rounding`.
    pub fn fill_rect(&mut self, rect: Rect, rounding: f32, color: Color) {
        if rect_ok(rect) && extent(rounding) {
            // SAFETY: the buffer is valid for the life of the canvas, and every cast is defined.
            unsafe { sys::nk_fill_rect(self.buf, rect.into(), rounding, color.into()) }
        }
    }

    /// Draw a filled rectangle with one color for each corner.
    /// The Nuklear names do not match the corners: `left` is the top left corner, `top` is the top right corner,
    /// `right` is the bottom right corner, and `bottom` is the bottom left corner.
    pub fn fill_rect_multi_color(
        &mut self,
        rect: Rect,
        left: Color,
        top: Color,
        right: Color,
        bottom: Color,
    ) {
        if rect_ok(rect) {
            // SAFETY: the buffer is valid for the life of the canvas, and the casts of `rect` are defined.
            unsafe {
                sys::nk_fill_rect_multi_color(
                    self.buf,
                    rect.into(),
                    left.into(),
                    top.into(),
                    right.into(),
                    bottom.into(),
                )
            }
        }
    }

    /// Draw a circle outline inside `rect`.
    pub fn stroke_circle(&mut self, rect: Rect, thickness: f32, color: Color) {
        if rect_ok(rect) && extent(thickness) {
            // SAFETY: the buffer is valid for the life of the canvas, and every cast is defined.
            unsafe { sys::nk_stroke_circle(self.buf, rect.into(), thickness, color.into()) }
        }
    }

    /// Draw a filled circle inside `rect`.
    pub fn fill_circle(&mut self, rect: Rect, color: Color) {
        if rect_ok(rect) {
            // SAFETY: the buffer is valid for the life of the canvas, and the casts of `rect` are defined.
            unsafe { sys::nk_fill_circle(self.buf, rect.into(), color.into()) }
        }
    }

    /// Draw an arc outline around `center`. The angles are in radians.
    pub fn stroke_arc(
        &mut self,
        center: Vec2,
        radius: f32,
        a_min: f32,
        a_max: f32,
        thickness: f32,
        color: Color,
    ) {
        if point_ok(center) && extent(radius) && extent(thickness) {
            // SAFETY: the buffer is valid for the life of the canvas, and every cast is defined. Nuklear stores the angles as floats.
            unsafe {
                sys::nk_stroke_arc(
                    self.buf,
                    center.x,
                    center.y,
                    radius,
                    a_min,
                    a_max,
                    thickness,
                    color.into(),
                )
            }
        }
    }

    /// Draw a filled arc around `center`. The angles are in radians.
    pub fn fill_arc(&mut self, center: Vec2, radius: f32, a_min: f32, a_max: f32, color: Color) {
        if point_ok(center) && extent(radius) {
            // SAFETY: the buffer is valid for the life of the canvas, and every cast is defined. Nuklear stores the angles as floats.
            unsafe {
                sys::nk_fill_arc(
                    self.buf,
                    center.x,
                    center.y,
                    radius,
                    a_min,
                    a_max,
                    color.into(),
                )
            }
        }
    }

    /// Draw a triangle outline.
    pub fn stroke_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, thickness: f32, color: Color) {
        if point_ok(a) && point_ok(b) && point_ok(c) && extent(thickness) {
            // SAFETY: the buffer is valid for the life of the canvas, and every cast is defined.
            unsafe {
                sys::nk_stroke_triangle(
                    self.buf,
                    a.x,
                    a.y,
                    b.x,
                    b.y,
                    c.x,
                    c.y,
                    thickness,
                    color.into(),
                )
            }
        }
    }

    /// Draw a filled triangle.
    pub fn fill_triangle(&mut self, a: Vec2, b: Vec2, c: Vec2, color: Color) {
        if point_ok(a) && point_ok(b) && point_ok(c) {
            // SAFETY: the buffer is valid for the life of the canvas, and every cast is defined.
            unsafe { sys::nk_fill_triangle(self.buf, a.x, a.y, b.x, b.y, c.x, c.y, color.into()) }
        }
    }

    /// Draw a polygon outline. The method draws nothing for more than 65535 points.
    pub fn stroke_polygon(&mut self, points: &[Vec2], thickness: f32, color: Color) {
        if points_ok(points) && extent(thickness) {
            // SAFETY: the buffer is valid for the life of the canvas. `Vec2` is `repr(C)` with two floats, so the slice is `2 * len` floats.
            unsafe {
                sys::nk_stroke_polygon(
                    self.buf,
                    points.as_ptr().cast(),
                    points.len() as i32,
                    thickness,
                    color.into(),
                )
            }
        }
    }

    /// Draw a filled polygon. The method draws nothing for more than 65535 points.
    pub fn fill_polygon(&mut self, points: &[Vec2], color: Color) {
        if points_ok(points) {
            // SAFETY: the buffer is valid for the life of the canvas. `Vec2` is `repr(C)` with two floats, so the slice is `2 * len` floats.
            unsafe {
                sys::nk_fill_polygon(
                    self.buf,
                    points.as_ptr().cast(),
                    points.len() as i32,
                    color.into(),
                )
            }
        }
    }

    /// Draw a line through the points. The method draws nothing for more than 65535 points.
    pub fn stroke_polyline(&mut self, points: &[Vec2], thickness: f32, color: Color) {
        if points_ok(points) && extent(thickness) {
            // SAFETY: the buffer is valid for the life of the canvas. `Vec2` is `repr(C)` with two floats, so the slice is `2 * len` floats.
            unsafe {
                sys::nk_stroke_polyline(
                    self.buf,
                    points.as_ptr().cast(),
                    points.len() as i32,
                    thickness,
                    color.into(),
                )
            }
        }
    }

    /// Draw an image into `rect`, tinted with `color`.
    pub fn draw_image(&mut self, rect: Rect, img: Image, color: Color) {
        if rect_ok(rect) {
            let img = sys::nk_image::from(img);
            // SAFETY: the buffer is valid for the life of the canvas, the casts of `rect` are defined, and `img` lives for the call.
            unsafe { sys::nk_draw_image(self.buf, rect.into(), &img, color.into()) }
        }
    }

    /// Draw `text` with the font of the context. Nuklear cuts the text at the right edge of `rect`.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn draw_text(&mut self, rect: Rect, text: &str, background: Color, foreground: Color) {
        if rect_ok(rect) {
            let (ptr, len) = text_arg(text);
            // SAFETY: the buffer is valid for the life of the canvas, the casts of `rect` are defined, and `ptr` points to `len` readable bytes.
            // The font is valid, because the context borrows it. Every font of this crate has a width callback.
            unsafe {
                sys::nk_draw_text(
                    self.buf,
                    rect.into(),
                    ptr,
                    len,
                    self.font,
                    background.into(),
                    foreground.into(),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn casts_reject_out_of_range() {
        assert!(coord(-32768.9) && coord(32767.9) && coord(0.0));
        assert!(!coord(-32769.0) && !coord(32768.0) && !coord(f32::NAN) && !coord(f32::INFINITY));
        assert!(extent(-0.5) && extent(65535.9));
        assert!(!extent(-1.0) && !extent(65536.0) && !extent(f32::NAN));
        assert!(!points_ok(&vec![Vec2::default(); 65536]));
        assert!(points_ok(&[Vec2 { x: 1.0, y: 2.0 }]));
    }
}
