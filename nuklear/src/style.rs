//! The style of a context. Get it from `Context::style` and change it with `Context::style_mut`.
//!
//! Each style type wraps one C style struct. The methods read and write the fields that hold plain values.
//! The wrapper gives no access to the font, the cursors, the `userdata` handles and the `draw_begin` and `draw_end` callbacks.
//! Safe code can therefore not put a pointer or a callback into a style that C calls later.
//!
//! Nuklear casts many style values to `short` or `unsigned short` when it draws a widget.
//! So a setter of an `f32` or a `Vec2` value ignores a value that is not finite, is negative, or is 65536 or more.

use crate::{sys, Color, Context, Image, Symbol, TextAlign, Vec2, Window};
use std::ffi::c_int;

/// Check that a style number stays in the range that the C casts accept.
fn number_ok(v: f32) -> bool {
    (0.0..65536.0).contains(&v)
}

/// A conversion between the value of a style field and the C value. `put` gets the old C value and can keep it.
trait StyleValue<R>: Sized {
    fn get(raw: R) -> Self;
    fn put(self, old: R) -> R;
}

impl StyleValue<f32> for f32 {
    fn get(raw: f32) -> f32 {
        raw
    }
    fn put(self, old: f32) -> f32 {
        if number_ok(self) {
            self
        } else {
            old
        }
    }
}

impl StyleValue<sys::nk_vec2> for Vec2 {
    fn get(raw: sys::nk_vec2) -> Vec2 {
        raw.into()
    }
    fn put(self, old: sys::nk_vec2) -> sys::nk_vec2 {
        if number_ok(self.x) && number_ok(self.y) {
            self.into()
        } else {
            old
        }
    }
}

impl StyleValue<sys::nk_color> for Color {
    fn get(raw: sys::nk_color) -> Color {
        raw.into()
    }
    fn put(self, _old: sys::nk_color) -> sys::nk_color {
        self.into()
    }
}

impl StyleValue<sys::nk_style_item> for StyleItem {
    fn get(raw: sys::nk_style_item) -> StyleItem {
        StyleItem(raw)
    }
    fn put(self, _old: sys::nk_style_item) -> sys::nk_style_item {
        self.0
    }
}

impl StyleValue<sys::nk_symbol_type> for Symbol {
    fn get(raw: sys::nk_symbol_type) -> Symbol {
        Symbol::try_from_raw(raw as u32).unwrap_or(Symbol::None)
    }
    fn put(self, _old: sys::nk_symbol_type) -> sys::nk_symbol_type {
        self.raw() as _
    }
}

impl StyleValue<sys::nk_flags> for TextAlign {
    fn get(raw: sys::nk_flags) -> TextAlign {
        TextAlign::from_bits(raw)
    }
    fn put(self, _old: sys::nk_flags) -> sys::nk_flags {
        self.bits()
    }
}

impl StyleValue<c_int> for bool {
    fn get(raw: c_int) -> bool {
        raw != 0
    }
    fn put(self, _old: c_int) -> c_int {
        c_int::from(self)
    }
}

impl StyleValue<bool> for bool {
    fn get(raw: bool) -> bool {
        raw
    }
    fn put(self, _old: bool) -> bool {
        self
    }
}

/// The background of a widget part: a color or an image.
#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct StyleItem(sys::nk_style_item);

impl StyleItem {
    /// Make an item that fills with `color`.
    pub fn color(color: Color) -> StyleItem {
        // The default zeroes all bytes of the union.
        let mut item = sys::nk_style_item { type_: sys::NK_STYLE_ITEM_COLOR, ..Default::default() };
        item.data.color = color.into();
        StyleItem(item)
    }

    /// Make an item that draws `img`.
    pub fn image(img: Image) -> StyleItem {
        // The default zeroes all bytes of the union.
        let mut item = sys::nk_style_item { type_: sys::NK_STYLE_ITEM_IMAGE, ..Default::default() };
        item.data.image = img.into();
        StyleItem(item)
    }

    /// Make an item that draws nothing. Nuklear uses a transparent color for it.
    pub fn hide() -> StyleItem {
        StyleItem::color(Color::default())
    }

    /// Get the color of a color item. The method returns `None` for an image item.
    pub fn as_color(&self) -> Option<Color> {
        // SAFETY: a color item comes from `StyleItem::color` or from Nuklear, and both write the color of the union.
        (self.0.type_ == sys::NK_STYLE_ITEM_COLOR).then(|| unsafe { self.0.data.color }.into())
    }
}

/// Make a style type over a C style struct. Each field gets a getter and a setter, and each nested struct gets two accessors.
macro_rules! style {
    (
        $(#[$m:meta])* $name:ident($raw:ident) {
            $($field:ident, $set:ident: $ty:ty;)*
        }
        $(nested { $($sub:ident, $sub_mut:ident: $sty:ident;)* })?
    ) => {
        $(#[$m])*
        ///
        /// The type is a copy of a C struct without pointers to Rust memory, so a copy can move to another context.
        #[repr(transparent)]
        #[derive(Clone, Copy)]
        pub struct $name(sys::$raw);

        impl $name {
            $(
                #[doc = concat!("Get the `", stringify!($field), "` value.")]
                pub fn $field(&self) -> $ty {
                    StyleValue::get(self.0.$field)
                }
                #[doc = concat!("Set the `", stringify!($field), "` value.")]
                pub fn $set(&mut self, value: $ty) {
                    self.0.$field = StyleValue::put(value, self.0.$field);
                }
            )*
            $($(
                #[doc = concat!("Get the `", stringify!($sub), "` style.")]
                pub fn $sub(&self) -> &$sty {
                    // SAFETY: the style type is `repr(transparent)` over the C struct of the field.
                    unsafe { &*std::ptr::addr_of!(self.0.$sub).cast::<$sty>() }
                }
                #[doc = concat!("Get the `", stringify!($sub), "` style for a change.")]
                pub fn $sub_mut(&mut self) -> &mut $sty {
                    // SAFETY: the style type is `repr(transparent)` over the C struct of the field.
                    unsafe { &mut *std::ptr::addr_of_mut!(self.0.$sub).cast::<$sty>() }
                }
            )*)?
        }
    };
}

style! {
    /// The style of text labels.
    StyleText(nk_style_text) {
        color, set_color: Color;
        padding, set_padding: Vec2;
        color_factor, set_color_factor: f32;
        disabled_factor, set_disabled_factor: f32;
    }
}

style! {
    /// The style of a button.
    StyleButton(nk_style_button) {
        normal, set_normal: StyleItem;
        hover, set_hover: StyleItem;
        active, set_active: StyleItem;
        border_color, set_border_color: Color;
        color_factor_background, set_color_factor_background: f32;
        text_background, set_text_background: Color;
        text_normal, set_text_normal: Color;
        text_hover, set_text_hover: Color;
        text_active, set_text_active: Color;
        text_alignment, set_text_alignment: TextAlign;
        color_factor_text, set_color_factor_text: f32;
        border, set_border: f32;
        rounding, set_rounding: f32;
        padding, set_padding: Vec2;
        image_padding, set_image_padding: Vec2;
        touch_padding, set_touch_padding: Vec2;
        disabled_factor, set_disabled_factor: f32;
    }
}

style! {
    /// The style of a checkbox or an option button.
    StyleToggle(nk_style_toggle) {
        normal, set_normal: StyleItem;
        hover, set_hover: StyleItem;
        active, set_active: StyleItem;
        border_color, set_border_color: Color;
        cursor_normal, set_cursor_normal: StyleItem;
        cursor_hover, set_cursor_hover: StyleItem;
        text_normal, set_text_normal: Color;
        text_hover, set_text_hover: Color;
        text_active, set_text_active: Color;
        text_background, set_text_background: Color;
        text_alignment, set_text_alignment: TextAlign;
        padding, set_padding: Vec2;
        touch_padding, set_touch_padding: Vec2;
        spacing, set_spacing: f32;
        border, set_border: f32;
        color_factor, set_color_factor: f32;
        disabled_factor, set_disabled_factor: f32;
    }
}

style! {
    /// The style of a selectable text.
    StyleSelectable(nk_style_selectable) {
        normal, set_normal: StyleItem;
        hover, set_hover: StyleItem;
        pressed, set_pressed: StyleItem;
        normal_active, set_normal_active: StyleItem;
        hover_active, set_hover_active: StyleItem;
        pressed_active, set_pressed_active: StyleItem;
        text_normal, set_text_normal: Color;
        text_hover, set_text_hover: Color;
        text_pressed, set_text_pressed: Color;
        text_normal_active, set_text_normal_active: Color;
        text_hover_active, set_text_hover_active: Color;
        text_pressed_active, set_text_pressed_active: Color;
        text_background, set_text_background: Color;
        text_alignment, set_text_alignment: TextAlign;
        rounding, set_rounding: f32;
        padding, set_padding: Vec2;
        touch_padding, set_touch_padding: Vec2;
        image_padding, set_image_padding: Vec2;
        color_factor, set_color_factor: f32;
        disabled_factor, set_disabled_factor: f32;
    }
}

style! {
    /// The style of a slider.
    StyleSlider(nk_style_slider) {
        normal, set_normal: StyleItem;
        hover, set_hover: StyleItem;
        active, set_active: StyleItem;
        border_color, set_border_color: Color;
        bar_normal, set_bar_normal: Color;
        bar_hover, set_bar_hover: Color;
        bar_active, set_bar_active: Color;
        bar_filled, set_bar_filled: Color;
        cursor_normal, set_cursor_normal: StyleItem;
        cursor_hover, set_cursor_hover: StyleItem;
        cursor_active, set_cursor_active: StyleItem;
        border, set_border: f32;
        rounding, set_rounding: f32;
        bar_height, set_bar_height: f32;
        padding, set_padding: Vec2;
        spacing, set_spacing: Vec2;
        cursor_size, set_cursor_size: Vec2;
        color_factor, set_color_factor: f32;
        disabled_factor, set_disabled_factor: f32;
        show_buttons, set_show_buttons: bool;
        inc_symbol, set_inc_symbol: Symbol;
        dec_symbol, set_dec_symbol: Symbol;
    }
    nested {
        inc_button, inc_button_mut: StyleButton;
        dec_button, dec_button_mut: StyleButton;
    }
}

style! {
    /// The style of a knob.
    StyleKnob(nk_style_knob) {
        normal, set_normal: StyleItem;
        hover, set_hover: StyleItem;
        active, set_active: StyleItem;
        border_color, set_border_color: Color;
        knob_normal, set_knob_normal: Color;
        knob_hover, set_knob_hover: Color;
        knob_active, set_knob_active: Color;
        knob_border_color, set_knob_border_color: Color;
        cursor_normal, set_cursor_normal: Color;
        cursor_hover, set_cursor_hover: Color;
        cursor_active, set_cursor_active: Color;
        border, set_border: f32;
        knob_border, set_knob_border: f32;
        padding, set_padding: Vec2;
        spacing, set_spacing: Vec2;
        cursor_width, set_cursor_width: f32;
        color_factor, set_color_factor: f32;
        disabled_factor, set_disabled_factor: f32;
    }
}

style! {
    /// The style of a progress bar.
    StyleProgress(nk_style_progress) {
        normal, set_normal: StyleItem;
        hover, set_hover: StyleItem;
        active, set_active: StyleItem;
        border_color, set_border_color: Color;
        cursor_normal, set_cursor_normal: StyleItem;
        cursor_hover, set_cursor_hover: StyleItem;
        cursor_active, set_cursor_active: StyleItem;
        cursor_border_color, set_cursor_border_color: Color;
        rounding, set_rounding: f32;
        border, set_border: f32;
        cursor_border, set_cursor_border: f32;
        cursor_rounding, set_cursor_rounding: f32;
        padding, set_padding: Vec2;
        color_factor, set_color_factor: f32;
        disabled_factor, set_disabled_factor: f32;
    }
}

style! {
    /// The style of a scrollbar.
    StyleScrollbar(nk_style_scrollbar) {
        normal, set_normal: StyleItem;
        hover, set_hover: StyleItem;
        active, set_active: StyleItem;
        border_color, set_border_color: Color;
        cursor_normal, set_cursor_normal: StyleItem;
        cursor_hover, set_cursor_hover: StyleItem;
        cursor_active, set_cursor_active: StyleItem;
        cursor_border_color, set_cursor_border_color: Color;
        border, set_border: f32;
        rounding, set_rounding: f32;
        border_cursor, set_border_cursor: f32;
        rounding_cursor, set_rounding_cursor: f32;
        padding, set_padding: Vec2;
        color_factor, set_color_factor: f32;
        disabled_factor, set_disabled_factor: f32;
        show_buttons, set_show_buttons: bool;
        inc_symbol, set_inc_symbol: Symbol;
        dec_symbol, set_dec_symbol: Symbol;
    }
    nested {
        inc_button, inc_button_mut: StyleButton;
        dec_button, dec_button_mut: StyleButton;
    }
}

style! {
    /// The style of a text edit widget.
    StyleEdit(nk_style_edit) {
        normal, set_normal: StyleItem;
        hover, set_hover: StyleItem;
        active, set_active: StyleItem;
        border_color, set_border_color: Color;
        cursor_normal, set_cursor_normal: Color;
        cursor_hover, set_cursor_hover: Color;
        cursor_text_normal, set_cursor_text_normal: Color;
        cursor_text_hover, set_cursor_text_hover: Color;
        text_normal, set_text_normal: Color;
        text_hover, set_text_hover: Color;
        text_active, set_text_active: Color;
        selected_normal, set_selected_normal: Color;
        selected_hover, set_selected_hover: Color;
        selected_text_normal, set_selected_text_normal: Color;
        selected_text_hover, set_selected_text_hover: Color;
        border, set_border: f32;
        rounding, set_rounding: f32;
        cursor_size, set_cursor_size: f32;
        scrollbar_size, set_scrollbar_size: Vec2;
        padding, set_padding: Vec2;
        row_padding, set_row_padding: f32;
        color_factor, set_color_factor: f32;
        disabled_factor, set_disabled_factor: f32;
    }
    nested {
        scrollbar, scrollbar_mut: StyleScrollbar;
    }
}

style! {
    /// The style of a property widget.
    StyleProperty(nk_style_property) {
        normal, set_normal: StyleItem;
        hover, set_hover: StyleItem;
        active, set_active: StyleItem;
        border_color, set_border_color: Color;
        label_normal, set_label_normal: Color;
        label_hover, set_label_hover: Color;
        label_active, set_label_active: Color;
        sym_left, set_sym_left: Symbol;
        sym_right, set_sym_right: Symbol;
        border, set_border: f32;
        rounding, set_rounding: f32;
        padding, set_padding: Vec2;
        color_factor, set_color_factor: f32;
        disabled_factor, set_disabled_factor: f32;
    }
    nested {
        edit, edit_mut: StyleEdit;
        inc_button, inc_button_mut: StyleButton;
        dec_button, dec_button_mut: StyleButton;
    }
}

style! {
    /// The style of a chart.
    StyleChart(nk_style_chart) {
        background, set_background: StyleItem;
        border_color, set_border_color: Color;
        selected_color, set_selected_color: Color;
        color, set_color: Color;
        border, set_border: f32;
        rounding, set_rounding: f32;
        padding, set_padding: Vec2;
        color_factor, set_color_factor: f32;
        disabled_factor, set_disabled_factor: f32;
        show_markers, set_show_markers: bool;
    }
}

style! {
    /// The style of a tree node or a tab.
    StyleTab(nk_style_tab) {
        background, set_background: StyleItem;
        border_color, set_border_color: Color;
        text, set_text: Color;
        sym_minimize, set_sym_minimize: Symbol;
        sym_maximize, set_sym_maximize: Symbol;
        border, set_border: f32;
        rounding, set_rounding: f32;
        indent, set_indent: f32;
        padding, set_padding: Vec2;
        spacing, set_spacing: Vec2;
        color_factor, set_color_factor: f32;
        disabled_factor, set_disabled_factor: f32;
    }
    nested {
        tab_maximize_button, tab_maximize_button_mut: StyleButton;
        tab_minimize_button, tab_minimize_button_mut: StyleButton;
        node_maximize_button, node_maximize_button_mut: StyleButton;
        node_minimize_button, node_minimize_button_mut: StyleButton;
    }
}

style! {
    /// The style of a combo box.
    StyleCombo(nk_style_combo) {
        normal, set_normal: StyleItem;
        hover, set_hover: StyleItem;
        active, set_active: StyleItem;
        border_color, set_border_color: Color;
        label_normal, set_label_normal: Color;
        label_hover, set_label_hover: Color;
        label_active, set_label_active: Color;
        symbol_normal, set_symbol_normal: Color;
        symbol_hover, set_symbol_hover: Color;
        symbol_active, set_symbol_active: Color;
        sym_normal, set_sym_normal: Symbol;
        sym_hover, set_sym_hover: Symbol;
        sym_active, set_sym_active: Symbol;
        border, set_border: f32;
        rounding, set_rounding: f32;
        content_padding, set_content_padding: Vec2;
        button_padding, set_button_padding: Vec2;
        spacing, set_spacing: Vec2;
        color_factor, set_color_factor: f32;
        disabled_factor, set_disabled_factor: f32;
    }
    nested {
        button, button_mut: StyleButton;
    }
}

style! {
    /// The style of a window header.
    StyleWindowHeader(nk_style_window_header) {
        normal, set_normal: StyleItem;
        hover, set_hover: StyleItem;
        active, set_active: StyleItem;
        close_symbol, set_close_symbol: Symbol;
        minimize_symbol, set_minimize_symbol: Symbol;
        maximize_symbol, set_maximize_symbol: Symbol;
        label_normal, set_label_normal: Color;
        label_hover, set_label_hover: Color;
        label_active, set_label_active: Color;
        padding, set_padding: Vec2;
        label_padding, set_label_padding: Vec2;
        spacing, set_spacing: Vec2;
    }
    nested {
        close_button, close_button_mut: StyleButton;
        minimize_button, minimize_button_mut: StyleButton;
    }
}

style! {
    /// The style of a window, a group and a popup.
    StyleWindow(nk_style_window) {
        fixed_background, set_fixed_background: StyleItem;
        background, set_background: Color;
        border_color, set_border_color: Color;
        popup_border_color, set_popup_border_color: Color;
        combo_border_color, set_combo_border_color: Color;
        contextual_border_color, set_contextual_border_color: Color;
        menu_border_color, set_menu_border_color: Color;
        group_border_color, set_group_border_color: Color;
        tooltip_border_color, set_tooltip_border_color: Color;
        scaler, set_scaler: StyleItem;
        border, set_border: f32;
        combo_border, set_combo_border: f32;
        contextual_border, set_contextual_border: f32;
        menu_border, set_menu_border: f32;
        group_border, set_group_border: f32;
        tooltip_border, set_tooltip_border: f32;
        popup_border, set_popup_border: f32;
        min_row_height_padding, set_min_row_height_padding: f32;
        rounding, set_rounding: f32;
        spacing, set_spacing: Vec2;
        scrollbar_size, set_scrollbar_size: Vec2;
        min_size, set_min_size: Vec2;
        padding, set_padding: Vec2;
        group_padding, set_group_padding: Vec2;
        popup_padding, set_popup_padding: Vec2;
        combo_padding, set_combo_padding: Vec2;
        contextual_padding, set_contextual_padding: Vec2;
        menu_padding, set_menu_padding: Vec2;
        tooltip_padding, set_tooltip_padding: Vec2;
        tooltip_offset, set_tooltip_offset: Vec2;
        tooltip_delay, set_tooltip_delay: f32;
    }
    nested {
        header, header_mut: StyleWindowHeader;
    }
}

/// The full style of a context.
///
/// The type has no `Clone`, because the C struct holds the font pointer of its own context.
/// A copy of a part, for example `StyleWindow`, has no pointer, so you can save and restore a part.
#[repr(transparent)]
pub struct Style(sys::nk_style);

/// Make the two accessors of each part of `Style`.
macro_rules! style_parts {
    ($($part:ident, $part_mut:ident: $ty:ident;)*) => {
        impl Style {
            $(
                #[doc = concat!("Get the `", stringify!($part), "` style.")]
                pub fn $part(&self) -> &$ty {
                    // SAFETY: the style type is `repr(transparent)` over the C struct of the field.
                    unsafe { &*std::ptr::addr_of!(self.0.$part).cast::<$ty>() }
                }
                #[doc = concat!("Get the `", stringify!($part), "` style for a change.")]
                pub fn $part_mut(&mut self) -> &mut $ty {
                    // SAFETY: the style type is `repr(transparent)` over the C struct of the field.
                    unsafe { &mut *std::ptr::addr_of_mut!(self.0.$part).cast::<$ty>() }
                }
            )*
        }
    };
}

style_parts! {
    text, text_mut: StyleText;
    button, button_mut: StyleButton;
    contextual_button, contextual_button_mut: StyleButton;
    menu_button, menu_button_mut: StyleButton;
    option, option_mut: StyleToggle;
    checkbox, checkbox_mut: StyleToggle;
    selectable, selectable_mut: StyleSelectable;
    slider, slider_mut: StyleSlider;
    knob, knob_mut: StyleKnob;
    progress, progress_mut: StyleProgress;
    property, property_mut: StyleProperty;
    edit, edit_mut: StyleEdit;
    chart, chart_mut: StyleChart;
    scrollh, scrollh_mut: StyleScrollbar;
    scrollv, scrollv_mut: StyleScrollbar;
    tab, tab_mut: StyleTab;
    combo, combo_mut: StyleCombo;
    window, window_mut: StyleWindow;
}

impl StyleButton {
    /// Get the C struct for a styled button call.
    pub(crate) fn raw(&self) -> &sys::nk_style_button {
        &self.0
    }
}

impl Context<'_> {
    /// Get the style.
    pub fn style(&self) -> &Style {
        // SAFETY: the context is valid. `Style` is `repr(transparent)` over `nk_style`. The shared borrow of `self` keeps C calls away.
        unsafe { &*std::ptr::addr_of!((*self.raw_const()).style).cast::<Style>() }
    }

    /// Get the style for a change. A change applies to the widgets that follow, also inside the current window.
    pub fn style_mut(&mut self) -> &mut Style {
        // SAFETY: the context is valid. `Style` is `repr(transparent)` over `nk_style`. The unique borrow of `self` keeps C calls away.
        unsafe { &mut *std::ptr::addr_of_mut!((*self.raw()).style).cast::<Style>() }
    }
}

impl Window<'_, '_> {
    /// Get the style of the context.
    pub fn style(&self) -> &Style {
        self.ctx.style()
    }

    /// Get the style of the context for a change. A change applies to the widgets that follow.
    pub fn style_mut(&mut self) -> &mut Style {
        self.ctx.style_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn number_setters_ignore_bad_values() {
        let mut b = StyleButton(sys::nk_style_button::default());
        b.set_rounding(4.0);
        for bad in [f32::NAN, f32::INFINITY, -1.0, 65536.0] {
            b.set_rounding(bad);
            assert_eq!(b.rounding(), 4.0);
        }
        b.set_padding(Vec2 { x: 1.0, y: 2.0 });
        b.set_padding(Vec2 { x: 3.0, y: -1.0 });
        assert_eq!(b.padding(), Vec2 { x: 1.0, y: 2.0 });
    }

    #[test]
    fn style_item_keeps_its_color() {
        let c = Color { r: 1, g: 2, b: 3, a: 4 };
        assert_eq!(StyleItem::color(c).as_color(), Some(c));
        assert_eq!(StyleItem::hide().as_color(), Some(Color::default()));
        assert_eq!(StyleItem::image(Image::default()).as_color(), None);
    }

    #[test]
    fn symbol_and_align_round_trip() {
        let mut s = StyleCombo(sys::nk_style_combo::default());
        s.set_sym_normal(Symbol::ChevronDown);
        assert_eq!(s.sym_normal(), Symbol::ChevronDown);
        s.0.sym_normal = 9999;
        assert_eq!(s.sym_normal(), Symbol::None);
        let mut t = StyleButton(sys::nk_style_button::default());
        t.set_text_alignment(TextAlign::CENTERED);
        assert_eq!(t.text_alignment(), TextAlign::CENTERED);
    }
}
