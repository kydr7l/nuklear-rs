//! Vertex buffer output: the buffers, the vertex layout, the conversion and the draw command iterator.

use crate::{sys, AntiAliasing, Context, Handle, NullTexture, Rect, VertexAttribute, VertexFormat};
use std::ffi::c_void;
use std::marker::PhantomData;
use std::ptr::{self, NonNull};

extern "C" {
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn free(p: *mut c_void);
}

/// The largest vertex size and the largest vertex alignment that `VertexLayout::new` accepts.
const MAX_VERTEX: usize = 1024;

/// The largest segment count that `Context::convert` accepts. A 16-bit index cannot address more vertices.
const MAX_SEGMENTS: u32 = u16::MAX as u32;

/// The allocation callback of every `Buffer` and of the context memory.
///
/// `nk_buffer_alloc` grows a buffer without the alignment padding, so an aligned write can go up to
/// `align - 1` bytes past the end. The largest `align` is `MAX_VERTEX` (vertices); commands and indices use less.
/// Each block has `MAX_VERTEX` extra bytes to hold that write.
/// `nk_buffer_realloc` copies the old block itself, so this function ignores `_old` and always allocates a new block.
unsafe extern "C" fn buffer_alloc(
    _: sys::nk_handle,
    _old: *mut c_void,
    size: sys::nk_size,
) -> *mut c_void {
    match size.checked_add(MAX_VERTEX) {
        // SAFETY: calloc has no precondition. It returns null or a zeroed block, so all bytes of the block are initialized.
        Some(n) => unsafe { calloc(1, n) },
        None => ptr::null_mut(),
    }
}

/// The free callback of `slack_allocator`.
unsafe extern "C" fn buffer_free(_: sys::nk_handle, p: *mut c_void) {
    // SAFETY: Nuklear gives only pointers that `buffer_alloc` returned, and each pointer only once. They come from calloc.
    unsafe { free(p) }
}

/// Get the allocator for every Nuklear buffer of this crate: calloc with `MAX_VERTEX` extra bytes per block.
/// Nuklear copies the allocator, so a temporary value is enough.
pub(crate) fn slack_allocator() -> sys::nk_allocator {
    sys::nk_allocator {
        userdata: sys::nk_handle::default(),
        alloc: Some(buffer_alloc),
        free: Some(buffer_free),
    }
}

/// A growable Nuklear memory buffer. `Context::convert` fills it with draw commands, vertices or indices.
///
/// All memory of the buffer starts as zero bytes, so the bytes that Nuklear skips for alignment are initialized.
pub struct Buffer {
    /// This pointer comes from `Box::into_raw`. The draw list of the context stores pointers to the buffer, so it must not move.
    raw: NonNull<sys::nk_buffer>,
}

impl Buffer {
    /// Make an empty buffer with 4096 bytes of memory.
    pub fn new() -> Buffer {
        let p = Box::into_raw(Box::<sys::nk_buffer>::default());
        // SAFETY: `p` comes from `Box::into_raw`, and no other reference to it exists. `nk_buffer_init` copies the allocator.
        let ptr = unsafe {
            sys::nk_buffer_init(p, &slack_allocator(), 4096);
            (*p).memory.ptr
        };
        // With a null block, Nuklear would write through a null pointer, because the size is already set.
        if ptr.is_null() {
            std::alloc::handle_alloc_error(
                std::alloc::Layout::from_size_align(4096 + MAX_VERTEX, 1).unwrap(),
            );
        }
        // SAFETY: `Box::into_raw` never returns a null pointer.
        Buffer {
            raw: unsafe { NonNull::new_unchecked(p) },
        }
    }

    /// Get the used bytes of the buffer.
    pub fn as_bytes(&self) -> &[u8] {
        let b = self.raw.as_ptr();
        // SAFETY: `b` is a live buffer. The block holds at least `allocated` bytes: `allocated` is at most the block size
        // plus the alignment padding, and `buffer_alloc` adds `MAX_VERTEX` bytes for that padding. calloc zeroed the block,
        // and Nuklear copies only initialized bytes into it. The shared borrow of `self` blocks all changes for the slice lifetime.
        unsafe {
            let p = sys::nk_buffer_memory_const(b).cast::<u8>();
            if p.is_null() {
                return &[];
            }
            std::slice::from_raw_parts(p, (*b).allocated)
        }
    }

    /// Get the used bytes of the buffer as 16-bit indices. The slice is empty when the memory is not aligned for `u16`.
    pub fn as_indices(&self) -> &[u16] {
        // SAFETY: every bit pattern is a valid `u16`.
        let (prefix, indices, _) = unsafe { self.as_bytes().align_to::<u16>() };
        if prefix.is_empty() {
            indices
        } else {
            &[]
        }
    }

    /// Get the number of used bytes.
    pub fn len(&self) -> usize {
        // SAFETY: `raw` is a live buffer.
        unsafe { (*self.raw.as_ptr()).allocated }
    }

    /// Check if the buffer has no used bytes.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Remove all used bytes. The buffer keeps its memory.
    pub fn clear(&mut self) {
        // SAFETY: `raw` is a live buffer, and `self` is borrowed mutably.
        unsafe { sys::nk_buffer_clear(self.raw.as_ptr()) }
    }
}

impl Default for Buffer {
    fn default() -> Buffer {
        Buffer::new()
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        let p = self.raw.as_ptr();
        // SAFETY: `p` comes from `Box::into_raw` and is live. `nk_buffer_free` frees the block once through `buffer_free`.
        // No `DrawCommands` lives, because it borrows the buffer. The draw list of a context can keep the pointer,
        // but only `nk_convert` reads that pointer, and `nk_convert` sets it again first.
        unsafe {
            sys::nk_buffer_free(p);
            drop(Box::from_raw(p));
        }
    }
}

/// The error of `VertexLayout::new`.
#[derive(Debug, PartialEq, Eq)]
pub enum LayoutError {
    /// The vertex size is zero.
    ZeroSize,
    /// The vertex alignment is not a power of two.
    BadAlignment,
    /// The vertex size or the vertex alignment is more than 1024.
    TooLarge,
    /// An element does not fit in the vertex.
    OutOfBounds {
        /// The attribute of the element.
        attribute: VertexAttribute,
        /// The offset of the element.
        offset: usize,
    },
    /// A color attribute has a value format, or a value attribute has a color format.
    FormatMismatch {
        /// The attribute of the element.
        attribute: VertexAttribute,
        /// The format of the element.
        format: VertexFormat,
    },
    /// The layout has more than 16 elements.
    TooManyElements,
}

impl VertexFormat {
    /// Get the number of bytes that Nuklear writes for one value of this format.
    ///
    /// A position and a texture coordinate have two values. A color has one value.
    /// For color formats, the size is the `NK_MEMCPY` size in `nk_draw_vertex_color`.
    pub const fn size(self) -> usize {
        match self {
            VertexFormat::SChar | VertexFormat::UChar => 1,
            VertexFormat::SShort | VertexFormat::UShort => 2,
            VertexFormat::SInt | VertexFormat::UInt | VertexFormat::Float => 4,
            VertexFormat::Double => 8,
            // `sizeof(struct nk_color)`. Nuklear writes the alpha byte too, so this is 4 and not 3.
            VertexFormat::R8G8B8 => 4,
            // `sizeof(nk_ushort[3])`.
            VertexFormat::R16G15B16 => 6,
            // `sizeof(nk_uint[3])`.
            VertexFormat::R32G32B32 => 12,
            // `sizeof(struct nk_color)`.
            VertexFormat::R8G8B8A8 | VertexFormat::B8G8R8A8 => 4,
            // `sizeof(nk_ushort[4])`.
            VertexFormat::R16G15B16A16 => 8,
            // `sizeof(nk_uint[4])` and `sizeof(float)*4`.
            VertexFormat::R32G32B32A32 | VertexFormat::R32G32B32A32Float => 16,
            // `sizeof(double[4])`.
            VertexFormat::R32G32B32A32Double => 32,
            // `sizeof(nk_uint)`.
            VertexFormat::Rgb32 | VertexFormat::Rgba32 => 4,
        }
    }

    /// Check if this format is a color format.
    pub const fn is_color(self) -> bool {
        self as u32 >= VertexFormat::R8G8B8 as u32
    }
}

/// The vertex layout for `Context::convert`. `new` makes sure that Nuklear writes only inside each vertex.
#[derive(Debug)]
pub struct VertexLayout {
    /// The elements and the end marker. Nuklear reads until the end marker.
    elements: Vec<sys::nk_draw_vertex_layout_element>,
    vertex_size: usize,
    vertex_align: usize,
}

impl VertexLayout {
    /// Make a layout for vertices of `vertex_size` bytes with the alignment `vertex_align`.
    ///
    /// Each attribute is an `(attribute, format, offset)` tuple. The offset is in bytes from the start of the vertex.
    pub fn new(
        vertex_size: usize,
        vertex_align: usize,
        attributes: &[(VertexAttribute, VertexFormat, usize)],
    ) -> Result<VertexLayout, LayoutError> {
        if vertex_size == 0 {
            return Err(LayoutError::ZeroSize);
        }
        if !vertex_align.is_power_of_two() {
            return Err(LayoutError::BadAlignment);
        }
        if vertex_size > MAX_VERTEX || vertex_align > MAX_VERTEX {
            return Err(LayoutError::TooLarge);
        }
        if attributes.len() > 16 {
            return Err(LayoutError::TooManyElements);
        }
        let mut elements = Vec::with_capacity(attributes.len() + 1);
        for &(attribute, format, offset) in attributes {
            let color = attribute == VertexAttribute::Color;
            if format.is_color() != color {
                return Err(LayoutError::FormatMismatch { attribute, format });
            }
            let bytes = if color {
                format.size()
            } else {
                2 * format.size()
            };
            if offset
                .checked_add(bytes)
                .is_none_or(|end| end > vertex_size)
            {
                return Err(LayoutError::OutOfBounds { attribute, offset });
            }
            elements.push(sys::nk_draw_vertex_layout_element {
                attribute: attribute.raw() as _,
                format: format.raw() as _,
                offset,
            });
        }
        elements.push(sys::nk_draw_vertex_layout_element {
            attribute: sys::NK_VERTEX_ATTRIBUTE_COUNT as _,
            format: sys::NK_FORMAT_COUNT as _,
            offset: 0,
        });
        Ok(VertexLayout {
            elements,
            vertex_size,
            vertex_align,
        })
    }
}

/// The settings of `Context::convert`.
pub struct ConvertConfig<'l> {
    /// The alpha factor for all shapes and text. `convert` clamps it to `0.0..=1.0`, and NaN becomes `0.0`.
    pub global_alpha: f32,
    /// The anti-aliasing of lines.
    pub line_aa: AntiAliasing,
    /// The anti-aliasing of filled shapes.
    pub shape_aa: AntiAliasing,
    /// The number of segments for a circle. `convert` fails when it is more than 65535.
    pub circle_segment_count: u32,
    /// The number of segments for an arc. `convert` fails when it is more than 65535.
    pub arc_segment_count: u32,
    /// The number of segments for a curve. `convert` fails when it is more than 65535.
    pub curve_segment_count: u32,
    /// The texture with a white pixel for shapes.
    pub null: NullTexture,
    /// The vertex layout.
    pub layout: &'l VertexLayout,
}

impl<'l> ConvertConfig<'l> {
    /// Make a config with alpha 1.0, anti-aliasing on, and 22 segments for circles, arcs and curves.
    pub fn new(layout: &'l VertexLayout, null: NullTexture) -> ConvertConfig<'l> {
        ConvertConfig {
            global_alpha: 1.0,
            line_aa: AntiAliasing::On,
            shape_aa: AntiAliasing::On,
            circle_segment_count: 22,
            arc_segment_count: 22,
            curve_segment_count: 22,
            null,
            layout,
        }
    }
}

/// The error of `Context::convert`. The value holds the `NK_CONVERT_*` flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConvertError(pub u32);

/// One draw call of the converted frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DrawCommand {
    /// The number of indices of the draw call.
    pub elem_count: u32,
    /// The clip rectangle.
    pub clip_rect: Rect,
    /// The texture.
    pub texture: Handle,
}

/// The iterator over the draw calls of a converted frame. Get it from `Context::convert`.
pub struct DrawCommands<'a> {
    ctx: *const sys::nk_context,
    buffer: *const sys::nk_buffer,
    next: *const sys::nk_draw_command,
    end: *const sys::nk_draw_command,
    _borrow: PhantomData<&'a mut Buffer>,
}

impl<'f> Context<'f> {
    /// Convert the draw commands of the frame into vertices, indices and draw calls.
    ///
    /// The method clears the three buffers first. Call it after all windows of the frame and before `clear`.
    /// The method ends the frame build. After it, `window` and `window_titled` return `None` until `clear` runs.
    /// The iterator borrows `cmds`, so this code does not compile:
    ///
    /// ```compile_fail,E0499
    /// use nuklear::*;
    /// struct F;
    /// impl Font for F {
    ///     fn height(&self) -> f32 { 12.0 }
    ///     fn width(&self, _h: f32, _t: &str) -> f32 { 0.0 }
    ///     fn glyph(&self, _h: f32, _c: char, _n: char) -> Glyph { Glyph::default() }
    ///     fn texture(&self) -> Handle { Handle::id(1) }
    /// }
    /// let font = UserFont::new(F);
    /// let mut ctx = Context::new(&font).unwrap();
    /// let layout = VertexLayout::new(8, 4, &[(VertexAttribute::Position, VertexFormat::Float, 0)]).unwrap();
    /// let cfg = ConvertConfig::new(&layout, NullTexture { texture: Handle::id(1), uv: Vec2::default() });
    /// let (mut cmds, mut verts, mut elems) = (Buffer::new(), Buffer::new(), Buffer::new());
    /// let mut draws = ctx.convert(&mut cmds, &mut verts, &mut elems, &cfg).unwrap();
    /// cmds.clear();
    /// draws.next();
    /// ```
    pub fn convert<'a>(
        &'a mut self,
        cmds: &'a mut Buffer,
        vertices: &mut Buffer,
        elements: &mut Buffer,
        config: &ConvertConfig<'_>,
    ) -> Result<DrawCommands<'a>, ConvertError> {
        let segments = [
            config.circle_segment_count,
            config.arc_segment_count,
            config.curve_segment_count,
        ];
        // `nk_draw_list_path_arc_to` and `_curve_to` loop with `i <= segments`, so `u32::MAX` never ends.
        if segments.iter().any(|&s| s > MAX_SEGMENTS) {
            return Err(ConvertError(sys::NK_CONVERT_INVALID_PARAM as u32));
        }
        let layout = config.layout;
        let alpha = config.global_alpha;
        let c = sys::nk_convert_config {
            // Nuklear casts `alpha * 255` to a byte, which is undefined in C outside the byte range.
            global_alpha: if alpha >= 0.0 { alpha.min(1.0) } else { 0.0 },
            line_AA: config.line_aa.raw() as _,
            shape_AA: config.shape_aa.raw() as _,
            circle_segment_count: config.circle_segment_count,
            arc_segment_count: config.arc_segment_count,
            curve_segment_count: config.curve_segment_count,
            tex_null: sys::nk_draw_null_texture {
                texture: config.null.texture.0,
                uv: config.null.uv.into(),
            },
            vertex_layout: layout.elements.as_ptr(),
            vertex_size: layout.vertex_size,
            vertex_alignment: layout.vertex_align,
        };
        // Nuklear starts the indices at zero for each call, so old data in the buffers would not match.
        cmds.clear();
        vertices.clear();
        elements.clear();
        self.begin_build();
        let ctx = self.raw();
        let buffer = cmds.raw.as_ptr();
        // SAFETY: the context and the three buffers are live and distinct, because they are borrowed mutably.
        // The layout ends with the end marker, and `VertexLayout::new` keeps every element write inside `vertex_size` bytes.
        // `begin_build` removed the stale popup links, so `nk_build` reads only commands of this frame.
        let res = unsafe {
            sys::nk_convert(
                ctx,
                buffer,
                vertices.raw.as_ptr(),
                elements.raw.as_ptr(),
                &c,
            )
        };
        if res != sys::NK_CONVERT_SUCCESS as u32 {
            return Err(ConvertError(res));
        }
        // SAFETY: `nk_convert` filled `buffer` and set the draw list of the context for it.
        let next = unsafe { sys::nk__draw_begin(ctx, buffer) };
        // `nk__draw_end` subtracts `cmd_count - 1`, which wraps for zero commands, so call it only when a command exists.
        // SAFETY: as above. At least one command exists.
        let end = if next.is_null() {
            ptr::null()
        } else {
            unsafe { sys::nk__draw_end(ctx, buffer) }
        };
        Ok(DrawCommands {
            ctx,
            buffer,
            next,
            end,
            _borrow: PhantomData,
        })
    }
}

impl Iterator for DrawCommands<'_> {
    type Item = DrawCommand;

    fn next(&mut self) -> Option<DrawCommand> {
        let cmd = self.next;
        if cmd.is_null() {
            return None;
        }
        self.next = if cmd == self.end {
            ptr::null()
        } else {
            // SAFETY: `cmd` comes from `nk__draw_begin` or `nk__draw_next` for this buffer and context.
            // Both stay borrowed for `'a`, so the draw list and the command memory do not change.
            unsafe { sys::nk__draw_next(cmd, self.buffer, self.ctx) }
        };
        // SAFETY: as above. Nuklear allocates the commands with the alignment of `nk_draw_command`.
        let c = unsafe { cmd.read() };
        // Nuklear copies each texture handle from a handle of this crate, so all bytes of the union are initialized.
        Some(DrawCommand {
            elem_count: c.elem_count,
            clip_rect: c.clip_rect.into(),
            texture: Handle(c.texture),
        })
    }
}
