//! Basic widgets of the window scope.
//!
//! Every text method passes the pointer and the length of the Rust string to a Nuklear `_text` function.
//! None of these functions reads the string up to a NUL byte.

use crate::{
    cstr, sys, text_arg, ButtonBehavior, ChartType, Color, ColorFormat, Colorf, EditEvents,
    EditFlags, Filter, Heading, Image, StyleButton, Symbol, TextAlign, TextEdit, Window,
};
use std::ffi::c_int;

/// The largest absolute value of a float property, exclusive. `nk_dtoa` casts the value to `int` and
/// writes past its 64 byte buffer when the cast is out of range. NaN fails the `<` test too.
const PROPERTY_LIMIT: f32 = 2_147_483_648.0;

impl<'a, 'f> Window<'a, 'f> {
    /// Show a text edit widget. The widget edits `edit` and drops the characters that `filter` rejects.
    pub fn edit(&mut self, flags: EditFlags, edit: &mut TextEdit, filter: Filter) -> EditEvents {
        // SAFETY: the context is valid, a window is open, and the pointer comes from the unique `TextEdit`.
        let r = unsafe { sys::nk_edit_buffer(self.raw(), flags.bits(), edit.raw(), filter.raw()) };
        EditEvents::from_bits(r)
    }

    /// Show a text edit widget that edits `text` in place. The text holds at most `max` bytes.
    /// Nuklear keeps the cursor and the selection of the active field in the window, so `text` can be a new `String` each frame.
    /// The method cuts a longer `text` at a character boundary, and it limits `max` to 2^30.
    /// The widget drops the characters that `filter` rejects.
    pub fn edit_string(
        &mut self,
        flags: EditFlags,
        text: &mut String,
        max: usize,
        filter: Filter,
    ) -> EditEvents {
        let max = max.min(crate::TEXT_LIMIT);
        let mut n = text.len().min(max);
        while !text.is_char_boundary(n) {
            n -= 1;
        }
        // `nk_edit_string` keeps the text at most `max - 1` bytes long, so the buffer has one more byte.
        let mut buf = vec![0u8; max + 1];
        buf[..n].copy_from_slice(&text.as_bytes()[..n]);
        let mut len = n as c_int;
        // SAFETY: the context is valid, a window is open, and `buf` is writable for `max + 1` bytes, which fits in an `int`.
        // `len` is at most `max`. Nuklear uses the buffer only during the call, as a fixed string that it does not grow.
        let r = unsafe {
            sys::nk_edit_string(
                self.raw(),
                flags.bits(),
                buf.as_mut_ptr().cast(),
                &mut len,
                (max + 1) as c_int,
                filter.raw(),
            )
        };
        let len = usize::try_from(len).unwrap_or(0).min(max);
        *text = String::from_utf8_lossy(&buf[..len]).into_owned();
        EditEvents::from_bits(r)
    }

    /// Show a horizontal line that fills the next widget slot. With `rounding`, the line has round ends.
    pub fn rule_horizontal(&mut self, color: Color, rounding: bool) {
        // SAFETY: the context is valid, and a window is open.
        unsafe { sys::nk_rule_horizontal(self.raw(), color.into(), rounding) }
    }

    /// Show a button with a text and its own style. The method returns `true` when the user clicks the button.
    /// Copy `Style::button` and change the copy to make `style`.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn button_label_styled(&mut self, style: &StyleButton, text: &str) -> bool {
        let (ptr, len) = text_arg(text);
        // SAFETY: the context is valid, a window is open, `style` is a complete style that lives for the call, and `ptr` points to `len` readable bytes.
        unsafe { sys::nk_button_text_styled(self.raw(), style.raw(), ptr, len) }
    }

    /// Show a text label.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn label(&mut self, text: &str, align: TextAlign) {
        let (ptr, len) = text_arg(text);
        // SAFETY: the context is valid, a window is open, and `ptr` points to `len` readable bytes.
        unsafe { sys::nk_text(self.raw(), ptr, len, align.bits()) }
    }

    /// Show a text label with a color.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn label_colored(&mut self, text: &str, align: TextAlign, color: Color) {
        let (ptr, len) = text_arg(text);
        // SAFETY: the context is valid, a window is open, and `ptr` points to `len` readable bytes.
        unsafe { sys::nk_text_colored(self.raw(), ptr, len, align.bits(), color.into()) }
    }

    /// Show a text label that wraps at the end of the line.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn label_wrap(&mut self, text: &str) {
        let (ptr, len) = text_arg(text);
        // SAFETY: the context is valid, a window is open, and `ptr` points to `len` readable bytes.
        unsafe { sys::nk_text_wrap(self.raw(), ptr, len) }
    }

    /// Show a text label with a color that wraps at the end of the line.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn label_wrap_colored(&mut self, text: &str, color: Color) {
        let (ptr, len) = text_arg(text);
        // SAFETY: the context is valid, a window is open, and `ptr` points to `len` readable bytes.
        unsafe { sys::nk_text_wrap_colored(self.raw(), ptr, len, color.into()) }
    }

    /// Show an image.
    pub fn image(&mut self, img: Image) {
        // SAFETY: the context is valid, and a window is open.
        unsafe { sys::nk_image(self.raw(), sys::nk_image::from(img)) }
    }

    /// Show an image with a color.
    pub fn image_color(&mut self, img: Image, color: Color) {
        // SAFETY: the context is valid, and a window is open.
        unsafe { sys::nk_image_color(self.raw(), sys::nk_image::from(img), color.into()) }
    }

    /// Show a button with a text. The method returns `true` when the user clicks the button.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn button_label(&mut self, text: &str) -> bool {
        let (ptr, len) = text_arg(text);
        // SAFETY: the context is valid, a window is open, and `ptr` points to `len` readable bytes.
        unsafe { sys::nk_button_text(self.raw(), ptr, len) }
    }

    /// Show a button with a color. The method returns `true` when the user clicks the button.
    pub fn button_color(&mut self, color: Color) -> bool {
        // SAFETY: the context is valid, and a window is open.
        unsafe { sys::nk_button_color(self.raw(), color.into()) }
    }

    /// Show a button with a symbol. The method returns `true` when the user clicks the button.
    pub fn button_symbol(&mut self, symbol: Symbol) -> bool {
        // SAFETY: the context is valid, and a window is open.
        unsafe { sys::nk_button_symbol(self.raw(), symbol.raw() as _) }
    }

    /// Show a button with an image. The method returns `true` when the user clicks the button.
    pub fn button_image(&mut self, img: Image) -> bool {
        // SAFETY: the context is valid, and a window is open.
        unsafe { sys::nk_button_image(self.raw(), sys::nk_image::from(img)) }
    }

    /// Show a button with a symbol and a text. The method returns `true` when the user clicks the button.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn button_symbol_label(&mut self, symbol: Symbol, text: &str, align: TextAlign) -> bool {
        let (ptr, len) = text_arg(text);
        // SAFETY: the context is valid, a window is open, and `ptr` points to `len` readable bytes.
        unsafe { sys::nk_button_symbol_text(self.raw(), symbol.raw() as _, ptr, len, align.bits()) }
    }

    /// Show a button with an image and a text. The method returns `true` when the user clicks the button.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn button_image_label(&mut self, img: Image, text: &str, align: TextAlign) -> bool {
        let (ptr, len) = text_arg(text);
        // SAFETY: the context is valid, a window is open, and `ptr` points to `len` readable bytes.
        unsafe {
            sys::nk_button_image_text(self.raw(), sys::nk_image::from(img), ptr, len, align.bits())
        }
    }

    /// Set the behavior of the buttons that follow.
    pub fn button_set_behavior(&mut self, behavior: ButtonBehavior) {
        // SAFETY: the context is valid, and `behavior` is a valid value.
        unsafe { sys::nk_button_set_behavior(self.raw(), behavior.raw() as _) }
    }

    /// Show a checkbox. The method returns `true` when the user changed `active`.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn checkbox(&mut self, text: &str, active: &mut bool) -> bool {
        let (ptr, len) = text_arg(text);
        // SAFETY: the context is valid, a window is open, `ptr` points to `len` readable bytes, and `active` is writable.
        unsafe { sys::nk_checkbox_text(self.raw(), ptr, len, active) }
    }

    /// Show a checkbox that sets or clears the bits `value` in `flags`. The method returns `true` when the user changed `flags`.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn checkbox_flags(&mut self, text: &str, flags: &mut u32, value: u32) -> bool {
        let (ptr, len) = text_arg(text);
        // SAFETY: the context is valid, a window is open, `ptr` points to `len` readable bytes, and `flags` is writable.
        unsafe { sys::nk_checkbox_flags_text(self.raw(), ptr, len, flags, value) }
    }

    /// Show a radio button. The method returns `true` when the user changed `active`.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn radio(&mut self, text: &str, active: &mut bool) -> bool {
        let (ptr, len) = text_arg(text);
        // SAFETY: the context is valid, a window is open, `ptr` points to `len` readable bytes, and `active` is writable.
        unsafe { sys::nk_radio_text(self.raw(), ptr, len, active) }
    }

    /// Show an option button. The method returns the state of the button: `true` when it is active.
    /// Nuklear does not report a change. Compare the result with `active` to find one.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn option(&mut self, text: &str, active: bool) -> bool {
        let (ptr, len) = text_arg(text);
        // SAFETY: the context is valid, a window is open, and `ptr` points to `len` readable bytes.
        unsafe { sys::nk_option_text(self.raw(), ptr, len, active) }
    }

    /// Show a selectable text. The method returns `true` when the user changed `value`.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn selectable(&mut self, text: &str, align: TextAlign, value: &mut bool) -> bool {
        let (ptr, len) = text_arg(text);
        // SAFETY: the context is valid, a window is open, `ptr` points to `len` readable bytes, and `value` is writable.
        unsafe { sys::nk_selectable_text(self.raw(), ptr, len, align.bits(), value) }
    }

    /// Show a selectable text with an image. The method returns `true` when the user changed `value`.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn selectable_image(
        &mut self,
        img: Image,
        text: &str,
        align: TextAlign,
        value: &mut bool,
    ) -> bool {
        let (ptr, len) = text_arg(text);
        // SAFETY: the context is valid, a window is open, `ptr` points to `len` readable bytes, and `value` is writable.
        unsafe {
            sys::nk_selectable_image_text(
                self.raw(),
                sys::nk_image::from(img),
                ptr,
                len,
                align.bits(),
                value,
            )
        }
    }

    /// Show a selectable text with a symbol. The method returns `true` when the user changed `value`.
    /// The method uses only the first 2^30 bytes of the text.
    pub fn selectable_symbol(
        &mut self,
        symbol: Symbol,
        text: &str,
        align: TextAlign,
        value: &mut bool,
    ) -> bool {
        let (ptr, len) = text_arg(text);
        // SAFETY: the context is valid, a window is open, `ptr` points to `len` readable bytes, and `value` is writable.
        unsafe {
            sys::nk_selectable_symbol_text(
                self.raw(),
                symbol.raw() as _,
                ptr,
                len,
                align.bits(),
                value,
            )
        }
    }

    /// Show a slider. The method returns `true` when the user changed `value`.
    /// The method does nothing and returns `false` when `step <= 0`, `min >= max`, or a number is not finite.
    pub fn slider_f32(&mut self, min: f32, value: &mut f32, max: f32, step: f32) -> bool {
        if step <= 0.0 || min >= max || ![min, *value, max, step].iter().all(|v| v.is_finite()) {
            return false;
        }
        // SAFETY: the context is valid, a window is open, and `value` is writable.
        unsafe { sys::nk_slider_float(self.raw(), min, value, max, step) }
    }

    /// Show a slider. The method returns `true` when the user changed `value`.
    /// The method does nothing and returns `false` when `step <= 0` or `min >= max`.
    pub fn slider_i32(&mut self, min: i32, value: &mut i32, max: i32, step: i32) -> bool {
        if step <= 0 || min >= max {
            return false;
        }
        // SAFETY: the context is valid, a window is open, and `value` is writable.
        unsafe { sys::nk_slider_int(self.raw(), min, value, max, step) }
    }

    /// Show a knob. The method returns `true` when the user changed `value`.
    /// The method does nothing and returns `false` when `step <= 0`, `min >= max`, or a number is not finite.
    pub fn knob_f32(
        &mut self,
        min: f32,
        value: &mut f32,
        max: f32,
        step: f32,
        zero: Heading,
        dead_zone_degrees: f32,
    ) -> bool {
        if step <= 0.0
            || min >= max
            || ![min, *value, max, step, dead_zone_degrees]
                .iter()
                .all(|v| v.is_finite())
        {
            return false;
        }
        // SAFETY: the context is valid, a window is open, and `value` is writable.
        unsafe {
            sys::nk_knob_float(
                self.raw(),
                min,
                value,
                max,
                step,
                zero.raw() as _,
                dead_zone_degrees,
            )
        }
    }

    /// Show a knob. The method returns `true` when the user changed `value`.
    /// The method does nothing and returns `false` when `step <= 0`, `min >= max`, or `dead_zone_degrees` is not finite.
    pub fn knob_i32(
        &mut self,
        min: i32,
        value: &mut i32,
        max: i32,
        step: i32,
        zero: Heading,
        dead_zone_degrees: f32,
    ) -> bool {
        if step <= 0 || min >= max || !dead_zone_degrees.is_finite() {
            return false;
        }
        // SAFETY: the context is valid, a window is open, and `value` is writable.
        unsafe {
            sys::nk_knob_int(
                self.raw(),
                min,
                value,
                max,
                step,
                zero.raw() as _,
                dead_zone_degrees,
            )
        }
    }

    /// Show a progress bar. The method returns `true` when the user changed `current`.
    /// The method does nothing and returns `false` when `max` is 0.
    pub fn progress(&mut self, current: &mut usize, max: usize, modifiable: bool) -> bool {
        if max == 0 {
            return false;
        }
        // SAFETY: the context is valid, a window is open, and `current` is writable.
        unsafe { sys::nk_progress(self.raw(), current, max, modifiable) }
    }

    /// Show a color picker. The method returns `true` when the user changed `color`.
    pub fn color_pick(&mut self, color: &mut Colorf, format: ColorFormat) -> bool {
        let mut c = sys::nk_colorf::from(*color);
        // SAFETY: the context is valid, a window is open, and `c` is writable.
        let changed = unsafe { sys::nk_color_pick(self.raw(), &mut c, format.raw() as _) };
        *color = c.into();
        changed
    }

    /// Show an integer property. The method returns `true` when the user changed `value`.
    /// The method does nothing and returns `false` when `min > max` or `inc_per_pixel` is not finite.
    pub fn property_i32(
        &mut self,
        name: &str,
        min: i32,
        value: &mut i32,
        max: i32,
        step: i32,
        inc_per_pixel: f32,
    ) -> bool {
        if min > max || !inc_per_pixel.is_finite() {
            return false;
        }
        let name = cstr(name);
        // SAFETY: the context is valid, a window is open, `name` ends with NUL, and `value` is writable.
        unsafe {
            sys::nk_property_int(
                self.raw(),
                name.as_ptr(),
                min,
                value,
                max,
                step,
                inc_per_pixel,
            )
        }
    }

    /// Show a `f32` property. The method returns `true` when the user changed `value`.
    /// The method does nothing and returns `false` when `min > max`, a number is not finite,
    /// or `min`, `max` or `value` has an absolute value of 2^31 or more.
    pub fn property_f32(
        &mut self,
        name: &str,
        min: f32,
        value: &mut f32,
        max: f32,
        step: f32,
        inc_per_pixel: f32,
    ) -> bool {
        let in_range = |v: f32| v.abs() < PROPERTY_LIMIT;
        if min > max
            || ![step, inc_per_pixel].iter().all(|v| v.is_finite())
            || !(in_range(min) && in_range(*value) && in_range(max))
        {
            return false;
        }
        let name = cstr(name);
        // SAFETY: the context is valid, a window is open, `name` ends with NUL, and `value` is writable.
        unsafe {
            sys::nk_property_float(
                self.raw(),
                name.as_ptr(),
                min,
                value,
                max,
                step,
                inc_per_pixel,
            )
        }
    }

    /// Show a `f64` property. The method returns `true` when the user changed `value`.
    /// The method does nothing and returns `false` when `min > max`, a number is not finite,
    /// or `min`, `max` or `value` has an absolute value of 2^31 or more.
    pub fn property_f64(
        &mut self,
        name: &str,
        min: f64,
        value: &mut f64,
        max: f64,
        step: f64,
        inc_per_pixel: f32,
    ) -> bool {
        let in_range = |v: f64| v.abs() < f64::from(PROPERTY_LIMIT);
        if min > max
            || !step.is_finite()
            || !inc_per_pixel.is_finite()
            || !(in_range(min) && in_range(*value) && in_range(max))
        {
            return false;
        }
        let name = cstr(name);
        // SAFETY: the context is valid, a window is open, `name` ends with NUL, and `value` is writable.
        unsafe {
            sys::nk_property_double(
                self.raw(),
                name.as_ptr(),
                min,
                value,
                max,
                step,
                inc_per_pixel,
            )
        }
    }

    /// Show a chart of `values` from the index `offset`. The method does nothing when no value is left
    /// or a value is not finite. When all values are equal to `v`, the chart range is `v - 0.5` to `v + 0.5`.
    /// For a large `v`, this range can stay empty. Then the method draws nothing.
    pub fn plot(&mut self, ty: ChartType, values: &[f32], offset: usize) {
        let values = values.get(offset..).unwrap_or_default();
        let values = &values[..values.len().min(i32::MAX as usize)];
        let Some(&first) = values.first() else { return };
        if !values.iter().all(|v| v.is_finite()) {
            return;
        }
        let (mut min, mut max) = values
            .iter()
            .fold((first, first), |(lo, hi), &v| (lo.min(v), hi.max(v)));
        // `nk_plot` would divide by `max - min`, and the chart rejects an empty range.
        if min == max {
            (min, max) = (min - 0.5, max + 0.5);
        }
        self.chart(ty, values.len() as i32, min, max, |c| {
            for &v in values {
                c.push(v);
            }
        });
    }

    /// Show a tooltip with `text`. The method does nothing inside a popup.
    pub fn tooltip(&mut self, text: &str) {
        if self.ctx.popup_depth > 0 {
            return;
        }
        let text = cstr(text);
        // SAFETY: the context is valid, a window is open, and `text` ends with NUL.
        self.keep_panel_type(|ctx| unsafe { sys::nk_tooltip(ctx, text.as_ptr()) });
    }

    /// Run `call` with the C context. Then undo a change of the parent panel type.
    ///
    /// When the tooltip popup fails to open, `nk_tooltip_begin` writes the tooltip type into the parent panel.
    /// Then a later popup asserts. This method restores the old panel type and the old popup type.
    pub(crate) fn keep_panel_type<R>(&mut self, call: impl FnOnce(*mut sys::nk_context) -> R) -> R {
        let ctx = self.raw();
        // SAFETY: a `Window` exists only inside a window body. There `current` and its `layout` are not null.
        let (win, layout) = unsafe { ((*ctx).current, (*(*ctx).current).layout) };
        // SAFETY: `win` and `layout` are valid, see above.
        let (layout_type, popup_type) = unsafe { ((*layout).type_, (*win).popup.type_) };
        let r = call(ctx);
        // SAFETY: the window and its panel still exist after the call.
        unsafe {
            if (*layout).type_ != layout_type {
                (*layout).type_ = layout_type;
                (*win).popup.type_ = popup_type;
            }
        }
        r
    }
}
