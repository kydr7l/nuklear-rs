//! The Nuklear context, the input scope and the window scope.

use crate::{cstr, sys, Button, CollapseState, Color, FontHandle, Key, PanelFlags, Rect, ShowState, Vec2, Window};
use std::marker::PhantomData;
use std::ptr::NonNull;

/// The number of colors in a style color table.
pub const COLOR_COUNT: usize = sys::NK_COLOR_COUNT as usize;

/// The kind of the current layout row.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum RowKind {
    #[default]
    None,
    Row,
    Template,
    Space,
}

/// The layout state of the current panel.
#[derive(Clone, Copy, Default)]
pub(crate) struct PanelState {
    /// The kind of row that the last layout call started.
    pub row_kind: RowKind,
    /// The number of columns pushed to the current row template.
    pub template_cols: u8,
    /// This flag is `true` after the first layout call. The menubar checks it.
    #[allow(dead_code)] // The menubar reads this field.
    pub layout_started: bool,
}

/// A Nuklear context. It holds all GUI state.
///
/// The context borrows the font for the lifetime `'f`. The font must live longer than the context.
/// This code does not compile, because the font ends before the last use of the context:
///
/// ```compile_fail,E0597
/// use nuklear::*;
/// struct F;
/// impl Font for F {
///     fn height(&self) -> f32 { 12.0 }
///     fn width(&self, _h: f32, _t: &str) -> f32 { 0.0 }
///     fn glyph(&self, _h: f32, _c: char, _n: char) -> Glyph { Glyph::default() }
///     fn texture(&self) -> Handle { Handle::id(1) }
/// }
/// let mut ctx;
/// {
///     let font = UserFont::new(F);
///     ctx = Context::new(&font).unwrap();
/// }
/// ctx.clear();
/// ```
pub struct Context<'f> {
    /// This pointer comes from `Box::into_raw`. Nuklear stores pointers into the allocation, so it must not move.
    raw: NonNull<sys::nk_context>,
    /// The number of open popups. A tooltip does nothing inside a popup.
    pub(crate) popup_depth: u32,
    pub(crate) panel: PanelState,
    /// Ratio arrays for `layout_row`. Nuklear keeps a pointer into each array until the window ends.
    pub(crate) ratios: Vec<Box<[f32]>>,
    _font: PhantomData<&'f sys::nk_user_font>,
}

impl<'f> Context<'f> {
    /// Make a context that uses `font`. The function returns `None` when Nuklear cannot start.
    pub fn new<F: FontHandle>(font: &'f F) -> Option<Context<'f>> {
        let p = Box::into_raw(Box::<sys::nk_context>::default());
        // SAFETY: `p` comes from `Box::into_raw`. The font pointer stays valid for `'f`, and `'f` outlives the context.
        let ok = unsafe { sys::nk_init_default(p, font.raw()) };
        if !ok {
            // SAFETY: `p` comes from `Box::into_raw`, and no other owner exists.
            drop(unsafe { Box::from_raw(p) });
            return None;
        }
        Some(Context {
            // SAFETY: `Box::into_raw` never returns a null pointer.
            raw: unsafe { NonNull::new_unchecked(p) },
            popup_depth: 0,
            panel: PanelState::default(),
            ratios: Vec::new(),
            _font: PhantomData,
        })
    }

    /// Get the C context for a call that changes it.
    pub(crate) fn raw(&mut self) -> *mut sys::nk_context {
        self.raw.as_ptr()
    }

    /// Get the C context for a call that only reads it.
    pub(crate) fn raw_const(&self) -> *const sys::nk_context {
        self.raw.as_ptr()
    }

    /// Run `f` with a clean panel state. The function restores the old state after `f`.
    pub(crate) fn panel_scope<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        let saved = std::mem::take(&mut self.panel);
        let r = f(self);
        self.panel = saved;
        r
    }

    /// Change the font of the context.
    pub fn set_font<F: FontHandle>(&mut self, font: &'f F) {
        // SAFETY: the context is valid. The font pointer stays valid for `'f`, and `'f` outlives the context.
        unsafe { sys::nk_style_set_font(self.raw(), font.raw()) }
    }

    /// Reset the style to the default style.
    pub fn style_default(&mut self) {
        // SAFETY: the context is valid.
        unsafe { sys::nk_style_default(self.raw()) }
    }

    /// Set the style colors from a table. The table has one color for each style color.
    pub fn style_from_table(&mut self, colors: &[Color; COLOR_COUNT]) {
        let table: [sys::nk_color; COLOR_COUNT] = colors.map(Into::into);
        // SAFETY: the context is valid, and `table` holds `COLOR_COUNT` colors. Nuklear copies the colors.
        unsafe { sys::nk_style_from_table(self.raw(), table.as_ptr()) }
    }

    /// Send input events to the context. Call this method once for each frame, before any window.
    pub fn input<R>(&mut self, f: impl FnOnce(&mut Input<'_, 'f>) -> R) -> R {
        // SAFETY: the context is valid.
        unsafe { sys::nk_input_begin(self.raw()) };
        let r = f(&mut Input(self));
        // SAFETY: the context is valid.
        unsafe { sys::nk_input_end(self.raw()) };
        r
    }

    /// Make a window with `title` as its name and its title. See `window_titled`.
    pub fn window<R>(
        &mut self,
        title: &str,
        bounds: Rect,
        flags: PanelFlags,
        f: impl FnOnce(&mut Window<'_, 'f>) -> R,
    ) -> Option<R> {
        self.window_titled(title, title, bounds, flags, f)
    }

    /// Make a window. The function runs `f` only when the window is open and visible.
    /// The function returns the value of `f`, or `None` when it did not run `f`.
    pub fn window_titled<R>(
        &mut self,
        name: &str,
        title: &str,
        bounds: Rect,
        flags: PanelFlags,
        f: impl FnOnce(&mut Window<'_, 'f>) -> R,
    ) -> Option<R> {
        let (name, title) = (cstr(name), cstr(title));
        // SAFETY: the context is valid, and both strings end with NUL. Nuklear copies the strings.
        let open = unsafe { sys::nk_begin_titled(self.raw(), name.as_ptr(), title.as_ptr(), bounds.into(), flags.bits()) };
        let r = open.then(|| self.panel_scope(|ctx| f(&mut Window { ctx })));
        // SAFETY: the context is valid. Every `nk_begin_titled` call needs one `nk_end` call, whatever it returned.
        unsafe { sys::nk_end(self.raw()) };
        // After `nk_end` the panel is gone, so Nuklear holds no pointer into the ratio arrays.
        self.ratios.clear();
        r
    }

    /// End the frame. Call this method after all windows of the frame.
    pub fn clear(&mut self) {
        // SAFETY: the context is valid.
        unsafe { sys::nk_clear(self.raw()) };
        self.ratios.clear();
    }

    /// Check if the window `name` is closed. The method returns `true` when the window does not exist.
    pub fn window_is_closed(&self, name: &str) -> bool {
        let name = cstr(name);
        // SAFETY: the context is valid, and `name` ends with NUL.
        unsafe { sys::nk_window_is_closed(self.raw_const(), name.as_ptr()) }
    }

    /// Check if the window `name` is hidden. The method returns `true` when the window does not exist.
    pub fn window_is_hidden(&self, name: &str) -> bool {
        let name = cstr(name);
        // SAFETY: the context is valid, and `name` ends with NUL.
        unsafe { sys::nk_window_is_hidden(self.raw_const(), name.as_ptr()) }
    }

    /// Check if the window `name` is collapsed.
    pub fn window_is_collapsed(&self, name: &str) -> bool {
        let name = cstr(name);
        // SAFETY: the context is valid, and `name` ends with NUL.
        unsafe { sys::nk_window_is_collapsed(self.raw_const(), name.as_ptr()) }
    }

    /// Check if the window `name` is the active window.
    pub fn window_is_active(&self, name: &str) -> bool {
        let name = cstr(name);
        // SAFETY: the context is valid, and `name` ends with NUL.
        unsafe { sys::nk_window_is_active(self.raw_const(), name.as_ptr()) }
    }

    /// Set the bounds of the window `name`.
    pub fn window_set_bounds(&mut self, name: &str, bounds: Rect) {
        let name = cstr(name);
        // SAFETY: the context is valid, and `name` ends with NUL.
        unsafe { sys::nk_window_set_bounds(self.raw(), name.as_ptr(), bounds.into()) }
    }

    /// Set the position of the window `name`.
    pub fn window_set_position(&mut self, name: &str, pos: Vec2) {
        let name = cstr(name);
        // SAFETY: the context is valid, and `name` ends with NUL.
        unsafe { sys::nk_window_set_position(self.raw(), name.as_ptr(), pos.into()) }
    }

    /// Set the size of the window `name`.
    pub fn window_set_size(&mut self, name: &str, size: Vec2) {
        let name = cstr(name);
        // SAFETY: the context is valid, and `name` ends with NUL.
        unsafe { sys::nk_window_set_size(self.raw(), name.as_ptr(), size.into()) }
    }

    /// Give the focus to the window `name`. The method does nothing when the window does not exist.
    pub fn window_set_focus(&mut self, name: &str) {
        let name = cstr(name);
        // SAFETY: the context is valid, and `name` ends with NUL.
        let found = unsafe { sys::nk_window_find(self.raw_const(), name.as_ptr()) };
        // For an unknown name, `nk_window_set_focus` sets the active window to NULL. That breaks the window list at the next frame.
        if found.is_null() {
            return;
        }
        // SAFETY: the context is valid, and `name` ends with NUL.
        unsafe { sys::nk_window_set_focus(self.raw(), name.as_ptr()) }
    }

    /// Close the window `name`.
    pub fn window_close(&mut self, name: &str) {
        let name = cstr(name);
        // SAFETY: the context is valid, and `name` ends with NUL. Normally no window is current, because `nk_end` always runs.
        // After a caught panic in a window body, the stale current window can hit `NK_ASSERT(ctx->current != win)`. That aborts and is not UB.
        unsafe { sys::nk_window_close(self.raw(), name.as_ptr()) }
    }

    /// Collapse or expand the window `name`.
    pub fn window_collapse(&mut self, name: &str, state: CollapseState) {
        let name = cstr(name);
        // SAFETY: the context is valid, and `name` ends with NUL.
        unsafe { sys::nk_window_collapse(self.raw(), name.as_ptr(), state.raw() as _) }
    }

    /// Show or hide the window `name`.
    pub fn window_show(&mut self, name: &str, state: ShowState) {
        let name = cstr(name);
        // SAFETY: the context is valid, and `name` ends with NUL.
        unsafe { sys::nk_window_show(self.raw(), name.as_ptr(), state.raw() as _) }
    }

    /// Check if the mouse is over any window.
    pub fn window_is_any_hovered(&self) -> bool {
        // SAFETY: the context is valid.
        unsafe { sys::nk_window_is_any_hovered(self.raw_const()) }
    }

    /// Check if the mouse is over any window or if any widget changed.
    pub fn item_is_any_active(&self) -> bool {
        // SAFETY: the context is valid.
        unsafe { sys::nk_item_is_any_active(self.raw_const()) }
    }
}

impl Drop for Context<'_> {
    fn drop(&mut self) {
        // SAFETY: the context is valid. `nk_free` releases the memory that Nuklear owns.
        unsafe { sys::nk_free(self.raw()) };
        // SAFETY: `raw` comes from `Box::into_raw` in `new`, and `drop` runs once.
        drop(unsafe { Box::from_raw(self.raw.as_ptr()) });
    }
}

/// The input scope of a context. Use it to send input events inside `Context::input`.
pub struct Input<'a, 'f>(&'a mut Context<'f>);

impl Input<'_, '_> {
    /// Move the mouse to the position `x`, `y`.
    pub fn motion(&mut self, x: i32, y: i32) {
        // SAFETY: the context is valid.
        unsafe { sys::nk_input_motion(self.0.raw(), x, y) }
    }

    /// Press or release a key.
    pub fn key(&mut self, key: Key, down: bool) {
        // SAFETY: the context is valid, and `key` is a valid key value.
        unsafe { sys::nk_input_key(self.0.raw(), key.raw() as _, down) }
    }

    /// Press or release a mouse button at the position `x`, `y`.
    pub fn button(&mut self, b: Button, x: i32, y: i32, down: bool) {
        // SAFETY: the context is valid, and `b` is a valid button value.
        unsafe { sys::nk_input_button(self.0.raw(), b.raw() as _, x, y, down) }
    }

    /// Scroll by `delta`.
    pub fn scroll(&mut self, delta: Vec2) {
        // SAFETY: the context is valid.
        unsafe { sys::nk_input_scroll(self.0.raw(), delta.into()) }
    }

    /// Send one typed character.
    pub fn unicode(&mut self, c: char) {
        // SAFETY: the context is valid.
        unsafe { sys::nk_input_unicode(self.0.raw(), c as u32) }
    }

    /// Send typed text. The method sends each character in turn.
    pub fn text(&mut self, s: &str) {
        s.chars().for_each(|c| self.unicode(c));
    }
}
