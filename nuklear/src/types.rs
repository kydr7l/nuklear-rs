//! Plain value types, flag sets and enums for the Nuklear wrapper.
use crate::sys;
use std::ffi::c_void;

/// Make two `From` impls between a wrapper struct and a `sys` struct. Copy field by field.
macro_rules! convert {
    ($name:ident, $raw:ident { $($f:ident),+ }) => {
        impl From<sys::$raw> for $name {
            fn from(v: sys::$raw) -> Self { Self { $($f: v.$f),+ } }
        }
        impl From<$name> for sys::$raw {
            fn from(v: $name) -> Self { Self { $($f: v.$f),+ } }
        }
    };
}

/// A 2D vector of `f32` values.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vec2 {
    /// The x value.
    pub x: f32,
    /// The y value.
    pub y: f32,
}
convert!(Vec2, nk_vec2 { x, y });

/// A 2D vector of `i16` values.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vec2i {
    /// The x value.
    pub x: i16,
    /// The y value.
    pub y: i16,
}
convert!(Vec2i, nk_vec2i { x, y });
// The command iterator casts `nk_vec2i` arrays to `Vec2i` slices, so the layouts must match.
const _: () = {
    assert!(std::mem::size_of::<Vec2i>() == std::mem::size_of::<sys::nk_vec2i>());
    assert!(std::mem::align_of::<Vec2i>() == std::mem::align_of::<sys::nk_vec2i>());
};

/// A rectangle with a position and a size.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Rect {
    /// The x position.
    pub x: f32,
    /// The y position.
    pub y: f32,
    /// The width.
    pub w: f32,
    /// The height.
    pub h: f32,
}
convert!(Rect, nk_rect { x, y, w, h });

/// A color with four 8-bit channels.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Color {
    /// The red channel.
    pub r: u8,
    /// The green channel.
    pub g: u8,
    /// The blue channel.
    pub b: u8,
    /// The alpha channel.
    pub a: u8,
}
convert!(Color, nk_color { r, g, b, a });

/// A color with four `f32` channels.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Colorf {
    /// The red channel.
    pub r: f32,
    /// The green channel.
    pub g: f32,
    /// The blue channel.
    pub b: f32,
    /// The alpha channel.
    pub a: f32,
}
convert!(Colorf, nk_colorf { r, g, b, a });

/// An opaque user handle. It holds an integer id or a raw pointer.
#[derive(Clone, Copy, Default)]
pub struct Handle(pub(crate) sys::nk_handle);

impl Handle {
    /// Make a handle from an integer id.
    pub fn id(id: i32) -> Handle {
        let mut h = sys::nk_handle::default(); // This call zeroes all bytes of the union.
        h.id = id;
        Handle(h)
    }
    /// Make a handle from a raw pointer.
    pub fn ptr(p: *mut c_void) -> Handle {
        let mut h = sys::nk_handle::default(); // This call zeroes all bytes of the union.
        h.ptr = p;
        Handle(h)
    }
    /// Read the handle as an integer id.
    pub fn as_id(&self) -> i32 {
        // SAFETY: every public constructor zeroes the union first, and crate-private conversions accept only zeroed handles. All bytes are initialized.
        unsafe { self.0.id }
    }
    /// Read the handle as a raw pointer.
    pub fn as_ptr(&self) -> *mut c_void {
        // SAFETY: every public constructor zeroes the union first, and crate-private conversions accept only zeroed handles. All bytes are initialized.
        unsafe { self.0.ptr }
    }
}

impl std::fmt::Debug for Handle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Handle").field(&self.as_id()).finish()
    }
}

impl PartialEq for Handle {
    fn eq(&self, other: &Self) -> bool {
        self.as_ptr() == other.as_ptr()
    }
}

/// An image with a handle, a size and a texture region.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Image {
    /// The texture handle.
    pub handle: Handle,
    /// The width in pixels.
    pub w: u16,
    /// The height in pixels.
    pub h: u16,
    /// The region as `[x, y, w, h]`.
    pub region: [u16; 4],
}

impl Image {
    /// Make an image that uses the full texture.
    pub fn new(handle: Handle, w: u16, h: u16) -> Image {
        Image { handle, w, h, region: [0, 0, w, h] }
    }
}

impl Image {
    /// Make an image from a C image. The handle must come from C or from this crate, so the union is zeroed.
    /// This function is not public: safe code could build a `sys::nk_handle` with uninitialized bytes.
    pub(crate) fn from_sys(v: sys::nk_image) -> Image {
        Image { handle: Handle(v.handle), w: v.w, h: v.h, region: v.region }
    }
}

impl From<Image> for sys::nk_image {
    fn from(v: Image) -> Self {
        sys::nk_image { handle: v.handle.0, w: v.w, h: v.h, region: v.region }
    }
}

/// Make a flag set type. `from_bits` keeps only the bits of the listed constants.
macro_rules! flags {
    ($(#[$m:meta])* $name:ident { $($(#[$cm:meta])* $c:ident = $v:expr;)+ }) => {
        $(#[$m])*
        #[repr(transparent)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
        pub struct $name(u32);

        impl $name {
            $(
                #[doc = concat!("The `", stringify!($c), "` flag.")]
                $(#[$cm])*
                pub const $c: Self = Self($v);
            )+
            /// The union of all defined flags.
            const ALL: u32 = 0 $(| $v)+;
            /// Make an empty flag set.
            pub const fn empty() -> Self { Self(0) }
            /// Read the raw bits.
            pub const fn bits(self) -> u32 { self.0 }
            /// Check if all flags of `other` are set.
            pub const fn contains(self, other: Self) -> bool { self.0 & other.0 == other.0 }
            /// Make a flag set from raw bits. The function drops bits that no constant defines.
            pub const fn from_bits(bits: u32) -> Self { Self(bits & Self::ALL) }
        }
        impl std::ops::BitOr for $name {
            type Output = Self;
            fn bitor(self, o: Self) -> Self { Self(self.0 | o.0) }
        }
        impl std::ops::BitOrAssign for $name {
            fn bitor_assign(&mut self, o: Self) { self.0 |= o.0; }
        }
        impl std::ops::BitAnd for $name {
            type Output = Self;
            fn bitand(self, o: Self) -> Self { Self(self.0 & o.0) }
        }
    };
}

flags! {
    /// Window and panel flags.
    PanelFlags {
        BORDER = sys::NK_WINDOW_BORDER as u32;
        MOVABLE = sys::NK_WINDOW_MOVABLE as u32;
        SCALABLE = sys::NK_WINDOW_SCALABLE as u32;
        CLOSABLE = sys::NK_WINDOW_CLOSABLE as u32;
        MINIMIZABLE = sys::NK_WINDOW_MINIMIZABLE as u32;
        NO_SCROLLBAR = sys::NK_WINDOW_NO_SCROLLBAR as u32;
        TITLE = sys::NK_WINDOW_TITLE as u32;
        SCROLL_AUTO_HIDE = sys::NK_WINDOW_SCROLL_AUTO_HIDE as u32;
        BACKGROUND = sys::NK_WINDOW_BACKGROUND as u32;
        SCALE_LEFT = sys::NK_WINDOW_SCALE_LEFT as u32;
        ///
        /// This flag turns off `MOVABLE`. Nuklear reads the mouse to move a panel, and a panel with this flag has no input.
        NO_INPUT = sys::NK_WINDOW_NO_INPUT as u32;
    }
}

impl PanelFlags {
    /// Get the bits to give to Nuklear. The method removes `MOVABLE` when `NO_INPUT` is set.
    /// `nk_panel_begin` reads the mouse of a movable panel through a null input pointer when `NO_INPUT` is set.
    pub(crate) const fn c_bits(self) -> u32 {
        if self.contains(Self::NO_INPUT) {
            self.0 & !Self::MOVABLE.0
        } else {
            self.0
        }
    }
}

flags! {
    /// Text alignment flags.
    TextAlign {
        ALIGN_LEFT = sys::NK_TEXT_ALIGN_LEFT as u32;
        ALIGN_CENTERED = sys::NK_TEXT_ALIGN_CENTERED as u32;
        ALIGN_RIGHT = sys::NK_TEXT_ALIGN_RIGHT as u32;
        ALIGN_TOP = sys::NK_TEXT_ALIGN_TOP as u32;
        ALIGN_MIDDLE = sys::NK_TEXT_ALIGN_MIDDLE as u32;
        ALIGN_BOTTOM = sys::NK_TEXT_ALIGN_BOTTOM as u32;
        LEFT = sys::NK_TEXT_LEFT as u32;
        CENTERED = sys::NK_TEXT_CENTERED as u32;
        RIGHT = sys::NK_TEXT_RIGHT as u32;
    }
}

flags! {
    /// Text edit flags.
    EditFlags {
        DEFAULT = sys::NK_EDIT_DEFAULT as u32;
        READ_ONLY = sys::NK_EDIT_READ_ONLY as u32;
        AUTO_SELECT = sys::NK_EDIT_AUTO_SELECT as u32;
        SIG_ENTER = sys::NK_EDIT_SIG_ENTER as u32;
        ALLOW_TAB = sys::NK_EDIT_ALLOW_TAB as u32;
        NO_CURSOR = sys::NK_EDIT_NO_CURSOR as u32;
        SELECTABLE = sys::NK_EDIT_SELECTABLE as u32;
        CLIPBOARD = sys::NK_EDIT_CLIPBOARD as u32;
        CTRL_ENTER_NEWLINE = sys::NK_EDIT_CTRL_ENTER_NEWLINE as u32;
        NO_HORIZONTAL_SCROLL = sys::NK_EDIT_NO_HORIZONTAL_SCROLL as u32;
        ALWAYS_INSERT_MODE = sys::NK_EDIT_ALWAYS_INSERT_MODE as u32;
        MULTILINE = sys::NK_EDIT_MULTILINE as u32;
        GOTO_END_ON_ACTIVATE = sys::NK_EDIT_GOTO_END_ON_ACTIVATE as u32;
        SIMPLE = sys::NK_EDIT_SIMPLE as u32;
        FIELD = sys::NK_EDIT_FIELD as u32;
        BOX = sys::NK_EDIT_BOX as u32;
        EDITOR = sys::NK_EDIT_EDITOR as u32;
    }
}

flags! {
    /// Text edit event flags.
    EditEvents {
        ACTIVE = sys::NK_EDIT_ACTIVE as u32;
        INACTIVE = sys::NK_EDIT_INACTIVE as u32;
        ACTIVATED = sys::NK_EDIT_ACTIVATED as u32;
        DEACTIVATED = sys::NK_EDIT_DEACTIVATED as u32;
        COMMITTED = sys::NK_EDIT_COMMITTED as u32;
    }
}

flags! {
    /// Chart event flags.
    ChartEvents {
        HOVERING = sys::NK_CHART_HOVERING as u32;
        CLICKED = sys::NK_CHART_CLICKED as u32;
    }
}

/// Make a `#[repr(u32)]` enum. Each variant takes its value from a `sys` constant.
macro_rules! sys_enum {
    ($(#[$m:meta])* $name:ident { $($v:ident = $c:ident,)+ }) => {
        $(#[$m])*
        #[repr(u32)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name {
            $(
                #[doc = concat!("The `", stringify!($c), "` value.")]
                $v = sys::$c as u32,
            )+
        }
        impl $name {
            #[allow(dead_code)] // Later tasks call this function.
            pub(crate) fn raw(self) -> u32 { self as u32 }
        }
    };
}

sys_enum! {
    /// A keyboard key.
    Key {
        None = NK_KEY_NONE, Shift = NK_KEY_SHIFT, Ctrl = NK_KEY_CTRL, Del = NK_KEY_DEL,
        Enter = NK_KEY_ENTER, Tab = NK_KEY_TAB, Backspace = NK_KEY_BACKSPACE, Copy = NK_KEY_COPY,
        Cut = NK_KEY_CUT, Paste = NK_KEY_PASTE, Up = NK_KEY_UP, Down = NK_KEY_DOWN,
        Left = NK_KEY_LEFT, Right = NK_KEY_RIGHT,
        TextInsertMode = NK_KEY_TEXT_INSERT_MODE, TextReplaceMode = NK_KEY_TEXT_REPLACE_MODE,
        TextResetMode = NK_KEY_TEXT_RESET_MODE, TextLineStart = NK_KEY_TEXT_LINE_START,
        TextLineEnd = NK_KEY_TEXT_LINE_END, TextStart = NK_KEY_TEXT_START,
        TextEnd = NK_KEY_TEXT_END, TextUndo = NK_KEY_TEXT_UNDO, TextRedo = NK_KEY_TEXT_REDO,
        TextSelectAll = NK_KEY_TEXT_SELECT_ALL, TextWordLeft = NK_KEY_TEXT_WORD_LEFT,
        TextWordRight = NK_KEY_TEXT_WORD_RIGHT,
        ScrollStart = NK_KEY_SCROLL_START, ScrollEnd = NK_KEY_SCROLL_END,
        ScrollDown = NK_KEY_SCROLL_DOWN, ScrollUp = NK_KEY_SCROLL_UP, Alt = NK_KEY_ALT,
        F1 = NK_KEY_F1, F2 = NK_KEY_F2, F3 = NK_KEY_F3, F4 = NK_KEY_F4, F5 = NK_KEY_F5,
        F6 = NK_KEY_F6, F7 = NK_KEY_F7, F8 = NK_KEY_F8, F9 = NK_KEY_F9, F10 = NK_KEY_F10,
        F11 = NK_KEY_F11, F12 = NK_KEY_F12,
    }
}

sys_enum! {
    /// A mouse button.
    Button {
        Left = NK_BUTTON_LEFT, Middle = NK_BUTTON_MIDDLE, Right = NK_BUTTON_RIGHT,
        Double = NK_BUTTON_DOUBLE,
    }
}

sys_enum! {
    /// A symbol that a widget can draw.
    Symbol {
        None = NK_SYMBOL_NONE, X = NK_SYMBOL_X, Underscore = NK_SYMBOL_UNDERSCORE,
        CircleSolid = NK_SYMBOL_CIRCLE_SOLID, CircleOutline = NK_SYMBOL_CIRCLE_OUTLINE,
        RectSolid = NK_SYMBOL_RECT_SOLID, RectOutline = NK_SYMBOL_RECT_OUTLINE,
        TriangleUp = NK_SYMBOL_TRIANGLE_UP, TriangleDown = NK_SYMBOL_TRIANGLE_DOWN,
        TriangleLeft = NK_SYMBOL_TRIANGLE_LEFT, TriangleRight = NK_SYMBOL_TRIANGLE_RIGHT,
        Plus = NK_SYMBOL_PLUS, Minus = NK_SYMBOL_MINUS,
        TriangleUpOutline = NK_SYMBOL_TRIANGLE_UP_OUTLINE,
        TriangleDownOutline = NK_SYMBOL_TRIANGLE_DOWN_OUTLINE,
        TriangleLeftOutline = NK_SYMBOL_TRIANGLE_LEFT_OUTLINE,
        TriangleRightOutline = NK_SYMBOL_TRIANGLE_RIGHT_OUTLINE,
        ChevronUp = NK_SYMBOL_CHEVRON_UP, ChevronRight = NK_SYMBOL_CHEVRON_RIGHT,
        ChevronDown = NK_SYMBOL_CHEVRON_DOWN, ChevronLeft = NK_SYMBOL_CHEVRON_LEFT,
        Hamburger = NK_SYMBOL_HAMBURGER,
    }
}

sys_enum! {
    /// The direction of an arrow.
    Heading { Up = NK_UP, Right = NK_RIGHT, Down = NK_DOWN, Left = NK_LEFT, }
}

sys_enum! {
    /// The repeat behavior of a button.
    ButtonBehavior { Default = NK_BUTTON_DEFAULT, Repeater = NK_BUTTON_REPEATER, }
}

sys_enum! {
    /// The collapse state of a window or a tree.
    CollapseState { Minimized = NK_MINIMIZED, Maximized = NK_MAXIMIZED, }
}

impl CollapseState {
    /// Make a state from a C value. A non-zero value gives `Maximized`.
    pub(crate) fn from_raw(v: u32) -> CollapseState {
        if v != 0 {
            CollapseState::Maximized
        } else {
            CollapseState::Minimized
        }
    }
}

sys_enum! {
    /// The visibility state of a window.
    ShowState { Hidden = NK_HIDDEN, Shown = NK_SHOWN, }
}

sys_enum! {
    /// The type of a chart.
    ChartType { Lines = NK_CHART_LINES, Column = NK_CHART_COLUMN, }
}

sys_enum! {
    /// The type of a popup.
    PopupType { Static = NK_POPUP_STATIC, Dynamic = NK_POPUP_DYNAMIC, }
}

sys_enum! {
    /// The format of a layout row.
    LayoutFormat { Dynamic = NK_DYNAMIC, Static = NK_STATIC, }
}

sys_enum! {
    /// The type of a tree.
    TreeType { Node = NK_TREE_NODE, Tab = NK_TREE_TAB, }
}

sys_enum! {
    /// The format of a color picker.
    ColorFormat { Rgb = NK_RGB, Rgba = NK_RGBA, }
}

sys_enum! {
    /// The anti-aliasing setting.
    AntiAliasing { Off = NK_ANTI_ALIASING_OFF, On = NK_ANTI_ALIASING_ON, }
}

sys_enum! {
    /// The pixel format of a font atlas.
    AtlasFormat { Alpha8 = NK_FONT_ATLAS_ALPHA8, Rgba32 = NK_FONT_ATLAS_RGBA32, }
}

sys_enum! {
    /// The attribute of a vertex element.
    VertexAttribute {
        Position = NK_VERTEX_POSITION, Color = NK_VERTEX_COLOR, Texcoord = NK_VERTEX_TEXCOORD,
    }
}

sys_enum! {
    /// The data format of a vertex element.
    VertexFormat {
        SChar = NK_FORMAT_SCHAR, SShort = NK_FORMAT_SSHORT, SInt = NK_FORMAT_SINT,
        UChar = NK_FORMAT_UCHAR, UShort = NK_FORMAT_USHORT, UInt = NK_FORMAT_UINT,
        Float = NK_FORMAT_FLOAT, Double = NK_FORMAT_DOUBLE, R8G8B8 = NK_FORMAT_R8G8B8,
        R16G15B16 = NK_FORMAT_R16G15B16, R32G32B32 = NK_FORMAT_R32G32B32,
        R8G8B8A8 = NK_FORMAT_R8G8B8A8, B8G8R8A8 = NK_FORMAT_B8G8R8A8,
        R16G15B16A16 = NK_FORMAT_R16G15B16A16, R32G32B32A32 = NK_FORMAT_R32G32B32A32,
        R32G32B32A32Float = NK_FORMAT_R32G32B32A32_FLOAT,
        R32G32B32A32Double = NK_FORMAT_R32G32B32A32_DOUBLE,
        Rgb32 = NK_FORMAT_RGB32, Rgba32 = NK_FORMAT_RGBA32,
    }
}

/// A character filter for text edit widgets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Filter {
    /// Accept all characters.
    Default,
    /// Accept ASCII characters.
    Ascii,
    /// Accept characters of a float number.
    Float,
    /// Accept decimal digits.
    Decimal,
    /// Accept hexadecimal digits.
    Hex,
    /// Accept octal digits.
    Oct,
    /// Accept binary digits.
    Binary,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn rect_roundtrip() { let r = Rect { x: 1.0, y: 2.0, w: 3.0, h: 4.0 }; assert_eq!(Rect::from(sys::nk_rect::from(r)), r); }
    #[test] fn handle_id_and_ptr_read_back() { assert_eq!(Handle::id(7).as_id(), 7); assert!(Handle::id(0).as_ptr().is_null()); let p = 0x10usize as *mut std::ffi::c_void; assert_eq!(Handle::ptr(p).as_ptr(), p); }
    #[test] fn flags_or_and_contains() { let f = PanelFlags::BORDER | PanelFlags::TITLE; assert!(f.contains(PanelFlags::TITLE)); assert!(!f.contains(PanelFlags::MOVABLE)); assert_eq!(f.bits(), sys::NK_WINDOW_BORDER as u32 | sys::NK_WINDOW_TITLE as u32); }
    #[test] fn c_bits_drops_movable_without_input() { let m = PanelFlags::MOVABLE; assert_eq!(m.c_bits(), m.bits()); assert_eq!((m | PanelFlags::NO_INPUT).c_bits(), PanelFlags::NO_INPUT.bits()); }
    #[test] fn enum_values_match_sys() { assert_eq!(Key::Enter.raw(), sys::NK_KEY_ENTER as u32); assert_eq!(Symbol::Hamburger.raw(), sys::NK_SYMBOL_HAMBURGER as u32); assert_eq!(VertexFormat::Rgba32.raw(), sys::NK_FORMAT_RGBA32 as u32); }
    #[test] fn image_roundtrip_through_sys() { let i = Image::new(Handle::id(9), 4, 5); let back = Image::from_sys(sys::nk_image::from(i)); assert_eq!(back, i); assert_eq!(back.handle.as_id(), 9); assert_eq!(back.region, [0, 0, 4, 5]); }
    #[test] fn cstr_truncates_at_nul() { assert_eq!(crate::cstr("ab\0cd").as_bytes(), b"ab"); }
    #[test] fn from_bits_drops_private_bits() { assert_eq!(PanelFlags::from_bits(1 << 12), PanelFlags::empty()); }
}
