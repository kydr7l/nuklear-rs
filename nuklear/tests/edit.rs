mod common;
use nuklear::*;
const WIN: Rect = Rect { x: 0.0, y: 0.0, w: 300.0, h: 300.0 };

#[test] fn set_and_get_text() {
    let mut e = TextEdit::new();
    assert_eq!(e.text(), "");
    e.set_text("héllo");
    assert_eq!(e.text(), "héllo");
}
#[test] fn typed_text_lands_in_edit() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let mut e = TextEdit::new();
    let b = ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); let b = w.widget_bounds(); w.edit(EditFlags::FIELD, &mut e, Filter::Default); b }).unwrap();
    ctx.clear();
    let (x, y) = ((b.x + b.w / 2.0) as i32, (b.y + b.h / 2.0) as i32);
    ctx.input(|i| { i.motion(x, y); i.button(Button::Left, x, y, true); });
    let ev = ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); w.edit(EditFlags::FIELD, &mut e, Filter::Default) }).unwrap();
    assert!(ev.contains(EditEvents::ACTIVE));
    ctx.clear();
    ctx.input(|i| { i.button(Button::Left, x, y, false); i.text("hi"); });
    ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); w.edit(EditFlags::FIELD, &mut e, Filter::Default) });
    ctx.clear();
    assert_eq!(e.text(), "hi");
}
#[test] fn decimal_filter_drops_letters() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let mut e = TextEdit::new();
    let b = ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); let b = w.widget_bounds(); w.edit(EditFlags::FIELD, &mut e, Filter::Decimal); b }).unwrap();
    ctx.clear();
    let (x, y) = ((b.x + b.w / 2.0) as i32, (b.y + b.h / 2.0) as i32);
    ctx.input(|i| { i.motion(x, y); i.button(Button::Left, x, y, true); });
    ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); w.edit(EditFlags::FIELD, &mut e, Filter::Decimal) });
    ctx.clear();
    ctx.input(|i| { i.button(Button::Left, x, y, false); i.text("4a2"); });
    ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); w.edit(EditFlags::FIELD, &mut e, Filter::Decimal) });
    ctx.clear();
    assert_eq!(e.text(), "42");
}
// Beyond the brief: set_text while active must not leave a cursor or selection past the new text.
#[test] fn set_text_while_active_then_draw() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let mut e = TextEdit::new();
    e.set_text("a long first text");
    let b = ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); let b = w.widget_bounds(); w.edit(EditFlags::FIELD, &mut e, Filter::Default); b }).unwrap();
    ctx.clear();
    let (x, y) = ((b.x + b.w / 2.0) as i32, (b.y + b.h / 2.0) as i32);
    ctx.input(|i| { i.motion(x, y); i.button(Button::Left, x, y, true); });
    ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); w.edit(EditFlags::FIELD, &mut e, Filter::Default) });
    ctx.clear();
    e.select_all();
    assert!(e.is_active());
    e.set_text("é");
    for _ in 0..2 {
        ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); w.edit(EditFlags::FIELD, &mut e, Filter::Default) });
        ctx.clear();
    }
    assert_eq!(e.text(), "é");
}
// One TextEdit in two edit widgets of one frame.
#[test] fn one_edit_in_two_widgets() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let mut e = TextEdit::new();
    e.set_text("shared");
    for _ in 0..2 {
        ctx.window("w", WIN, PanelFlags::empty(), |w| {
            w.layout_row_dynamic(30.0, 1);
            w.edit(EditFlags::FIELD, &mut e, Filter::Default);
            w.edit(EditFlags::FIELD, &mut e, Filter::Default);
        });
        ctx.clear();
    }
    assert_eq!(e.text(), "shared");
}
#[test] fn every_filter_runs() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let mut e = TextEdit::new();
    for f in [Filter::Default, Filter::Ascii, Filter::Float, Filter::Decimal, Filter::Hex, Filter::Oct, Filter::Binary] {
        ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); w.edit(EditFlags::FIELD, &mut e, f) });
        ctx.clear();
    }
}
