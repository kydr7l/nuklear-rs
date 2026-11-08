//! Run one frame without a window. Print the draw commands, then convert them to vertex data.

use nuklear::*;

/// A font where every character is 6 pixels wide and 12 pixels high.
struct FixedFont;

impl Font for FixedFont {
    fn height(&self) -> f32 {
        12.0
    }
    fn width(&self, _height: f32, text: &str) -> f32 {
        6.0 * text.chars().count() as f32
    }
    fn glyph(&self, _height: f32, _codepoint: char, _next: char) -> Glyph {
        Glyph {
            width: 6.0,
            height: 12.0,
            xadvance: 6.0,
            ..Default::default()
        }
    }
    fn texture(&self) -> Handle {
        Handle::id(1)
    }
}

fn main() {
    let font = UserFont::new(FixedFont);
    let mut ctx = Context::new(&font).expect("cannot make the context");

    // Build the frame: one window with a label, a button and a checkbox.
    let mut checked = false;
    ctx.input(|_| {});
    ctx.window(
        "Demo",
        Rect {
            x: 10.0,
            y: 10.0,
            w: 220.0,
            h: 160.0,
        },
        PanelFlags::TITLE | PanelFlags::BORDER,
        |w| {
            w.layout_row_dynamic(30.0, 1);
            w.label("hello", TextAlign::LEFT);
            w.button_label("Click");
            w.checkbox("Check", &mut checked);
        },
    );

    // Print every draw command.
    for command in ctx.commands() {
        println!("{command:?}");
    }

    // Convert the same frame. The layout has 20 bytes: position, texture coordinates and color.
    let layout = VertexLayout::new(
        20,
        4,
        &[
            (VertexAttribute::Position, VertexFormat::Float, 0),
            (VertexAttribute::Texcoord, VertexFormat::Float, 8),
            (VertexAttribute::Color, VertexFormat::R8G8B8A8, 16),
        ],
    )
    .expect("the layout is valid");
    let null = NullTexture {
        texture: Handle::id(1),
        uv: Vec2 { x: 0.5, y: 0.5 },
    };
    let config = ConvertConfig::new(&layout, null);
    let (mut cmds, mut vertices, mut elements) = (Buffer::new(), Buffer::new(), Buffer::new());
    let draw_commands = ctx
        .convert(&mut cmds, &mut vertices, &mut elements, &config)
        .expect("convert failed")
        .count();
    println!("draw commands: {draw_commands}");
    println!("vertices: {}", vertices.len() / 20);
    println!("indices: {}", elements.as_indices().len());

    ctx.clear();
}
