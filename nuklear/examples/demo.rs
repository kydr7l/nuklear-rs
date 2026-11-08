//! Open a window with the Nuklear overview demo.
//!
//! The example has no GPU code. A small CPU rasterizer draws the triangles that `Context::convert` makes.
//! Run it with `cargo run -p nuklear --release --example demo`.

use nuklear::*;
use softbuffer::{Context as SbContext, Surface};
use std::{num::NonZeroU32, rc::Rc, time::Duration};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key as WKey, ModifiersState, NamedKey},
    window::{Window as Win, WindowId},
};

const BACKGROUND: [f32; 3] = [26.0, 46.0, 61.0];
/// The size of one vertex: position (2 floats), texture coordinates (2 floats) and color (4 bytes).
const VERTEX_SIZE: usize = 20;

// ---------------------------------------------------------------------------------------------
// The demo window
// ---------------------------------------------------------------------------------------------

struct Demo {
    status: String,
    quit: bool,
    time: f32,
    checked: bool,
    option: usize,
    slider: f32,
    progress: usize,
    property: i32,
    combo: usize,
    color: Colorf,
    selected: [bool; 4],
    popup: bool,
    text: TextEdit,
}

impl Demo {
    fn new() -> Demo {
        let mut text = TextEdit::new();
        text.set_text("Edit this text");
        Demo {
            status: "Ready".into(),
            quit: false,
            time: 0.0,
            checked: true,
            option: 0,
            slider: 0.5,
            progress: 40,
            property: 20,
            combo: 0,
            color: Colorf {
                r: 0.4,
                g: 0.6,
                b: 0.9,
                a: 1.0,
            },
            selected: [false, true, false, false],
            popup: false,
            text,
        }
    }
}

fn ui(ctx: &mut Context<'_>, d: &mut Demo, size: (f32, f32)) {
    d.time += 0.05;
    let flags = PanelFlags::BORDER
        | PanelFlags::MOVABLE
        | PanelFlags::SCALABLE
        | PanelFlags::MINIMIZABLE
        | PanelFlags::TITLE;
    let bounds = Rect {
        x: 20.0,
        y: 20.0,
        w: 380.0,
        h: (size.1 - 40.0).clamp(200.0, 620.0),
    };
    ctx.window("Overview", bounds, flags, |w| {
        menubar(w, d);
        w.tree(TreeType::Tab, "Widgets", CollapseState::Maximized, 0, |w| {
            basic(w, d)
        });
        w.tree(
            TreeType::Tab,
            "Popup and contextual menu",
            CollapseState::Minimized,
            1,
            |w| popups(w, d),
        );
        w.tree(TreeType::Tab, "Layout", CollapseState::Minimized, 2, |w| {
            layouts(w, d)
        });
        w.tree(TreeType::Tab, "Charts", CollapseState::Minimized, 3, |w| {
            charts(w, d)
        });
        w.tree(TreeType::Tab, "Input", CollapseState::Minimized, 4, |w| {
            input(w, d)
        });
        w.layout_row_dynamic(20.0, 1);
        w.label(&format!("Status: {}", d.status), TextAlign::LEFT);
    });
    ctx.window(
        "Info",
        Rect {
            x: 420.0,
            y: 20.0,
            w: 240.0,
            h: 150.0,
        },
        flags,
        |w| {
            w.layout_row_dynamic(22.0, 1);
            w.label("A window you can drag.", TextAlign::LEFT);
            w.label_colored(
                "Colored text",
                TextAlign::LEFT,
                Color {
                    r: 255,
                    g: 200,
                    b: 60,
                    a: 255,
                },
            );
            w.label(&format!("Slider: {:.2}", d.slider), TextAlign::LEFT);
            w.label(&format!("Checkbox: {}", d.checked), TextAlign::LEFT);
        },
    );
}

fn menubar(w: &mut Window<'_, '_>, d: &mut Demo) {
    w.menubar(|m| {
        m.layout_row_begin(LayoutFormat::Static, 25.0, 2);
        m.layout_row_push(45.0);
        m.menu_label(
            "File",
            TextAlign::LEFT,
            Vec2 { x: 120.0, y: 100.0 },
            |menu| {
                menu.layout_row_dynamic(25.0, 1);
                if menu.item_label("New", TextAlign::LEFT) {
                    d.status = "File > New".into();
                }
                if menu.item_label("Open", TextAlign::LEFT) {
                    d.status = "File > Open".into();
                }
                if menu.item_label("Quit", TextAlign::LEFT) {
                    d.quit = true;
                }
            },
        );
        m.layout_row_push(45.0);
        m.menu_label(
            "Edit",
            TextAlign::LEFT,
            Vec2 { x: 120.0, y: 100.0 },
            |menu| {
                menu.layout_row_dynamic(25.0, 1);
                for name in ["Copy", "Cut", "Paste"] {
                    if menu.item_label(name, TextAlign::LEFT) {
                        d.status = format!("Edit > {name}");
                    }
                }
            },
        );
        m.layout_row_end();
    });
}

fn basic(w: &mut Window<'_, '_>, d: &mut Demo) {
    w.layout_row_static(30.0, 80, 1);
    if w.button_label("button") {
        d.status = "Button pressed".into();
    }
    w.layout_row_dynamic(30.0, 2);
    w.checkbox("check", &mut d.checked);
    w.label(&format!("{:.2}", d.slider), TextAlign::RIGHT);
    for (i, name) in ["easy", "hard", "insane"].iter().enumerate() {
        if i % 2 == 0 {
            w.layout_row_dynamic(30.0, 2);
        }
        if w.option(name, d.option == i) {
            d.option = i;
        }
    }
    w.layout_row_dynamic(25.0, 1);
    w.slider_f32(0.0, &mut d.slider, 1.0, 0.01);
    w.progress(&mut d.progress, 100, true);
    w.property_i32("Compression:", 0, &mut d.property, 100, 10, 1.0);
    w.layout_row_dynamic(25.0, 1);
    w.combo(
        &["Item 1", "Item 2", "Item 3"],
        &mut d.combo,
        25,
        Vec2 { x: 200.0, y: 200.0 },
    );
    let c = d.color;
    let swatch = Color {
        r: (c.r * 255.0) as u8,
        g: (c.g * 255.0) as u8,
        b: (c.b * 255.0) as u8,
        a: (c.a * 255.0) as u8,
    };
    let width = w.widget_bounds().w;
    w.combo_color(swatch, Vec2 { x: width, y: 300.0 }, |c| {
        c.layout_row_dynamic(120.0, 1);
        c.color_pick(&mut d.color, ColorFormat::Rgba);
    });
    w.layout_row_dynamic(25.0, 2);
    for (i, sel) in d.selected.iter_mut().enumerate() {
        w.selectable(&format!("Selectable {i}"), TextAlign::LEFT, sel);
    }
    w.layout_row_dynamic(25.0, 1);
    w.label("Hover this label.", TextAlign::LEFT);
    if w.widget_is_hovered() {
        w.tooltip("This is a tooltip");
    }
}

fn popups(w: &mut Window<'_, '_>, d: &mut Demo) {
    w.layout_row_static(30.0, 100, 1);
    if w.button_label("Open popup") {
        d.popup = true;
    }
    if d.popup {
        let flags = PanelFlags::CLOSABLE | PanelFlags::MOVABLE | PanelFlags::TITLE;
        let open = w.popup(
            PopupType::Static,
            "Popup",
            flags,
            Rect {
                x: 20.0,
                y: 20.0,
                w: 220.0,
                h: 90.0,
            },
            |p| {
                p.layout_row_dynamic(25.0, 1);
                p.label("A popup window.", TextAlign::LEFT);
                p.layout_row_dynamic(25.0, 2);
                if p.button_label("OK") {
                    d.popup = false;
                    p.close();
                }
                if p.button_label("Cancel") {
                    d.popup = false;
                    p.close();
                }
            },
        );
        // The user closed the popup with its title bar button.
        d.popup &= open.is_some();
    }
    w.layout_row_dynamic(80.0, 1);
    let area = w.widget_bounds();
    w.label("Right click here for a menu.", TextAlign::LEFT);
    w.contextual(
        PanelFlags::empty(),
        Vec2 { x: 120.0, y: 100.0 },
        area,
        |c| {
            c.layout_row_dynamic(25.0, 1);
            for name in ["Open", "Save", "Close"] {
                if c.item_label(name, TextAlign::LEFT) {
                    d.status = format!("Context menu: {name}");
                }
            }
        },
    );
}

fn layouts(w: &mut Window<'_, '_>, _d: &mut Demo) {
    w.layout_row_dynamic(25.0, 3);
    for name in ["dynamic 1", "dynamic 2", "dynamic 3"] {
        w.button_label(name);
    }
    w.layout_row(LayoutFormat::Dynamic, 25.0, &[0.2, 0.6, 0.2]);
    for name in ["20%", "60%", "20%"] {
        w.button_label(name);
    }
    w.layout_row_template_begin(25.0);
    w.layout_row_template_push_static(60.0);
    w.layout_row_template_push_variable(80.0);
    w.layout_row_template_push_dynamic();
    w.layout_row_template_end();
    for name in ["static", "variable", "dynamic"] {
        w.button_label(name);
    }
    w.layout_row_dynamic(110.0, 1);
    w.group("Group", PanelFlags::BORDER | PanelFlags::TITLE, |g| {
        g.layout_row_dynamic(22.0, 1);
        for i in 0..6 {
            g.label(&format!("Group item {i}"), TextAlign::LEFT);
        }
    });
}

fn charts(w: &mut Window<'_, '_>, d: &mut Demo) {
    w.layout_row_dynamic(100.0, 1);
    w.chart(ChartType::Lines, 40, -1.0, 1.0, |c| {
        for i in 0..40 {
            c.push((i as f32 * 0.25 + d.time).sin());
        }
    });
    let columns: Vec<f32> = (0..16)
        .map(|i| ((i as f32 * 0.5 + d.time).cos() + 1.0) * 5.0)
        .collect();
    w.plot(ChartType::Column, &columns, 0);
}

fn input(w: &mut Window<'_, '_>, d: &mut Demo) {
    w.layout_row_dynamic(30.0, 1);
    w.edit(EditFlags::FIELD, &mut d.text, Filter::Default);
    w.label(&format!("Text: {}", d.text.text()), TextAlign::LEFT);
}

// ---------------------------------------------------------------------------------------------
// The CPU rasterizer
// ---------------------------------------------------------------------------------------------

/// The font atlas image with one byte of alpha for each pixel.
struct Atlas {
    w: usize,
    h: usize,
    alpha: Vec<u8>,
}

/// One vertex as x, y, u, v, r, g, b, a.
type Vertex = [f32; 8];

fn vertex(bytes: &[u8]) -> Vertex {
    let f = |o: usize| f32::from_ne_bytes(bytes[o..o + 4].try_into().unwrap());
    [
        f(0),
        f(4),
        f(8),
        f(12),
        bytes[16] as f32,
        bytes[17] as f32,
        bytes[18] as f32,
        bytes[19] as f32,
    ]
}

/// Fill a triangle inside `clip` (x0, y0, x1, y1) and blend it over the pixels of `fb`.
fn triangle(fb: &mut [u32], width: usize, clip: [usize; 4], atlas: &Atlas, [a, b, c]: [Vertex; 3]) {
    let area = (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);
    if area == 0.0 || !area.is_finite() {
        return;
    }
    let min = |i: usize| a[i].min(b[i]).min(c[i]);
    let max = |i: usize| a[i].max(b[i]).max(c[i]);
    let (x0, x1) = (
        (min(0).floor() as usize).max(clip[0]),
        (max(0).ceil() as usize).min(clip[2]),
    );
    let (y0, y1) = (
        (min(1).floor() as usize).max(clip[1]),
        (max(1).ceil() as usize).min(clip[3]),
    );
    for y in y0..y1 {
        for x in x0..x1 {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let wa = ((b[0] - px) * (c[1] - py) - (b[1] - py) * (c[0] - px)) / area;
            let wb = ((c[0] - px) * (a[1] - py) - (c[1] - py) * (a[0] - px)) / area;
            let wc = 1.0 - wa - wb;
            if wa < 0.0 || wb < 0.0 || wc < 0.0 {
                continue;
            }
            let at = |i: usize| wa * a[i] + wb * b[i] + wc * c[i];
            let tx = ((at(2) * atlas.w as f32) as usize).min(atlas.w - 1);
            let ty = ((at(3) * atlas.h as f32) as usize).min(atlas.h - 1);
            let alpha = at(7) / 255.0 * atlas.alpha[ty * atlas.w + tx] as f32 / 255.0;
            let dst = &mut fb[y * width + x];
            let mut out = 0u32;
            for (shift, i) in [(16, 4), (8, 5), (0, 6)] {
                let old = ((*dst >> shift) & 255) as f32;
                out |= ((at(i) * alpha + old * (1.0 - alpha)) as u32).min(255) << shift;
            }
            *dst = out;
        }
    }
}

fn raster(
    fb: &mut [u32],
    (w, h): (usize, usize),
    atlas: &Atlas,
    vertices: &[u8],
    indices: &[u16],
    draws: &[DrawCommand],
) {
    let mut at = 0;
    for d in draws {
        let n = d.elem_count as usize;
        let r = d.clip_rect;
        let clip = [
            r.x.floor() as usize,
            r.y.floor() as usize,
            ((r.x + r.w).ceil() as usize).min(w),
            ((r.y + r.h).ceil() as usize).min(h),
        ];
        for t in indices[at..at + n].chunks_exact(3) {
            let v = |i: u16| vertex(&vertices[i as usize * VERTEX_SIZE..][..VERTEX_SIZE]);
            triangle(fb, w, clip, atlas, [v(t[0]), v(t[1]), v(t[2])]);
        }
        at += n;
    }
}

// ---------------------------------------------------------------------------------------------
// The window loop
// ---------------------------------------------------------------------------------------------

/// An input event that waits for the start of the next frame.
enum Event {
    Motion(i32, i32),
    Button(Button, i32, i32, bool),
    Scroll(Vec2),
    Key(Key, bool),
    Text(String),
}

struct App<'f> {
    ctx: Context<'f>,
    demo: Demo,
    atlas: Atlas,
    layout: VertexLayout,
    null: NullTexture,
    buffers: [Buffer; 3],
    events: Vec<Event>,
    mouse: (i32, i32),
    modifiers: ModifiersState,
    window: Option<Rc<Win>>,
    surface: Option<Surface<Rc<Win>, Rc<Win>>>,
}

impl App<'_> {
    fn frame(&mut self, event_loop: &ActiveEventLoop) {
        let (Some(window), Some(surface)) = (&self.window, &mut self.surface) else {
            return;
        };
        let size = window.inner_size();
        let (Some(w), Some(h)) = (NonZeroU32::new(size.width), NonZeroU32::new(size.height)) else {
            return;
        };
        surface.resize(w, h).expect("cannot resize the surface");

        let events = std::mem::take(&mut self.events);
        self.ctx.input(|i| {
            for e in events {
                match e {
                    Event::Motion(x, y) => i.motion(x, y),
                    Event::Button(b, x, y, down) => i.button(b, x, y, down),
                    Event::Scroll(delta) => i.scroll(delta),
                    Event::Key(k, down) => i.key(k, down),
                    Event::Text(s) => i.text(&s),
                }
            }
        });
        ui(
            &mut self.ctx,
            &mut self.demo,
            (size.width as f32, size.height as f32),
        );

        let [cmds, vertices, elements] = &mut self.buffers;
        let config = ConvertConfig::new(&self.layout, self.null);
        let draws: Vec<DrawCommand> = self
            .ctx
            .convert(cmds, vertices, elements, &config)
            .expect("cannot convert the frame")
            .collect();
        let mut buffer = surface.buffer_mut().expect("cannot get the pixel buffer");
        let [r, g, b] = BACKGROUND.map(|c| c as u32);
        buffer.fill(r << 16 | g << 8 | b);
        raster(
            &mut buffer,
            (size.width as usize, size.height as usize),
            &self.atlas,
            vertices.as_bytes(),
            elements.as_indices(),
            &draws,
        );
        buffer.present().expect("cannot show the frame");

        self.ctx.clear();
        cmds.clear();
        vertices.clear();
        elements.clear();
        if self.demo.quit {
            event_loop.exit();
        }
    }

    fn on_key(&mut self, key: &WKey, down: bool) {
        let mapped = match key {
            WKey::Named(NamedKey::Backspace) => Key::Backspace,
            WKey::Named(NamedKey::Delete) => Key::Del,
            WKey::Named(NamedKey::Enter) => Key::Enter,
            WKey::Named(NamedKey::Tab) => Key::Tab,
            WKey::Named(NamedKey::ArrowUp) => Key::Up,
            WKey::Named(NamedKey::ArrowDown) => Key::Down,
            WKey::Named(NamedKey::ArrowLeft) => Key::Left,
            WKey::Named(NamedKey::ArrowRight) => Key::Right,
            WKey::Named(NamedKey::Home) => Key::TextLineStart,
            WKey::Named(NamedKey::End) => Key::TextLineEnd,
            _ => return,
        };
        self.events.push(Event::Key(mapped, down));
    }
}

impl ApplicationHandler for App<'_> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attributes = Win::default_attributes()
            .with_title("nuklear-rs demo")
            .with_inner_size(PhysicalSize::new(900, 680));
        let window = Rc::new(
            event_loop
                .create_window(attributes)
                .expect("cannot make the window"),
        );
        let context = SbContext::new(window.clone()).expect("cannot make the display context");
        self.surface =
            Some(Surface::new(&context, window.clone()).expect("cannot make the surface"));
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => self.frame(event_loop),
            WindowEvent::ModifiersChanged(m) => {
                self.modifiers = m.state();
                self.events
                    .push(Event::Key(Key::Shift, self.modifiers.shift_key()));
                self.events
                    .push(Event::Key(Key::Ctrl, self.modifiers.control_key()));
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse = (position.x as i32, position.y as i32);
                self.events.push(Event::Motion(self.mouse.0, self.mouse.1));
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let button = match button {
                    MouseButton::Left => Button::Left,
                    MouseButton::Middle => Button::Middle,
                    MouseButton::Right => Button::Right,
                    _ => return,
                };
                self.events.push(Event::Button(
                    button,
                    self.mouse.0,
                    self.mouse.1,
                    state == ElementState::Pressed,
                ));
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(x, y) => Vec2 { x, y },
                    MouseScrollDelta::PixelDelta(p) => Vec2 {
                        x: p.x as f32 / 20.0,
                        y: p.y as f32 / 20.0,
                    },
                };
                self.events.push(Event::Scroll(delta));
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let down = event.state == ElementState::Pressed;
                self.on_key(&event.logical_key, down);
                if let (true, Some(text)) = (down, &event.text) {
                    let text: String = text.chars().filter(|c| !c.is_control()).collect();
                    if !text.is_empty() {
                        self.events.push(Event::Text(text));
                    }
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // Draw at about 60 frames per second. Nuklear needs more than one frame to react to some input.
        event_loop.set_control_flow(ControlFlow::wait_duration(Duration::from_millis(16)));
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    let mut builder = FontAtlasBuilder::new();
    let font_id = builder
        .add_default(13.0)
        .expect("cannot add the default font");
    let baked = builder
        .bake(AtlasFormat::Alpha8)
        .expect("cannot bake the font atlas");
    let atlas = Atlas {
        w: baked.width() as usize,
        h: baked.height() as usize,
        alpha: baked.pixels().to_vec(),
    };
    let (font_atlas, null) = baked.end(Handle::id(1));
    let font = font_atlas.font(font_id).expect("the font is in the atlas");
    let ctx = Context::new(&font).expect("cannot make the context");

    let layout = VertexLayout::new(
        VERTEX_SIZE,
        4,
        &[
            (VertexAttribute::Position, VertexFormat::Float, 0),
            (VertexAttribute::Texcoord, VertexFormat::Float, 8),
            (VertexAttribute::Color, VertexFormat::R8G8B8A8, 16),
        ],
    )
    .expect("the vertex layout is valid");

    let mut app = App {
        ctx,
        demo: Demo::new(),
        atlas,
        layout,
        null,
        buffers: [Buffer::new(), Buffer::new(), Buffer::new()],
        events: Vec::new(),
        mouse: (0, 0),
        modifiers: ModifiersState::default(),
        window: None,
        surface: None,
    };
    let event_loop = EventLoop::new().expect("cannot make the event loop");
    event_loop.run_app(&mut app).expect("the event loop failed");
}
