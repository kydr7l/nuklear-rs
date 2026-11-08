//! The input state of the frame, for custom widgets. Get it from `Context::input_state` or `Window::input_state`.

use crate::{sys, Button, Context, Key, Rect, Vec2, Window};

/// The mouse and keyboard state that `Context::input` gave to the context in this frame. The state is read only.
///
/// The built-in widgets ignore the input in a window that another popup blocks.
/// This state does not do that. Use the input only when `Window::widget` gives `WidgetState::Valid` to get the same behavior.
#[derive(Clone, Copy)]
pub struct InputState<'a>(&'a sys::nk_input);

impl Context<'_> {
    /// Get the input state of the frame.
    pub fn input_state(&self) -> InputState<'_> {
        // SAFETY: the context is valid. The shared borrow of `self` keeps every C call that changes the input away while the state lives.
        InputState(unsafe { &(*self.raw_const()).input })
    }
}

impl Window<'_, '_> {
    /// Get the input state of the frame.
    pub fn input_state(&self) -> InputState<'_> {
        self.ctx.input_state()
    }
}

impl InputState<'_> {
    /// The position of the mouse.
    pub fn mouse_pos(&self) -> Vec2 {
        self.0.mouse.pos.into()
    }

    /// The position of the mouse in the last frame.
    pub fn mouse_prev(&self) -> Vec2 {
        self.0.mouse.prev.into()
    }

    /// The movement of the mouse since the last frame.
    pub fn mouse_delta(&self) -> Vec2 {
        self.0.mouse.delta.into()
    }

    /// The scroll movement of this frame.
    pub fn scroll_delta(&self) -> Vec2 {
        self.0.mouse.scroll_delta.into()
    }

    /// Check if the mouse moved in this frame.
    pub fn is_mouse_moved(&self) -> bool {
        // SAFETY: the input is valid for the life of `self`.
        unsafe { sys::nk_input_is_mouse_moved(self.0) }
    }

    /// Check if `button` is down.
    pub fn is_mouse_down(&self, button: Button) -> bool {
        // SAFETY: the input is valid for the life of `self`, and `button` is a valid button value.
        unsafe { sys::nk_input_is_mouse_down(self.0, button.raw() as _) }
    }

    /// Check if `button` went down in this frame.
    pub fn is_mouse_pressed(&self, button: Button) -> bool {
        // SAFETY: the input is valid for the life of `self`, and `button` is a valid button value.
        unsafe { sys::nk_input_is_mouse_pressed(self.0, button.raw() as _) }
    }

    /// Check if `button` went up in this frame.
    pub fn is_mouse_released(&self, button: Button) -> bool {
        // SAFETY: the input is valid for the life of `self`, and `button` is a valid button value.
        unsafe { sys::nk_input_is_mouse_released(self.0, button.raw() as _) }
    }

    /// Check if `button` went down and up in this frame, and the mouse is over `rect`.
    pub fn mouse_clicked(&self, button: Button, rect: Rect) -> bool {
        // SAFETY: the input is valid for the life of `self`, and `button` is a valid button value.
        unsafe { sys::nk_input_mouse_clicked(self.0, button.raw() as _, rect.into()) }
    }

    /// Check if `button` changed in this frame at a position inside `rect`.
    pub fn has_mouse_click_in_rect(&self, button: Button, rect: Rect) -> bool {
        // SAFETY: the input is valid for the life of `self`, and `button` is a valid button value.
        unsafe { sys::nk_input_has_mouse_click_in_rect(self.0, button.raw() as _, rect.into()) }
    }

    /// Check if `button` went to the state `down` at a position inside `rect`, and stays in that state.
    pub fn has_mouse_click_down_in_rect(&self, button: Button, rect: Rect, down: bool) -> bool {
        // SAFETY: the input is valid for the life of `self`, and `button` is a valid button value.
        unsafe {
            sys::nk_input_has_mouse_click_down_in_rect(self.0, button.raw() as _, rect.into(), down)
        }
    }

    /// Check if the mouse is over `rect`.
    pub fn is_mouse_hovering_rect(&self, rect: Rect) -> bool {
        // SAFETY: the input is valid for the life of `self`.
        unsafe { sys::nk_input_is_mouse_hovering_rect(self.0, rect.into()) }
    }

    /// Check if the mouse was over `rect` in the last frame.
    pub fn is_mouse_prev_hovering_rect(&self, rect: Rect) -> bool {
        // SAFETY: the input is valid for the life of `self`.
        unsafe { sys::nk_input_is_mouse_prev_hovering_rect(self.0, rect.into()) }
    }

    /// Check if `key` is down.
    pub fn is_key_down(&self, key: Key) -> bool {
        // SAFETY: the input is valid for the life of `self`, and `key` is a valid key value.
        unsafe { sys::nk_input_is_key_down(self.0, key.raw() as _) }
    }

    /// Check if `key` went down in this frame.
    pub fn is_key_pressed(&self, key: Key) -> bool {
        // SAFETY: the input is valid for the life of `self`, and `key` is a valid key value.
        unsafe { sys::nk_input_is_key_pressed(self.0, key.raw() as _) }
    }

    /// Check if `key` went up in this frame.
    pub fn is_key_released(&self, key: Key) -> bool {
        // SAFETY: the input is valid for the life of `self`, and `key` is a valid key value.
        unsafe { sys::nk_input_is_key_released(self.0, key.raw() as _) }
    }
}
