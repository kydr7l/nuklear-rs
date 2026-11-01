//! The window scope. `Context::window` gives a `Window` to its body.

use crate::{sys, Context, Rect, Vec2};

/// An open window. Nuklear ends the window when the body of `Context::window` returns.
pub struct Window<'a, 'f> {
    pub(crate) ctx: &'a mut Context<'f>,
}

impl Window<'_, '_> {
    /// Get the C context for a call that changes it.
    pub(crate) fn raw(&mut self) -> *mut sys::nk_context {
        self.ctx.raw()
    }

    /// Get the bounds of the window.
    pub fn bounds(&self) -> Rect {
        // SAFETY: the context is valid, and a window is current.
        unsafe { sys::nk_window_get_bounds(self.ctx.raw_const()) }.into()
    }

    /// Get the position of the window.
    pub fn position(&self) -> Vec2 {
        // SAFETY: the context is valid, and a window is current.
        unsafe { sys::nk_window_get_position(self.ctx.raw_const()) }.into()
    }

    /// Get the size of the window.
    pub fn size(&self) -> Vec2 {
        // SAFETY: the context is valid, and a window is current.
        unsafe { sys::nk_window_get_size(self.ctx.raw_const()) }.into()
    }

    /// Get the content region of the window.
    pub fn content_region(&self) -> Rect {
        // SAFETY: the context is valid, and an open window with a layout is current.
        unsafe { sys::nk_window_get_content_region(self.ctx.raw_const()) }.into()
    }

    /// Check if the window has the focus.
    pub fn has_focus(&self) -> bool {
        // SAFETY: the context is valid, and an open window with a layout is current.
        unsafe { sys::nk_window_has_focus(self.ctx.raw_const()) }
    }

    /// Check if the mouse is over the window.
    pub fn is_hovered(&self) -> bool {
        // SAFETY: the context is valid, and an open window with a layout is current.
        unsafe { sys::nk_window_is_hovered(self.ctx.raw_const()) }
    }

    /// Get the scroll offset of the window as `(x, y)`.
    pub fn scroll(&self) -> (u32, u32) {
        let (mut x, mut y) = (0, 0);
        // SAFETY: the context is valid, a window is current, and both pointers refer to writable integers.
        unsafe { sys::nk_window_get_scroll(self.ctx.raw_const(), &mut x, &mut y) };
        (x, y)
    }

    /// Set the scroll offset of the window.
    pub fn set_scroll(&mut self, x: u32, y: u32) {
        // SAFETY: the context is valid, and a window is current.
        unsafe { sys::nk_window_set_scroll(self.raw(), x, y) }
    }
}
