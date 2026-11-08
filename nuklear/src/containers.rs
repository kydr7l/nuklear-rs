//! Scoped containers: groups, trees, popups, combo boxes, contextual menus, menus, tooltips and charts.
//!
//! Each container method calls the Nuklear begin function, runs the body when the container is open,
//! and calls the matching end function. Nuklear asserts that popups do not contain popups,
//! so the popup methods return `None` inside a popup. A panic in a body aborts the process, because the end call cannot be skipped.

use crate::context::RowKind;
use crate::{
    cstr, sys, text_arg, ChartEvents, ChartType, CollapseState, Color, Image, PanelFlags,
    PopupType, Rect, Symbol, TextAlign, TreeType, Vec2, Window,
};
use std::ops::{Deref, DerefMut};

/// Make a newtype over `Window` for the body of a popup kind. `close` calls `$close`.
macro_rules! popup_scope {
    ($(#[$m:meta])* $name:ident, $close:ident) => {
        $(#[$m])*
        pub struct $name<'a, 'f>(Window<'a, 'f>);

        impl<'a, 'f> Deref for $name<'a, 'f> {
            type Target = Window<'a, 'f>;
            fn deref(&self) -> &Window<'a, 'f> {
                &self.0
            }
        }

        impl<'a, 'f> DerefMut for $name<'a, 'f> {
            fn deref_mut(&mut self) -> &mut Window<'a, 'f> {
                &mut self.0
            }
        }

        impl $name<'_, '_> {
            /// Close the popup. Nuklear hides it after this frame.
            pub fn close(&mut self) {
                // SAFETY: the context is valid. Inside this body the current window is the popup, and its panel is the popup panel.
                unsafe { sys::$close(self.0.raw()) }
            }
        }
    };
}

popup_scope! {
    /// The body of a popup.
    Popup, nk_popup_close
}
popup_scope! {
    /// The body of an open combo box.
    Combo, nk_combo_close
}
popup_scope! {
    /// The body of an open contextual menu.
    Contextual, nk_contextual_close
}
popup_scope! {
    /// The body of an open menu.
    Menu, nk_menu_close
}

/// Add the item methods to a popup newtype. Each method returns `true` when the user clicks the item.
macro_rules! items {
    ($name:ident, $text:ident, $image:ident, $symbol:ident) => {
        impl $name<'_, '_> {
            /// Show an item with a text.
            /// The method uses only the first 2^30 bytes of the text.
            pub fn item_label(&mut self, text: &str, align: TextAlign) -> bool {
                let (ptr, len) = text_arg(text);
                // SAFETY: the context is valid, the popup is open, and `ptr` points to `len` readable bytes.
                unsafe { sys::$text(self.0.raw(), ptr, len, align.bits()) }
            }

            /// Show an item with an image and a text.
            /// The method uses only the first 2^30 bytes of the text.
            pub fn item_image_label(&mut self, img: Image, text: &str, align: TextAlign) -> bool {
                let (ptr, len) = text_arg(text);
                // SAFETY: the context is valid, the popup is open, and `ptr` points to `len` readable bytes.
                unsafe {
                    sys::$image(
                        self.0.raw(),
                        sys::nk_image::from(img),
                        ptr,
                        len,
                        align.bits(),
                    )
                }
            }

            /// Show an item with a symbol and a text.
            /// The method uses only the first 2^30 bytes of the text.
            pub fn item_symbol_label(
                &mut self,
                symbol: Symbol,
                text: &str,
                align: TextAlign,
            ) -> bool {
                let (ptr, len) = text_arg(text);
                // SAFETY: the context is valid, the popup is open, and `ptr` points to `len` readable bytes.
                unsafe { sys::$symbol(self.0.raw(), symbol.raw() as _, ptr, len, align.bits()) }
            }
        }
    };
}

items!(
    Combo,
    nk_combo_item_text,
    nk_combo_item_image_text,
    nk_combo_item_symbol_text
);
items!(
    Contextual,
    nk_contextual_item_text,
    nk_contextual_item_image_text,
    nk_contextual_item_symbol_text
);
items!(
    Menu,
    nk_menu_item_text,
    nk_menu_item_image_text,
    nk_menu_item_symbol_text
);

/// The body of an open chart.
pub struct Chart<'a, 'f>(Window<'a, 'f>);

impl Chart<'_, '_> {
    /// Add a value to the chart. The method returns the mouse events on the value.
    /// The method does nothing and returns no event when `value` is not finite.
    pub fn push(&mut self, value: f32) -> ChartEvents {
        if !value.is_finite() {
            return ChartEvents::empty();
        }
        // SAFETY: the context is valid, and the chart is open on the current panel.
        ChartEvents::from_bits(unsafe { sys::nk_chart_push(self.0.raw(), value) })
    }
}

impl<'a, 'f> Window<'a, 'f> {
    /// Make a reborrowed `Window` for a body that uses the same panel.
    fn reborrow(&mut self) -> Window<'_, 'f> {
        Window {
            ctx: &mut *self.ctx,
        }
    }

    /// Run `body` in a popup scope. `begin` opens the popup and `end` closes it. The method returns `None` inside a popup.
    fn popup_scope<R>(
        &mut self,
        begin: impl FnOnce(&mut Self) -> bool,
        body: impl FnOnce(Window<'_, 'f>) -> R,
        end: unsafe extern "C" fn(*mut sys::nk_context),
    ) -> Option<R> {
        if self.ctx.popup_depth > 0 {
            return None;
        }
        self.ctx.popup_depth += 1;
        let open = begin(self);
        let r = open.then(|| {
            let r = self.ctx.panel_scope(|ctx| body(Window { ctx }));
            // SAFETY: the context is valid, and the popup that `begin` opened is current.
            unsafe { end(self.raw()) };
            r
        });
        self.ctx.popup_depth -= 1;
        r
    }

    /// Run `f` in a group panel when the group is open. `begin` calls the C begin function.
    fn group_scope<R>(
        &mut self,
        begin: impl FnOnce(*mut sys::nk_context) -> bool,
        f: impl FnOnce(&mut Window<'_, 'f>) -> R,
    ) -> Option<R> {
        let before = self.panel();
        let open = begin(self.raw());
        // Nuklear ends a closed or minimized group itself and still returns a true value. Only a new panel shows an open group.
        if !open || self.panel() == before {
            return None;
        }
        let r = self.ctx.panel_scope(|ctx| f(&mut Window { ctx }));
        // SAFETY: the context is valid, and the group panel that `begin` opened is current.
        unsafe { sys::nk_group_end(self.raw()) };
        Some(r)
    }

    /// Make a group with `title` as its name. The method runs `f` only when the group is open.
    pub fn group<R>(
        &mut self,
        title: &str,
        flags: PanelFlags,
        f: impl FnOnce(&mut Window<'_, 'f>) -> R,
    ) -> Option<R> {
        let title = cstr(title);
        // SAFETY: the context is valid, a window is open, and `title` ends with NUL. Nuklear copies the title into its hash and draw commands.
        self.group_scope(
            |ctx| unsafe {
                sys::nk_group_begin_titled(ctx, title.as_ptr(), title.as_ptr(), flags.c_bits())
            },
            f,
        )
    }

    /// Make a group that keeps its scroll offset `(x, y)` in `scroll`. The method runs `f` only when the group is open.
    pub fn group_scrolled<R>(
        &mut self,
        scroll: &mut (u32, u32),
        title: &str,
        flags: PanelFlags,
        f: impl FnOnce(&mut Window<'_, 'f>) -> R,
    ) -> Option<R> {
        let title = cstr(title);
        let (x, y) = (&mut scroll.0 as *mut u32, &mut scroll.1 as *mut u32);
        // SAFETY: the context is valid, a window is open, and `title` ends with NUL.
        // Nuklear keeps `x` and `y` until `nk_group_end`. `scroll` stays borrowed until this method returns.
        self.group_scope(
            |ctx| unsafe {
                sys::nk_group_scrolled_offset_begin(ctx, x, y, title.as_ptr(), flags.c_bits())
            },
            f,
        )
    }

    /// Forget the layout state before a tree header. Nuklear starts its own row for the header.
    fn before_tree(&mut self) {
        self.ctx.panel.row_kind = RowKind::None;
        self.ctx.panel.layout_started = true;
    }

    /// Run `f` when a tree node is open, and pop the node after `f`.
    fn tree_body<R>(
        &mut self,
        open: bool,
        f: impl FnOnce(&mut Window<'_, 'f>) -> R,
        pop: unsafe extern "C" fn(*mut sys::nk_context),
    ) -> Option<R> {
        open.then(|| {
            let r = crate::no_unwind(|| f(&mut self.reborrow()));
            // SAFETY: the context is valid, and the tree node that was pushed is open on the current panel.
            unsafe { pop(self.raw()) };
            r
        })
    }

    /// Make a tree node. Nuklear keeps its state under the bytes of `title` and `id`. The method runs `f` only when the node is open.
    /// The hash uses only the first 2^30 bytes of `title`.
    pub fn tree<R>(
        &mut self,
        ty: TreeType,
        title: &str,
        initial: CollapseState,
        id: u32,
        f: impl FnOnce(&mut Window<'_, 'f>) -> R,
    ) -> Option<R> {
        let (hash, len) = text_arg(title);
        let title = cstr(title);
        self.before_tree();
        // SAFETY: the context is valid, a window is open, `title` ends with NUL, and `hash` points to `len` readable bytes.
        let open = unsafe {
            sys::nk_tree_push_hashed(
                self.raw(),
                ty.raw() as _,
                title.as_ptr(),
                initial.raw() as _,
                hash,
                len,
                id as i32,
            )
        };
        self.tree_body(open, f, sys::nk_tree_pop)
    }

    /// Make a tree node that keeps its state in `state`. The method runs `f` only when the node is open.
    pub fn tree_state<R>(
        &mut self,
        ty: TreeType,
        title: &str,
        state: &mut CollapseState,
        f: impl FnOnce(&mut Window<'_, 'f>) -> R,
    ) -> Option<R> {
        let title = cstr(title);
        let mut raw: sys::nk_collapse_states = state.raw() as _;
        self.before_tree();
        // SAFETY: the context is valid, a window is open, `title` ends with NUL, and `raw` is writable.
        let open =
            unsafe { sys::nk_tree_state_push(self.raw(), ty.raw() as _, title.as_ptr(), &mut raw) };
        *state = CollapseState::from_raw(raw as u32);
        self.tree_body(open, f, sys::nk_tree_state_pop)
    }

    /// Make a selectable tree node. Nuklear keeps its state under the bytes of `title` and `id`.
    /// The method runs `f` only when the node is open.
    /// The hash uses only the first 2^30 bytes of `title`.
    pub fn tree_element<R>(
        &mut self,
        ty: TreeType,
        title: &str,
        initial: CollapseState,
        selected: &mut bool,
        id: u32,
        f: impl FnOnce(&mut Window<'_, 'f>) -> R,
    ) -> Option<R> {
        let (hash, len) = text_arg(title);
        let title = cstr(title);
        self.before_tree();
        // SAFETY: the context is valid, a window is open, `title` ends with NUL, `selected` is writable, and `hash` points to `len` readable bytes.
        let open = unsafe {
            sys::nk_tree_element_push_hashed(
                self.raw(),
                ty.raw() as _,
                title.as_ptr(),
                initial.raw() as _,
                selected,
                hash,
                len,
                id as i32,
            )
        };
        self.tree_body(open, f, sys::nk_tree_element_pop)
    }

    /// Make a popup with `bounds` relative to the window. The method runs `f` only when the popup is open.
    /// The method returns `None` inside a popup.
    pub fn popup<R>(
        &mut self,
        ty: PopupType,
        title: &str,
        flags: PanelFlags,
        bounds: Rect,
        f: impl FnOnce(&mut Popup<'_, 'f>) -> R,
    ) -> Option<R> {
        let title = cstr(title);
        // SAFETY: the context is valid, a window is open, and `title` ends with NUL.
        let begin = |w: &mut Self| unsafe {
            sys::nk_popup_begin(
                w.raw(),
                ty.raw() as _,
                title.as_ptr(),
                flags.c_bits(),
                bounds.into(),
            )
        };
        self.popup_scope(begin, |w| f(&mut Popup(w)), sys::nk_popup_end)
    }

    /// Show a combo box with the text `selected`. The method runs `f` only when the combo box is open.
    /// The method returns `None` inside a popup.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn combo_label<R>(
        &mut self,
        selected: &str,
        size: Vec2,
        f: impl FnOnce(&mut Combo<'_, 'f>) -> R,
    ) -> Option<R> {
        let (ptr, len) = text_arg(selected);
        // SAFETY: the context is valid, a window is open, and `ptr` points to `len` readable bytes.
        let begin =
            |w: &mut Self| unsafe { sys::nk_combo_begin_text(w.raw(), ptr, len, size.into()) };
        self.popup_scope(begin, |w| f(&mut Combo(w)), sys::nk_combo_end)
    }

    /// Show a combo box with a color. The method runs `f` only when the combo box is open.
    /// The method returns `None` inside a popup.
    pub fn combo_color<R>(
        &mut self,
        color: Color,
        size: Vec2,
        f: impl FnOnce(&mut Combo<'_, 'f>) -> R,
    ) -> Option<R> {
        // SAFETY: the context is valid, and a window is open.
        let begin =
            |w: &mut Self| unsafe { sys::nk_combo_begin_color(w.raw(), color.into(), size.into()) };
        self.popup_scope(begin, |w| f(&mut Combo(w)), sys::nk_combo_end)
    }

    /// Show a combo box with a symbol and the text `selected`. The method runs `f` only when the combo box is open.
    /// The method returns `None` inside a popup.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn combo_symbol_label<R>(
        &mut self,
        selected: &str,
        symbol: Symbol,
        size: Vec2,
        f: impl FnOnce(&mut Combo<'_, 'f>) -> R,
    ) -> Option<R> {
        let (ptr, len) = text_arg(selected);
        // SAFETY: the context is valid, a window is open, and `ptr` points to `len` readable bytes.
        let begin = |w: &mut Self| unsafe {
            sys::nk_combo_begin_symbol_text(w.raw(), ptr, len, symbol.raw() as _, size.into())
        };
        self.popup_scope(begin, |w| f(&mut Combo(w)), sys::nk_combo_end)
    }

    /// Show a combo box with an image and the text `selected`. The method runs `f` only when the combo box is open.
    /// The method returns `None` inside a popup.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn combo_image_label<R>(
        &mut self,
        selected: &str,
        img: Image,
        size: Vec2,
        f: impl FnOnce(&mut Combo<'_, 'f>) -> R,
    ) -> Option<R> {
        let (ptr, len) = text_arg(selected);
        // SAFETY: the context is valid, a window is open, and `ptr` points to `len` readable bytes.
        let begin = |w: &mut Self| unsafe {
            sys::nk_combo_begin_image_text(w.raw(), ptr, len, sys::nk_image::from(img), size.into())
        };
        self.popup_scope(begin, |w| f(&mut Combo(w)), sys::nk_combo_end)
    }

    /// Show a combo box of `items`. Each item row is `item_height` pixels high.
    /// The method returns `true` when the user selected another item. It does nothing and returns `false` when `items` is empty.
    /// A `selected` index past the end changes to the last index.
    pub fn combo(
        &mut self,
        items: &[&str],
        selected: &mut usize,
        item_height: i32,
        size: Vec2,
    ) -> bool {
        let Some(last) = items.len().checked_sub(1) else {
            return false;
        };
        *selected = (*selected).min(last);
        let old = *selected;
        self.combo_label(items[old], size, |c| {
            c.layout_row_dynamic(item_height as f32, 1);
            for (i, item) in items.iter().enumerate() {
                if c.item_label(item, TextAlign::LEFT) {
                    *selected = i;
                }
            }
        });
        *selected != old
    }

    /// Show a contextual menu of `size` when the user right clicks in `trigger`. The method runs `f` only when the menu is open.
    /// The method returns `None` inside a popup, and when another popup makes the window read only.
    pub fn contextual<R>(
        &mut self,
        flags: PanelFlags,
        size: Vec2,
        trigger: Rect,
        f: impl FnOnce(&mut Contextual<'_, 'f>) -> R,
    ) -> Option<R> {
        // SAFETY: a `Window` exists only inside a window body. There `current` is not null.
        let win = unsafe { (*self.ctx.raw_const()).current };
        // SAFETY: `win` and its panel are valid inside a window body.
        let (rom, open) = unsafe {
            (
                (*self.panel()).flags & sys::NK_WINDOW_ROM as u32 != 0,
                !(*win).popup.win.is_null() && (*win).popup.type_ == sys::NK_PANEL_CONTEXTUAL as _,
            )
        };
        // A window has one popup slot. `nk_nonblock_begin` reuses `popup.win` and does not check its type.
        // So without this check, a contextual menu takes over the slot of an active static popup.
        if rom && !open {
            return None;
        }
        // SAFETY: the context is valid, and a window is open.
        let begin = |w: &mut Self| unsafe {
            sys::nk_contextual_begin(w.raw(), flags.c_bits(), size.into(), trigger.into())
        };
        self.popup_scope(begin, |w| f(&mut Contextual(w)), sys::nk_contextual_end)
    }

    /// Make a menubar. Call it before any other widget or layout of the panel.
    /// The method returns `None` when the panel already has a layout or a widget.
    pub fn menubar<R>(&mut self, f: impl FnOnce(&mut Window<'_, 'f>) -> R) -> Option<R> {
        let panel = self.panel();
        // SAFETY: `panel` is not null inside a window body.
        let at_top = unsafe { (*panel).at_y == (*panel).bounds.y };
        // Nuklear asserts that the menubar starts at the top of the panel.
        if self.ctx.panel.layout_started || !at_top {
            return None;
        }
        // SAFETY: the context is valid, a window is open, and the panel has no row yet.
        unsafe { sys::nk_menubar_begin(self.raw()) };
        self.ctx.panel.layout_started = true;
        let r = crate::no_unwind(|| f(&mut self.reborrow()));
        // SAFETY: the context is valid, and `nk_menubar_begin` ran on the current panel.
        unsafe { sys::nk_menubar_end(self.raw()) };
        Some(r)
    }

    /// Show a menu button with the text `title`. The method runs `f` only when the menu is open.
    /// The method returns `None` inside a popup.
    pub fn menu_label<R>(
        &mut self,
        title: &str,
        align: TextAlign,
        size: Vec2,
        f: impl FnOnce(&mut Menu<'_, 'f>) -> R,
    ) -> Option<R> {
        let title = cstr(title);
        // SAFETY: the context is valid, a window is open, and `title` ends with NUL.
        let begin = |w: &mut Self| unsafe {
            sys::nk_menu_begin_label(w.raw(), title.as_ptr(), align.bits(), size.into())
        };
        self.popup_scope(begin, |w| f(&mut Menu(w)), sys::nk_menu_end)
    }

    /// Show a menu button with an image and the text `title`. The method runs `f` only when the menu is open.
    /// The method returns `None` inside a popup.
    pub fn menu_image_label<R>(
        &mut self,
        title: &str,
        align: TextAlign,
        img: Image,
        size: Vec2,
        f: impl FnOnce(&mut Menu<'_, 'f>) -> R,
    ) -> Option<R> {
        let title = cstr(title);
        // SAFETY: the context is valid, a window is open, and `title` ends with NUL.
        let begin = |w: &mut Self| unsafe {
            sys::nk_menu_begin_image_label(
                w.raw(),
                title.as_ptr(),
                align.bits(),
                sys::nk_image::from(img),
                size.into(),
            )
        };
        self.popup_scope(begin, |w| f(&mut Menu(w)), sys::nk_menu_end)
    }

    /// Show a menu button with a symbol and the text `title`. The method runs `f` only when the menu is open.
    /// The method returns `None` inside a popup.
    pub fn menu_symbol_label<R>(
        &mut self,
        title: &str,
        align: TextAlign,
        symbol: Symbol,
        size: Vec2,
        f: impl FnOnce(&mut Menu<'_, 'f>) -> R,
    ) -> Option<R> {
        let title = cstr(title);
        // SAFETY: the context is valid, a window is open, and `title` ends with NUL.
        let begin = |w: &mut Self| unsafe {
            sys::nk_menu_begin_symbol_label(
                w.raw(),
                title.as_ptr(),
                align.bits(),
                symbol.raw() as _,
                size.into(),
            )
        };
        self.popup_scope(begin, |w| f(&mut Menu(w)), sys::nk_menu_end)
    }

    /// Show a tooltip of `width` at the mouse. The method runs `f` only when the tooltip is open.
    /// The method returns `None` inside a popup and when `width` is not finite.
    pub fn tooltip_with<R>(
        &mut self,
        width: f32,
        f: impl FnOnce(&mut Window<'_, 'f>) -> R,
    ) -> Option<R> {
        if !width.is_finite() {
            return None;
        }
        // SAFETY: the context is valid, and a window is open.
        let begin =
            |w: &mut Self| w.keep_panel_type(|ctx| unsafe { sys::nk_tooltip_begin(ctx, width) });
        self.popup_scope(begin, |mut w| f(&mut w), sys::nk_tooltip_end)
    }

    /// Show a chart with the default colors. See `chart_colored`.
    pub fn chart<R>(
        &mut self,
        ty: ChartType,
        count: i32,
        min: f32,
        max: f32,
        f: impl FnOnce(&mut Chart<'_, 'f>) -> R,
    ) -> Option<R> {
        // SAFETY: the context is valid. The style is part of the context.
        let (color, active) = unsafe {
            let style = &(*self.ctx.raw_const()).style.chart;
            (style.color.into(), style.selected_color.into())
        };
        self.chart_colored(ty, color, active, count, min, max, f)
    }

    /// Show a chart for `count` values from `min` to `max`. `active` is the color of a value under the mouse.
    /// The method runs `f` only when the chart is visible. It returns `None` when `count <= 0`, a limit is not finite, or `min == max`.
    #[allow(clippy::too_many_arguments)] // The arguments match `nk_chart_begin_colored`.
    pub fn chart_colored<R>(
        &mut self,
        ty: ChartType,
        color: Color,
        active: Color,
        count: i32,
        min: f32,
        max: f32,
        f: impl FnOnce(&mut Chart<'_, 'f>) -> R,
    ) -> Option<R> {
        // Nuklear divides by `count` and by `max - min`.
        if count <= 0 || !min.is_finite() || !max.is_finite() || min == max {
            return None;
        }
        // SAFETY: the context is valid, and a window is open.
        let open = unsafe {
            sys::nk_chart_begin_colored(
                self.raw(),
                ty.raw() as _,
                color.into(),
                active.into(),
                count,
                min,
                max,
            )
        };
        open.then(|| {
            let r = crate::no_unwind(|| f(&mut Chart(self.reborrow())));
            // SAFETY: the context is valid, and the chart is open on the current panel.
            unsafe { sys::nk_chart_end(self.raw()) };
            r
        })
    }
}
