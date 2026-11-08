mod common;
use nuklear::*;

fn layout() -> VertexLayout {
    VertexLayout::new(
        20,
        4,
        &[
            (VertexAttribute::Position, VertexFormat::Float, 0),
            (VertexAttribute::Texcoord, VertexFormat::Float, 8),
            (VertexAttribute::Color, VertexFormat::R8G8B8A8, 16),
        ],
    )
    .unwrap()
}

#[test]
fn convert_fills_buffers_consistently() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    ctx.window(
        "w",
        Rect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 200.0,
        },
        PanelFlags::TITLE | PanelFlags::BORDER,
        |w| {
            w.layout_row_dynamic(30.0, 1);
            w.label("hello", TextAlign::LEFT);
            w.button_label("b");
        },
    );
    let layout = layout();
    let null = NullTexture {
        texture: Handle::id(1),
        uv: Vec2 { x: 0.5, y: 0.5 },
    };
    let cfg = ConvertConfig::new(&layout, null);
    let (mut cmds, mut verts, mut elems) = (Buffer::new(), Buffer::new(), Buffer::new());
    let total: u32 = ctx
        .convert(&mut cmds, &mut verts, &mut elems, &cfg)
        .unwrap()
        .map(|d| d.elem_count)
        .sum();
    assert!(total > 0);
    assert_eq!(total as usize, elems.as_indices().len());
    assert!(!verts.is_empty() && verts.len() % 20 == 0);
    let max_index = *elems.as_indices().iter().max().unwrap() as usize;
    assert!(max_index < verts.len() / 20);
    ctx.clear();
}
#[test]
fn vertex_layout_rejects_bad_elements() {
    assert_eq!(
        VertexLayout::new(0, 4, &[]).unwrap_err(),
        LayoutError::ZeroSize
    );
    assert_eq!(
        VertexLayout::new(20, 3, &[]).unwrap_err(),
        LayoutError::BadAlignment
    );
    assert_eq!(
        VertexLayout::new(
            20,
            4,
            &[(VertexAttribute::Position, VertexFormat::Float, 16)]
        )
        .unwrap_err(),
        LayoutError::OutOfBounds {
            attribute: VertexAttribute::Position,
            offset: 16
        }
    );
    assert_eq!(
        VertexLayout::new(20, 4, &[(VertexAttribute::Color, VertexFormat::Float, 0)]).unwrap_err(),
        LayoutError::FormatMismatch {
            attribute: VertexAttribute::Color,
            format: VertexFormat::Float
        }
    );
    assert_eq!(
        VertexLayout::new(
            20,
            4,
            &[(VertexAttribute::Position, VertexFormat::Rgba32, 0)]
        )
        .unwrap_err(),
        LayoutError::FormatMismatch {
            attribute: VertexAttribute::Position,
            format: VertexFormat::Rgba32
        }
    );
    assert_eq!(
        VertexLayout::new(
            20,
            4,
            &[(VertexAttribute::Position, VertexFormat::Float, usize::MAX)]
        )
        .unwrap_err(),
        LayoutError::OutOfBounds {
            attribute: VertexAttribute::Position,
            offset: usize::MAX
        }
    );
    assert!(VertexLayout::new(
        16,
        4,
        &[(VertexAttribute::Position, VertexFormat::Float, 8)]
    )
    .is_ok());
}
#[test]
fn empty_buffer_slices_are_empty() {
    let b = Buffer::new();
    assert!(b.as_bytes().is_empty());
    assert!(b.as_indices().is_empty());
    assert!(b.is_empty());
}
#[test]
fn convert_on_empty_frame_yields_no_draw_commands() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let layout = layout();
    let cfg = ConvertConfig::new(
        &layout,
        NullTexture {
            texture: Handle::id(1),
            uv: Vec2::default(),
        },
    );
    let (mut cmds, mut verts, mut elems) = (Buffer::new(), Buffer::new(), Buffer::new());
    let n = ctx
        .convert(&mut cmds, &mut verts, &mut elems, &cfg)
        .unwrap()
        .count();
    assert_eq!(n, 0);
    ctx.clear();
}

// The tests below cover the rulings and the C facts that the brief does not list.

#[test]
fn r8g8b8_needs_the_four_bytes_that_nuklear_writes() {
    assert_eq!(VertexFormat::R8G8B8.size(), 4);
    assert_eq!(
        VertexLayout::new(3, 1, &[(VertexAttribute::Color, VertexFormat::R8G8B8, 0)]).unwrap_err(),
        LayoutError::OutOfBounds {
            attribute: VertexAttribute::Color,
            offset: 0
        }
    );
    assert!(VertexLayout::new(4, 1, &[(VertexAttribute::Color, VertexFormat::R8G8B8, 0)]).is_ok());
    assert_eq!(
        VertexLayout::new(
            15,
            1,
            &[(VertexAttribute::Position, VertexFormat::Double, 0)]
        )
        .unwrap_err(),
        LayoutError::OutOfBounds {
            attribute: VertexAttribute::Position,
            offset: 0
        }
    );
}
#[test]
fn vertex_layout_rejects_large_values_and_many_elements() {
    assert_eq!(
        VertexLayout::new(1025, 4, &[]).unwrap_err(),
        LayoutError::TooLarge
    );
    assert_eq!(
        VertexLayout::new(1024, 2048, &[]).unwrap_err(),
        LayoutError::TooLarge
    );
    assert!(VertexLayout::new(1024, 1024, &[]).is_ok());
    // TooLarge comes before TooManyElements and before the element checks.
    assert_eq!(
        VertexLayout::new(
            4096,
            4,
            &[(VertexAttribute::Position, VertexFormat::Float, 9999); 17]
        )
        .unwrap_err(),
        LayoutError::TooLarge
    );
    assert_eq!(
        VertexLayout::new(
            20,
            4,
            &[(VertexAttribute::Position, VertexFormat::Float, 0); 17]
        )
        .unwrap_err(),
        LayoutError::TooManyElements
    );
    assert!(VertexLayout::new(
        20,
        4,
        &[(VertexAttribute::Position, VertexFormat::Float, 0); 16]
    )
    .is_ok());
}
#[test]
fn convert_rejects_segment_counts_that_never_end() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let layout = layout();
    let mut cfg = ConvertConfig::new(
        &layout,
        NullTexture {
            texture: Handle::id(1),
            uv: Vec2::default(),
        },
    );
    cfg.arc_segment_count = u32::MAX;
    let (mut cmds, mut verts, mut elems) = (Buffer::new(), Buffer::new(), Buffer::new());
    assert!(ctx
        .convert(&mut cmds, &mut verts, &mut elems, &cfg)
        .is_err());
    ctx.clear();
}
#[test]
fn convert_twice_restarts_the_buffers() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    ctx.window(
        "w",
        Rect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 200.0,
        },
        PanelFlags::BORDER,
        |w| {
            w.layout_row_dynamic(30.0, 1);
            w.button_label("b");
        },
    );
    let layout = layout();
    let cfg = ConvertConfig::new(
        &layout,
        NullTexture {
            texture: Handle::id(1),
            uv: Vec2::default(),
        },
    );
    let (mut cmds, mut verts, mut elems) = (Buffer::new(), Buffer::new(), Buffer::new());
    let first = ctx
        .convert(&mut cmds, &mut verts, &mut elems, &cfg)
        .unwrap()
        .count();
    let (v, e) = (verts.len(), elems.len());
    let second = ctx
        .convert(&mut cmds, &mut verts, &mut elems, &cfg)
        .unwrap()
        .count();
    assert_eq!((first, v, e), (second, verts.len(), elems.len()));
    // The window is dropped, because convert ended the frame build.
    assert!(ctx
        .window(
            "x",
            Rect {
                x: 0.0,
                y: 0.0,
                w: 50.0,
                h: 50.0
            },
            PanelFlags::BORDER,
            |_| ()
        )
        .is_none());
    ctx.clear();
}
#[test]
fn large_alignment_converts_without_overflow() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    ctx.window(
        "w",
        Rect {
            x: 0.0,
            y: 0.0,
            w: 300.0,
            h: 300.0,
        },
        PanelFlags::TITLE | PanelFlags::BORDER,
        |w| {
            w.layout_row_dynamic(30.0, 1);
            for _ in 0..5 {
                w.button_label("b");
            }
        },
    );
    let layout = VertexLayout::new(
        1000,
        1024,
        &[
            (VertexAttribute::Position, VertexFormat::Float, 0),
            (VertexAttribute::Color, VertexFormat::R8G8B8, 996),
        ],
    )
    .unwrap();
    let cfg = ConvertConfig::new(
        &layout,
        NullTexture {
            texture: Handle::id(1),
            uv: Vec2::default(),
        },
    );
    let (mut cmds, mut verts, mut elems) = (Buffer::new(), Buffer::new(), Buffer::new());
    let total: u32 = ctx
        .convert(&mut cmds, &mut verts, &mut elems, &cfg)
        .unwrap()
        .map(|d| d.elem_count)
        .sum();
    assert_eq!(total as usize, elems.as_indices().len());
    assert!(verts.as_bytes().len() >= 1000);
    ctx.clear();
}
/// A font with zero text width, so Nuklear never shortens a label to the widget width.
struct ZeroFont;
impl Font for ZeroFont {
    fn height(&self) -> f32 {
        12.0
    }
    fn width(&self, _h: f32, _t: &str) -> f32 {
        0.0
    }
    fn glyph(&self, _h: f32, _c: char, _n: char) -> Glyph {
        Glyph {
            height: 12.0,
            ..Default::default()
        }
    }
    fn texture(&self) -> Handle {
        Handle::id(1)
    }
}
// This test runs the grow path of the context command memory with unaligned text commands near power-of-two sizes.
// It cannot detect an overrun without a heap checker.
#[test]
fn long_labels_grow_the_command_memory() {
    let font = UserFont::new(ZeroFont);
    let mut ctx = Context::new(&font).unwrap();
    let layout = layout();
    let cfg = ConvertConfig::new(
        &layout,
        NullTexture {
            texture: Handle::id(1),
            uv: Vec2::default(),
        },
    );
    let (mut cmds, mut verts, mut elems) = (Buffer::new(), Buffer::new(), Buffer::new());
    let lengths = (3970..4110)
        .chain((8060..8200).step_by(3))
        .chain((16260..16400).step_by(7))
        .chain([20_000]);
    for n in lengths {
        let text = "x".repeat(n);
        ctx.window(
            "w",
            Rect {
                x: 0.0,
                y: 0.0,
                w: 200.0,
                h: 200.0,
            },
            PanelFlags::BORDER,
            |w| {
                w.layout_row_dynamic(30.0, 1);
                w.label(&text, TextAlign::LEFT);
            },
        );
        assert!(ctx
            .commands()
            .any(|c| matches!(c, Command::Text { ref text, .. } if text.len() == n)));
        // Each glyph makes 4 vertices, and the 16-bit index limit is 65535 vertices.
        if n < 12_000 {
            assert!(
                ctx.convert(&mut cmds, &mut verts, &mut elems, &cfg)
                    .unwrap()
                    .count()
                    > 0
            );
        }
        ctx.clear();
    }
}
// The Buffer memory starts zeroed. Nuklear must not write the 4 bytes after the color of any vertex.
// Anti-aliasing is off, because the AA paths keep temporary normals in the free vertex memory, and later vertices keep those bytes in their padding.
#[test]
fn color_formats_write_only_inside_the_vertex() {
    use VertexFormat::*;
    let formats = [
        R8G8B8,
        R16G15B16,
        R32G32B32,
        R8G8B8A8,
        B8G8R8A8,
        R16G15B16A16,
        R32G32B32A32,
        R32G32B32A32Float,
        R32G32B32A32Double,
        Rgb32,
        Rgba32,
    ];
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    for fmt in formats {
        let size = 8 + fmt.size() + 4;
        let layout = VertexLayout::new(
            size,
            4,
            &[
                (VertexAttribute::Position, Float, 0),
                (VertexAttribute::Color, fmt, 8),
            ],
        )
        .unwrap();
        let mut cfg = ConvertConfig::new(
            &layout,
            NullTexture {
                texture: Handle::id(1),
                uv: Vec2::default(),
            },
        );
        (cfg.line_aa, cfg.shape_aa) = (AntiAliasing::Off, AntiAliasing::Off);
        ctx.window(
            "w",
            Rect {
                x: 0.0,
                y: 0.0,
                w: 200.0,
                h: 200.0,
            },
            PanelFlags::TITLE | PanelFlags::BORDER,
            |w| {
                w.layout_row_dynamic(30.0, 1);
                w.label("hello", TextAlign::LEFT);
            },
        );
        let (mut cmds, mut verts, mut elems) = (Buffer::new(), Buffer::new(), Buffer::new());
        assert!(
            ctx.convert(&mut cmds, &mut verts, &mut elems, &cfg)
                .unwrap()
                .count()
                > 0,
            "{fmt:?}"
        );
        let bytes = verts.as_bytes();
        assert!(!bytes.is_empty() && bytes.len() % size == 0, "{fmt:?}");
        for (i, v) in bytes.chunks_exact(size).enumerate() {
            assert_eq!(v[size - 4..], [0; 4], "{fmt:?}, vertex {i}");
        }
        ctx.clear();
    }
}
#[test]
fn image_with_a_pointer_handle_converts() {
    let mut texel = 0u8;
    let tex = Handle::ptr((&mut texel as *mut u8).cast());
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    ctx.window(
        "w",
        Rect {
            x: 0.0,
            y: 0.0,
            w: 200.0,
            h: 200.0,
        },
        PanelFlags::BORDER,
        |w| {
            w.layout_row_dynamic(30.0, 1);
            w.image(Image::new(tex, 8, 8));
        },
    );
    let layout = layout();
    let cfg = ConvertConfig::new(
        &layout,
        NullTexture {
            texture: Handle::id(1),
            uv: Vec2::default(),
        },
    );
    let (mut cmds, mut verts, mut elems) = (Buffer::new(), Buffer::new(), Buffer::new());
    let textures: Vec<_> = ctx
        .convert(&mut cmds, &mut verts, &mut elems, &cfg)
        .unwrap()
        .map(|d| d.texture.as_ptr())
        .collect();
    assert!(textures.contains(&tex.as_ptr()), "{textures:?}");
    ctx.clear();
}
