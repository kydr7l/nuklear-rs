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
    /// This flag is `true` after `commands`. `nk_build` links the end of the command list to the used memory, so a new window would break the next walk.
    pub(crate) frame_built: bool,
    _font: PhantomData<&'f sys::nk_user_font>,
}

impl<'f> Context<'f> {
    /// Make a context that uses `font`. The function returns `None` when Nuklear cannot start or cannot get memory.
    pub fn new<F: FontHandle>(font: &'f F) -> Option<Context<'f>> {
        let p = Box::into_raw(Box::<sys::nk_context>::default());
        // SAFETY: `p` comes from `Box::into_raw`. The font pointer stays valid for `'f`, and `'f` outlives the context.
        // `nk_init` copies the allocator. Its slack bytes hold the aligned command writes that the grow path of `nk_buffer_alloc` can put past a block.
        // After `nk_init`, the command memory is allocated or null. The page pool allocates nothing yet.
        let (ok, mem) = unsafe { (sys::nk_init(p, &crate::convert::slack_allocator(), font.raw()), (*p).memory.memory.ptr) };
        // With a null block, Nuklear would write through a null pointer, because the size is already set. Nothing else needs a free.
        if !ok || mem.is_null() {
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
            frame_built: false,
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
    /// After `commands` and before `clear`, the function returns `None` and does not call Nuklear.
    /// The function returns the value of `f`, or `None` when it did not run `f`.
    pub fn window_titled<R>(
        &mut self,
        name: &str,
        title: &str,
        bounds: Rect,
        flags: PanelFlags,
        f: impl FnOnce(&mut Window<'_, 'f>) -> R,
    ) -> Option<R> {
        if self.frame_built {
            return None;
        }
        let (name, title) = (cstr(name), cstr(title));
        // SAFETY: the context is valid, and both strings end with NUL. Nuklear copies the strings.
        let open = unsafe { sys::nk_begin_titled(self.raw(), name.as_ptr(), title.as_ptr(), bounds.into(), flags.c_bits()) };
        if !open {
            self.reset_hidden_buffer();
        }
        let r = open.then(|| self.panel_scope(|ctx| f(&mut Window { ctx })));
        // SAFETY: the context is valid. Every `nk_begin_titled` call needs one `nk_end` call, whatever it returned.
        unsafe { sys::nk_end(self.raw()) };
        // After `nk_end` the panel is gone, so Nuklear holds no pointer into the ratio arrays.
        self.ratios.clear();
        r
    }

    /// Empty the command buffer of the current window when it is hidden.
    ///
    /// `nk_begin_titled` returns early for a hidden window and does not call `nk_start`. So the buffer keeps the offsets of the last frame.
    /// A later `window_show` could make `nk_build` and `nk__begin` use these old offsets inside the commands of this frame.
    /// This code does what `nk_start_buffer` does, so both functions treat the window as empty.
    fn reset_hidden_buffer(&mut self) {
        let c = self.raw();
        // SAFETY: the context is valid. `current` is null or points to a window of the window list. The window is not shared, because `self` is borrowed mutably.
        unsafe {
            let win = (*c).current;
            if win.is_null() || (*win).flags & sys::NK_WINDOW_HIDDEN as u32 == 0 {
                return;
            }
            let at = (*c).memory.allocated;
            let buffer = &mut (*win).buffer;
            buffer.begin = at;
            buffer.end = at;
            buffer.last = at;
            buffer.clip = sys::nk_rect { x: -8192.0, y: -8192.0, w: 16384.0, h: 16384.0 }; // This is `nk_null_rect`.
        }
    }

    /// Clear the `popup.buf.active` flag of every window for which `keep` returns `false`.
    ///
    /// `nk_build` links the popup commands of each window with this flag. The loop checks neither `HIDDEN` nor `seq`.
    /// So an old flag would link offsets of an old frame, or write through a null command pointer.
    fn drop_popup_buffers(&mut self, keep: impl Fn(*const sys::nk_window, &sys::nk_context) -> bool) {
        let c = self.raw();
        // SAFETY: the context is valid. The window list holds valid windows or ends with null. `self` is borrowed mutably.
        unsafe {
            let mut win = (*c).begin;
            while !win.is_null() {
                if !keep(win, &*c) {
                    (*win).popup.buf.active = false;
                }
                win = (*win).next;
            }
        }
    }

    /// Prepare the first build of the draw commands in this frame. Later calls in the same frame do nothing.
    ///
    /// After the call, the frame is built and the window methods return `None` until `clear`.
    pub(crate) fn begin_build(&mut self) {
        if self.frame_built {
            return;
        }
        // A popup flag stays set when a window is hidden after its popup, or has no commands. `nk_build` cannot link such a popup.
        self.drop_popup_buffers(|win, c| {
            // SAFETY: `win` is a valid window of the list.
            unsafe { (*win).flags & sys::NK_WINDOW_HIDDEN as u32 == 0 && (*win).seq == c.seq && (*win).buffer.last != (*win).buffer.begin }
        });
        self.frame_built = true;
    }

    /// End the frame. Call this method after all windows of the frame.
    pub fn clear(&mut self) {
        // The flag stays set when `commands` did not run in the frame, because only `nk_build` resets it. `nk_clear` does not reset it.
        self.drop_popup_buffers(|_, _| false);
        // SAFETY: the context is valid.
        unsafe { sys::nk_clear(self.raw()) };
        self.ratios.clear();
        self.frame_built = false;
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
