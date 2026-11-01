//! The window scope. `Context::window` gives a `Window` to its body.

use crate::context::RowKind;
use crate::{sys, Button, Context, LayoutFormat, Rect, Vec2};

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

/// The most columns that a row template holds.
const MAX_TEMPLATE_COLUMNS: u8 = sys::NK_MAX_LAYOUT_ROW_TEMPLATE_COLUMNS as u8;

/// Layout methods.
impl Window<'_, '_> {
    /// Get the C panel of the current window.
    pub(crate) fn panel(&self) -> *mut sys::nk_panel {
        // SAFETY: a `Window` exists only inside a window body. There `current` and its `layout` are not null.
        unsafe { (*(*self.ctx.raw_const()).current).layout }
    }

    /// Start a row with `cols` columns of equal width that fill the window.
    pub fn layout_row_dynamic(&mut self, height: f32, cols: i32) {
        self.ctx.panel.layout_started = true;
        self.ctx.panel.row_kind = RowKind::None;
        // SAFETY: the context is valid, and a window is open.
        unsafe { sys::nk_layout_row_dynamic(self.raw(), height, cols) }
    }

    /// Start a row with `cols` columns that are `item_width` pixels wide.
    pub fn layout_row_static(&mut self, height: f32, item_width: i32, cols: i32) {
        self.ctx.panel.layout_started = true;
        self.ctx.panel.row_kind = RowKind::None;
        // SAFETY: the context is valid, and a window is open.
        unsafe { sys::nk_layout_row_static(self.raw(), height, item_width, cols) }
    }

    /// Start a row with one column for each entry of `ratios`. The method does nothing when `ratios` is empty.
    /// For a dynamic row, each ratio is a part of the window width. For a static row, each ratio is a width in pixels.
    pub fn layout_row(&mut self, fmt: LayoutFormat, height: f32, ratios: &[f32]) {
        if ratios.is_empty() {
            return;
        }
        let cols = ratios.len().min(i32::MAX as usize);
        self.ctx.panel.layout_started = true;
        self.ctx.panel.row_kind = RowKind::None;
        // Nuklear keeps the pointer until the window ends, so the context owns a copy of the ratios.
        self.ctx.ratios.push(ratios[..cols].into());
        let ptr = self.ctx.ratios.last().map_or(std::ptr::null(), |r| r.as_ptr());
        // SAFETY: the context is valid, and a window is open. `ptr` points to `cols` floats that live until the window ends.
        unsafe { sys::nk_layout_row(self.raw(), fmt.raw() as _, height, cols as i32, ptr) }
    }

    /// Start a row that you fill with `layout_row_push`. The row has `cols` columns.
    pub fn layout_row_begin(&mut self, fmt: LayoutFormat, height: f32, cols: i32) {
        self.ctx.panel.layout_started = true;
        self.ctx.panel.row_kind = RowKind::Row;
        // SAFETY: the context is valid, and a window is open.
        unsafe { sys::nk_layout_row_begin(self.raw(), fmt.raw() as _, height, cols) }
    }

    /// Set the width of the next column of a row from `layout_row_begin`. The method does nothing outside such a row.
    pub fn layout_row_push(&mut self, value: f32) {
        if self.ctx.panel.row_kind != RowKind::Row {
            return;
        }
        // SAFETY: the context is valid, and the current row is a row from `layout_row_begin`.
        unsafe { sys::nk_layout_row_push(self.raw(), value) }
    }

    /// End a row from `layout_row_begin`. The method does nothing outside such a row.
    pub fn layout_row_end(&mut self) {
        if self.ctx.panel.row_kind != RowKind::Row {
            return;
        }
        self.ctx.panel.row_kind = RowKind::None;
        // SAFETY: the context is valid, and the current row is a row from `layout_row_begin`.
        unsafe { sys::nk_layout_row_end(self.raw()) }
    }

    /// Start a row template. Add columns with the `layout_row_template_push_*` methods.
    pub fn layout_row_template_begin(&mut self, height: f32) {
        self.ctx.panel.layout_started = true;
        self.ctx.panel.row_kind = RowKind::Template;
        self.ctx.panel.template_cols = 0;
        // SAFETY: the context is valid, and a window is open.
        unsafe { sys::nk_layout_row_template_begin(self.raw(), height) }
    }

    /// Check that a template column fits. The method counts the column when it fits.
    fn template_slot(&mut self) -> bool {
        let panel = &mut self.ctx.panel;
        let fits = panel.row_kind == RowKind::Template && panel.template_cols < MAX_TEMPLATE_COLUMNS;
        panel.template_cols += u8::from(fits);
        fits
    }

    /// Add a column to the row template. The column takes the free space.
    /// The method does nothing outside a template or after 16 columns.
    pub fn layout_row_template_push_dynamic(&mut self) {
        if self.template_slot() {
            // SAFETY: the context is valid, and the current row is a template with a free column.
            unsafe { sys::nk_layout_row_template_push_dynamic(self.raw()) }
        }
    }

    /// Add a column to the row template. The column takes the free space and is at least `min_width` pixels wide.
    /// The method does nothing outside a template or after 16 columns.
    pub fn layout_row_template_push_variable(&mut self, min_width: f32) {
        if self.template_slot() {
            // SAFETY: the context is valid, and the current row is a template with a free column.
            unsafe { sys::nk_layout_row_template_push_variable(self.raw(), min_width) }
        }
    }

    /// Add a column of `width` pixels to the row template.
    /// The method does nothing outside a template or after 16 columns.
    pub fn layout_row_template_push_static(&mut self, width: f32) {
        if self.template_slot() {
            // SAFETY: the context is valid, and the current row is a template with a free column.
            unsafe { sys::nk_layout_row_template_push_static(self.raw(), width) }
        }
    }

    /// End the row template. The method does nothing outside a template.
    pub fn layout_row_template_end(&mut self) {
        if self.ctx.panel.row_kind != RowKind::Template {
            return;
        }
        self.ctx.panel.row_kind = RowKind::None;
        // SAFETY: the context is valid, and the current row is a template.
        unsafe { sys::nk_layout_row_template_end(self.raw()) }
    }

    /// Start a free layout space of `height` with room for `widget_count` widgets. Place each widget with `layout_space_push`.
    pub fn layout_space_begin(&mut self, fmt: LayoutFormat, height: f32, widget_count: i32) {
        self.ctx.panel.layout_started = true;
        self.ctx.panel.row_kind = RowKind::Space;
        // SAFETY: the context is valid, and a window is open.
        unsafe { sys::nk_layout_space_begin(self.raw(), fmt.raw() as _, height, widget_count) }
    }

    /// Set the bounds of the next widget in a layout space. The method does nothing outside a layout space.
    pub fn layout_space_push(&mut self, bounds: Rect) {
        if self.ctx.panel.row_kind != RowKind::Space {
            return;
        }
        // SAFETY: the context is valid, and the current row is a layout space.
        unsafe { sys::nk_layout_space_push(self.raw(), bounds.into()) }
    }

    /// End a layout space. The method does nothing outside a layout space.
    pub fn layout_space_end(&mut self) {
        if self.ctx.panel.row_kind != RowKind::Space {
            return;
        }
        self.ctx.panel.row_kind = RowKind::None;
        // SAFETY: the context is valid, and the current row is a layout space.
        unsafe { sys::nk_layout_space_end(self.raw()) }
    }

    /// Get the bounds of the layout space.
    pub fn layout_space_bounds(&self) -> Rect {
        // SAFETY: the context is valid, and an open window with a layout is current.
        unsafe { sys::nk_layout_space_bounds(self.ctx.raw_const()) }.into()
    }

    /// Change a position in the layout space to a position on the screen.
    pub fn layout_space_to_screen(&self, pos: Vec2) -> Vec2 {
        // SAFETY: the context is valid, and an open window with a layout is current.
        unsafe { sys::nk_layout_space_to_screen(self.ctx.raw_const(), pos.into()) }.into()
    }

    /// Change a position on the screen to a position in the layout space.
    pub fn layout_space_to_local(&self, pos: Vec2) -> Vec2 {
        // SAFETY: the context is valid, and an open window with a layout is current.
        unsafe { sys::nk_layout_space_to_local(self.ctx.raw_const(), pos.into()) }.into()
    }

    /// Set the least height of a row.
    pub fn layout_set_min_row_height(&mut self, height: f32) {
        // SAFETY: the context is valid, and a window is open.
        unsafe { sys::nk_layout_set_min_row_height(self.raw(), height) }
    }

    /// Set the least height of a row back to the default.
    pub fn layout_reset_min_row_height(&mut self) {
        // SAFETY: the context is valid, and a window is open.
        unsafe { sys::nk_layout_reset_min_row_height(self.raw()) }
    }

    /// Get the bounds that the next layout row has.
    pub fn layout_widget_bounds(&self) -> Rect {
        // SAFETY: the context is valid, and an open window with a layout is current.
        unsafe { sys::nk_layout_widget_bounds(self.ctx.raw_const()) }.into()
    }

    /// Change a width in pixels to a ratio of the window width.
    pub fn layout_ratio_from_pixel(&self, pixel_width: f32) -> f32 {
        // SAFETY: the context is valid, and an open window with a layout is current.
        unsafe { sys::nk_layout_ratio_from_pixel(self.ctx.raw_const(), pixel_width) }
    }

    /// Skip one widget slot of the current row. The method does nothing when the row has no columns.
    pub fn spacer(&mut self) {
        // SAFETY: `panel` is not null inside a window body.
        if unsafe { (*self.panel()).row.columns } <= 0 {
            return; // A row without columns has no slot to skip.
        }
        // SAFETY: the context is valid, and the row has a column.
        unsafe { sys::nk_spacer(self.raw()) }
    }

    /// Skip `cols` widget slots of the current row. The method does nothing when `cols` is not positive or the row has no columns.
    pub fn spacing(&mut self, cols: i32) {
        let panel = self.panel();
        // SAFETY: `panel` is not null inside a window body.
        let (columns, index) = unsafe { ((*panel).row.columns, (*panel).row.index) };
        // `nk_spacing` divides by the column count and adds `cols` to the index.
        if cols <= 0 || columns <= 0 || index.checked_add(cols).is_none() {
            return;
        }
        // SAFETY: the context is valid, the row has a column, and the sum of `index` and `cols` fits in an `int`.
        unsafe { sys::nk_spacing(self.raw(), cols) }
    }

    /// Get the bounds of the next widget.
    pub fn widget_bounds(&self) -> Rect {
        // SAFETY: the context is valid, and an open window with a layout is current.
        unsafe { sys::nk_widget_bounds(self.ctx.raw_const()) }.into()
    }

    /// Check if the mouse is over the next widget.
    pub fn widget_is_hovered(&self) -> bool {
        // SAFETY: the context is valid, and an open window with a layout is current.
        unsafe { sys::nk_widget_is_hovered(self.ctx.raw_const()) }
    }

    /// Check if the mouse clicked the next widget with `button`.
    pub fn widget_is_mouse_clicked(&self, button: Button) -> bool {
        // SAFETY: the context is valid, and an open window with a layout is current.
        unsafe { sys::nk_widget_is_mouse_clicked(self.ctx.raw_const(), button.raw() as _) }
    }

    /// Check if the mouse button `button` is in the state `down` over the next widget.
    pub fn widget_has_mouse_click_down(&self, button: Button, down: bool) -> bool {
        // SAFETY: the context is valid, and an open window with a layout is current.
        unsafe { sys::nk_widget_has_mouse_click_down(self.ctx.raw_const(), button.raw() as _, down) }
    }

    /// Run `f`. The widgets that `f` makes are disabled and do not react to input. A panic in `f` aborts the process.
    pub fn disabled<R>(&mut self, f: impl FnOnce(&mut Self) -> R) -> R {
        // SAFETY: the context is valid, and a window is open.
        unsafe { sys::nk_widget_disable_begin(self.raw()) };
        let r = crate::no_unwind(|| f(self));
        // SAFETY: the context is valid, and a window is open.
        unsafe { sys::nk_widget_disable_end(self.raw()) };
        r
    }
}
