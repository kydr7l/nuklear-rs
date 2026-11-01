//! The text buffer of an edit widget.

use crate::{sys, text_arg};
use std::ptr::{addr_of, addr_of_mut, NonNull};

/// A text buffer for `Window::edit`. The buffer owns its text, its cursor and its undo history.
///
/// Nuklear stores the cursor of an undo record as a `short`. If the text has more than 32767 runes,
/// an undo key (`Key::TextUndo`) can fail an assertion in Nuklear. The process then aborts.
pub struct TextEdit {
    /// This pointer comes from `Box::into_raw`. The `nk_str` inside stores a pointer to its own buffer,
    /// and Nuklear writes through the edit struct. So the struct must not move.
    raw: NonNull<sys::nk_text_edit>,
}

impl TextEdit {
    /// Make an empty text buffer.
    pub fn new() -> TextEdit {
        let p = Box::into_raw(Box::<sys::nk_text_edit>::default());
        // SAFETY: `p` comes from `Box::into_raw`, so it is valid and unique.
        unsafe { sys::nk_textedit_init_default(p) };
        // SAFETY: `Box::into_raw` never returns a null pointer.
        TextEdit { raw: unsafe { NonNull::new_unchecked(p) } }
    }

    /// Get a copy of the text. The method replaces invalid UTF-8 bytes.
    pub fn text(&self) -> String {
        let p = self.raw.as_ptr();
        // SAFETY: `p` is valid. The string field is initialized.
        let (ptr, len) = unsafe { (sys::nk_str_get_const(addr_of!((*p).string)), sys::nk_str_len_char(addr_of!((*p).string))) };
        if ptr.is_null() || len <= 0 {
            return String::new();
        }
        // SAFETY: Nuklear returns the buffer start and the byte length of the same string. The borrow ends in this function.
        String::from_utf8_lossy(unsafe { std::slice::from_raw_parts(ptr.cast::<u8>(), len as usize) }).into_owned()
    }

    /// Replace the text. The method resets the cursor, the selection and the undo history.
    /// The method cuts a string longer than 2^30 bytes.
    /// The cut can split a UTF-8 character. In this case `text` replaces the broken bytes with U+FFFD.
    pub fn set_text(&mut self, s: &str) {
        let p = self.raw.as_ptr();
        let (ptr, len) = text_arg(s);
        // SAFETY: `p` is valid and unique. `ptr` points to `len` readable bytes. The `_char` variant takes a byte length.
        // The undo values are the ones that `nk_textedit_clear_state` sets. Old undo records would point past the new text.
        unsafe {
            sys::nk_str_clear(addr_of_mut!((*p).string));
            sys::nk_str_append_text_char(addr_of_mut!((*p).string), ptr, len);
            (*p).cursor = 0;
            (*p).select_start = 0;
            (*p).select_end = 0;
            (*p).undo = sys::nk_text_undo_state::default();
            (*p).undo.redo_point = sys::NK_TEXTEDIT_UNDOSTATECOUNT as _;
            (*p).undo.redo_char_point = sys::NK_TEXTEDIT_UNDOCHARCOUNT as _;
        }
    }

    /// Select all of the text.
    pub fn select_all(&mut self) {
        // SAFETY: the pointer is valid and unique.
        unsafe { sys::nk_textedit_select_all(self.raw.as_ptr()) }
    }

    /// Check if the user edits this buffer now.
    pub fn is_active(&self) -> bool {
        // SAFETY: the pointer is valid. The method reads one byte field.
        unsafe { (*addr_of!((*self.raw.as_ptr()).active)) != 0 }
    }

    /// Get the pointer for a call that changes the buffer.
    pub(crate) fn raw(&mut self) -> *mut sys::nk_text_edit {
        self.raw.as_ptr()
    }
}

impl Default for TextEdit {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TextEdit {
    fn drop(&mut self) {
        // SAFETY: the pointer is valid. `nk_textedit_free` releases the memory that Nuklear owns.
        unsafe { sys::nk_textedit_free(self.raw.as_ptr()) };
        // SAFETY: `raw` comes from `Box::into_raw` in `new`, and `drop` runs once.
        drop(unsafe { Box::from_raw(self.raw.as_ptr()) });
    }
}
