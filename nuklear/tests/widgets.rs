mod common;
use nuklear::*;
const WIN: Rect = Rect { x: 0.0, y: 0.0, w: 300.0, h: 300.0 };

fn click(ctx: &mut Context<'_>, at: Rect) {
    let (x, y) = ((at.x + at.w / 2.0) as i32, (at.y + at.h / 2.0) as i32);
    ctx.input(|i| { i.motion(x, y); i.button(Button::Left, x, y, true); });
}

#[test] fn button_reports_click() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let bounds = ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); let b = w.widget_bounds(); assert!(!w.button_label("ok")); b }).unwrap();
    ctx.clear();
    click(&mut ctx, bounds);
    let clicked = ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); w.button_label("ok") });
    assert_eq!(clicked, Some(true));
    ctx.clear();
}
#[test] fn checkbox_toggles_on_click() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let mut on = false;
    let bounds = ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); let b = w.widget_bounds(); w.checkbox("c", &mut on); b }).unwrap();
    ctx.clear();
    click(&mut ctx, bounds);
    let changed = ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); w.checkbox("c", &mut on) });
    assert_eq!(changed, Some(true));
    assert!(on);
    ctx.clear();
}
#[test] fn slider_and_property_keep_values_without_input() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let (mut s, mut p, mut prog) = (0.5f32, 3i32, 2usize);
    ctx.window("w", WIN, PanelFlags::empty(), |w| {
        w.layout_row_dynamic(30.0, 1);
        assert!(!w.slider_f32(0.0, &mut s, 1.0, 0.1));
        assert!(!w.property_i32("p", 0, &mut p, 10, 1, 1.0));
        assert!(!w.progress(&mut prog, 10, true));
        // `option` returns the active state, not "changed".
        assert!(w.option("o", true));
    });
    assert_eq!((s, p, prog), (0.5, 3, 2));
    ctx.clear();
}
#[test] fn empty_inputs_are_no_ops() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    ctx.window("w", WIN, PanelFlags::empty(), |w| {
        w.layout_row(LayoutFormat::Dynamic, 30.0, &[]);
        w.label("", TextAlign::LEFT);
        w.plot(ChartType::Lines, &[], 0);
        w.plot(ChartType::Lines, &[1.0], 5);
        w.layout_row_template_begin(30.0);
        for _ in 0..40 { w.layout_row_template_push_static(10.0); }
        w.layout_row_template_end();
        w.layout_row_push(1.0);
        w.layout_row_end();
        w.layout_space_push(Rect::default());
        w.layout_space_end();
        w.disabled(|w| w.label("off", TextAlign::LEFT));
    });
    ctx.clear();
}
#[test] fn bad_numeric_parameters_are_rejected() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let (mut s, mut i, mut prog, mut p) = (0.5f32, 3i32, 2usize, 1.0f32);
    ctx.window("w", WIN, PanelFlags::empty(), |w| {
        w.layout_row_dynamic(30.0, 1);
        assert!(!w.slider_f32(0.0, &mut s, 1.0, 0.0));
        assert!(!w.slider_f32(1.0, &mut s, 0.0, 0.1));
        assert!(!w.slider_f32(0.0, &mut s, f32::INFINITY, 0.1));
        assert!(!w.slider_i32(0, &mut i, 10, 0));
        assert!(!w.progress(&mut prog, 0, true));
        assert!(!w.property_f32("p", 0.0, &mut p, 1.0, f32::NAN, 1.0));
        assert!(!w.property_i32("q", 5, &mut i, 1, 1, 1.0));
        assert!(!w.knob_i32(0, &mut i, 10, 1, Heading::Up, f32::NAN));
    });
    assert_eq!((s, i, prog, p), (0.5, 3, 2, 1.0));
    ctx.clear();
}
// The ratio array must outlive `layout_row`, because later widgets read it.
#[test] fn layout_row_ratios_survive_the_call() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    for _ in 0..2 {
        ctx.window("w", WIN, PanelFlags::empty(), |w| {
            { let r = vec![0.25f32, 0.75]; w.layout_row(LayoutFormat::Dynamic, 30.0, &r); }
            let a = w.widget_bounds();
            w.label("a", TextAlign::LEFT);
            let b = w.widget_bounds();
            w.label("b", TextAlign::LEFT);
            assert!(b.x > a.x && b.w > a.w);
            w.layout_row(LayoutFormat::Static, 30.0, &[40.0, 50.0]);
            w.label("c", TextAlign::LEFT);
            w.label("d", TextAlign::LEFT);
        });
        ctx.clear();
    }
}
#[test] fn spacing_and_spacer_are_guarded() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    ctx.window("w", WIN, PanelFlags::empty(), |w| {
        w.layout_row_dynamic(30.0, 0);
        w.spacer();
        w.spacing(2);
        w.layout_row_dynamic(30.0, 3);
        w.spacing(-1);
        w.spacing(1);
        w.spacing(i32::MAX); // The index is 1, so the sum overflows and the guard skips the call.
        w.spacer();
    });
    ctx.clear();
}
// Task 7 must add the popup-sequence regression test: Static popup "p", then `tooltip`, then a second `popup`.
#[test] fn tooltip_runs_after_a_row() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    for _ in 0..2 {
        ctx.window("w", WIN, PanelFlags::empty(), |w| {
            w.layout_row_dynamic(30.0, 1);
            w.label("x", TextAlign::LEFT);
            w.tooltip("t");
            w.layout_row_dynamic(30.0, 1);
            w.label("y", TextAlign::LEFT);
        });
        ctx.clear();
    }
}
#[test] fn space_layout_and_queries() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    ctx.window("w", WIN, PanelFlags::empty(), |w| {
        w.layout_space_begin(LayoutFormat::Static, 100.0, 1);
        w.layout_space_push(Rect { x: 0.0, y: 0.0, w: 50.0, h: 20.0 });
        w.label("s", TextAlign::LEFT);
        let v = w.layout_space_to_local(w.layout_space_to_screen(Vec2 { x: 3.0, y: 4.0 }));
        assert_eq!(v, Vec2 { x: 3.0, y: 4.0 });
        w.layout_space_end();
        w.layout_set_min_row_height(40.0);
        w.layout_row_dynamic(0.0, 1);
        assert!(w.layout_widget_bounds().h >= 40.0);
        w.layout_reset_min_row_height();
        assert!(!w.widget_is_hovered());
        assert!(!w.widget_is_mouse_clicked(Button::Left));
        assert!(!w.widget_has_mouse_click_down(Button::Left, true));
        assert!((0.0..=1.0).contains(&w.layout_ratio_from_pixel(10.0)));
    });
    ctx.clear();
}
#[test] fn widgets_smoke() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let img = Image::new(Handle::id(1), 8, 8);
    let (mut b, mut flags, mut f, mut i, mut d) = (false, 0u32, 0.5f32, 3i32, 2.0f64);
    let mut c = Colorf { r: 0.1, g: 0.2, b: 0.3, a: 1.0 };
    ctx.window("w", WIN, PanelFlags::empty(), |w| {
        w.layout_row_dynamic(30.0, 1);
        w.label_colored("é", TextAlign::CENTERED, Color { r: 1, g: 2, b: 3, a: 4 });
        w.label_wrap("wrap me");
        w.label_wrap_colored("wrap", Color::default());
        w.image(img);
        w.image_color(img, Color::default());
        w.button_set_behavior(ButtonBehavior::Repeater);
        w.button_color(Color::default());
        w.button_symbol(Symbol::Plus);
        w.button_image(img);
        w.button_symbol_label(Symbol::Plus, "s", TextAlign::LEFT);
        w.button_image_label(img, "i", TextAlign::LEFT);
        w.checkbox_flags("cf", &mut flags, 1);
        w.radio("r", &mut b);
        w.selectable("sel", TextAlign::LEFT, &mut b);
        w.selectable_image(img, "si", TextAlign::LEFT, &mut b);
        w.selectable_symbol(Symbol::Plus, "ss", TextAlign::LEFT, &mut b);
        w.knob_f32(0.0, &mut f, 1.0, 0.1, Heading::Up, 0.0);
        w.knob_i32(0, &mut i, 10, 1, Heading::Up, 0.0);
        w.color_pick(&mut c, ColorFormat::Rgba);
        w.property_f32("pf", 0.0, &mut f, 1.0, 0.1, 1.0);
        w.property_f64("pd", 0.0, &mut d, 10.0, 0.1, 1.0);
        w.plot(ChartType::Column, &[1.0, 2.0, 3.0], 1);
    });
    ctx.clear();
}
