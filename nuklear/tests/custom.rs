mod common;
use nuklear::*;
const WIN: Rect = Rect { x: 0.0, y: 0.0, w: 300.0, h: 300.0 };
const RED: Color = Color { r: 255, g: 0, b: 0, a: 255 };

fn click(ctx: &mut Context<'_>, at: Rect, down: bool) {
    let (x, y) = ((at.x + at.w / 2.0) as i32, (at.y + at.h / 2.0) as i32);
    ctx.input(|i| { i.motion(x, y); i.button(Button::Left, x, y, down); });
}

#[test] fn canvas_records_shapes_in_order() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    ctx.window("w", WIN, PanelFlags::empty(), |w| {
        let mut c = w.canvas();
        c.fill_rect(Rect { x: 10.0, y: 20.0, w: 30.0, h: 40.0 }, 2.0, RED);
        c.stroke_line(Vec2 { x: 1.0, y: 2.0 }, Vec2 { x: 3.0, y: 4.0 }, 1.0, RED);
        c.fill_rect_multi_color(Rect { x: 5.0, y: 5.0, w: 10.0, h: 10.0 }, RED, RED, RED, RED);
        c.fill_circle(Rect { x: 50.0, y: 50.0, w: 8.0, h: 8.0 }, RED);
        c.draw_text(Rect { x: 0.0, y: 0.0, w: 100.0, h: 12.0 }, "hi", Color::default(), RED);
    });
    let cmds: Vec<String> = ctx.commands().map(|c| format!("{c:?}")).collect();
    let pos = |s: &str| cmds.iter().position(|c| c.starts_with(s)).unwrap_or_else(|| panic!("no {s} in {cmds:?}"));
    assert!(cmds.iter().any(|c| c == &format!("{:?}", Command::RectFilled { rounding: 2, x: 10, y: 20, w: 30, h: 40, color: RED })));
    assert!(pos("RectFilled { rounding: 2") < pos("Line") && pos("Line") < pos("RectMultiColor") && pos("RectMultiColor") < pos("CircleFilled") && pos("CircleFilled") < pos("Text"));
    assert!(cmds.iter().any(|c| c.contains("text: \"hi\"")));
    ctx.clear();
}

#[test] fn canvas_skips_numbers_that_do_not_fit() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let count = |ctx: &mut Context<'_>| ctx.commands().count();
    ctx.window("w", WIN, PanelFlags::empty(), |_| {});
    let base = count(&mut ctx);
    ctx.clear();
    ctx.window("w", WIN, PanelFlags::empty(), |w| {
        let mut c = w.canvas();
        c.fill_rect(Rect { x: f32::NAN, y: 0.0, w: 10.0, h: 10.0 }, 0.0, RED);
        c.fill_rect(Rect { x: 0.0, y: 0.0, w: 10.0, h: 10.0 }, -1.0, RED);
        c.stroke_line(Vec2 { x: 1e9, y: 0.0 }, Vec2::default(), 1.0, RED);
        c.push_scissor(Rect { x: 0.0, y: 0.0, w: 70000.0, h: 1.0 });
        c.draw_text(Rect { x: 0.0, y: 0.0, w: -5.0, h: 12.0 }, "x", Color::default(), RED);
        c.fill_polygon(&vec![Vec2::default(); 70000], RED);
    });
    assert_eq!(count(&mut ctx), base);
    ctx.clear();
}

#[test] fn widget_slot_and_input_state() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let (state, bounds) = ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); w.widget() }).unwrap();
    // The mouse is not over the slot yet, so the slot is visible but read only.
    assert_eq!(state, WidgetState::Rom);
    assert!(bounds.w > 0.0 && bounds.h == 30.0);
    ctx.clear();
    click(&mut ctx, bounds, true);
    assert!(ctx.input_state().is_mouse_down(Button::Left));
    assert!(ctx.input_state().is_mouse_pressed(Button::Left));
    assert!(ctx.input_state().has_mouse_click_down_in_rect(Button::Left, bounds, true));
    assert_eq!(ctx.input_state().mouse_pos(), Vec2 { x: (bounds.x + bounds.w / 2.0) as i32 as f32, y: (bounds.y + bounds.h / 2.0) as i32 as f32 });
    let hovered = ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); assert!(w.input_state().is_mouse_hovering_rect(bounds)); w.widget().0 });
    assert_eq!(hovered, Some(WidgetState::Valid));
    ctx.clear();
    click(&mut ctx, bounds, false);
    assert!(ctx.input_state().mouse_clicked(Button::Left, bounds));
    assert!(ctx.input_state().is_mouse_released(Button::Left));
    ctx.clear();
}

#[test] fn style_changes_and_restores() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let saved = *ctx.style().window();
    ctx.style_mut().window_mut().set_padding(Vec2 { x: 20.0, y: 0.0 });
    ctx.style_mut().combo_mut().button_mut().set_rounding(3.0);
    assert_eq!(ctx.style().window().padding(), Vec2 { x: 20.0, y: 0.0 });
    assert_eq!(ctx.style().combo().button().rounding(), 3.0);
    let x = ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); w.widget_bounds().x }).unwrap();
    assert_eq!(x, 20.0);
    ctx.clear();
    *ctx.style_mut().window_mut() = saved;
    assert_eq!(ctx.style().window().padding(), saved.padding());
}

#[test] fn styled_button_uses_its_style() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    ctx.window("w", WIN, PanelFlags::empty(), |w| {
        w.layout_row_dynamic(30.0, 1);
        let mut style = *w.style().button();
        style.set_normal(StyleItem::color(RED));
        assert_eq!(style.normal().as_color(), Some(RED));
        assert!(!w.button_label_styled(&style, "go"));
        w.rule_horizontal(RED, false);
    });
    let red_fills = ctx.commands().filter(|c| matches!(c, Command::RectFilled { color, .. } if *color == RED)).count();
    assert_eq!(red_fills, 2);
    ctx.clear();
}

#[test] fn edit_string_types_and_keeps_max() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let mut text = String::from("ab");
    let bounds = ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); let b = w.widget_bounds(); w.edit_string(EditFlags::FIELD, &mut text, 4, Filter::Default); b }).unwrap();
    ctx.clear();
    click(&mut ctx, bounds, true);
    ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); w.edit_string(EditFlags::FIELD, &mut text, 4, Filter::Default) });
    ctx.clear();
    ctx.input(|i| { i.button(Button::Left, 0, 0, false); i.text("cdef"); });
    let ev = ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); w.edit_string(EditFlags::FIELD, &mut text, 4, Filter::Default) }).unwrap();
    ctx.clear();
    assert!(ev.contains(EditEvents::ACTIVE));
    assert_eq!(text, "abcd");
    // A text longer than `max` gets cut at a character boundary.
    let mut long = String::from("aé");
    ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); w.edit_string(EditFlags::FIELD | EditFlags::READ_ONLY, &mut long, 2, Filter::Default) });
    ctx.clear();
    assert_eq!(long, "a");
}

#[test] fn text_metrics_use_the_font() {
    let font = UserFont::new(common::FixedFont);
    let ctx = Context::new(&font).unwrap();
    assert_eq!(ctx.text_width("abc"), 18.0);
    assert_eq!(ctx.text_width(""), 0.0);
    assert_eq!(ctx.font_height(), 12.0);
}

#[test] fn layout_clip_is_the_panel_clip() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let clip = ctx.window("w", WIN, PanelFlags::empty(), |w| w.layout_clip()).unwrap();
    assert!(clip.w > 0.0 && clip.w <= WIN.w && clip.h > 0.0);
    ctx.clear();
}
