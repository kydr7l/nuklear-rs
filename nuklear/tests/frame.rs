mod common;
use nuklear::*;

#[test] fn window_body_runs_and_returns_value() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let r = ctx.window("w", Rect { x: 0.0, y: 0.0, w: 200.0, h: 200.0 }, PanelFlags::TITLE, |w| { assert!(w.size().x > 0.0); 42 });
    assert_eq!(r, Some(42));
    ctx.clear();
}
#[test] fn closed_window_body_does_not_run() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    ctx.window("w", Rect { x: 0.0, y: 0.0, w: 200.0, h: 200.0 }, PanelFlags::CLOSABLE, |_| ());
    ctx.clear();
    ctx.window_close("w");
    let r = ctx.window("w", Rect { x: 0.0, y: 0.0, w: 200.0, h: 200.0 }, PanelFlags::CLOSABLE, |_| 1);
    assert_eq!(r, None);
    assert!(ctx.window_is_closed("w"));
    ctx.clear();
}
#[test] fn strings_with_nul_do_not_panic() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    assert!(ctx.window("a\0b", Rect { x: 0.0, y: 0.0, w: 50.0, h: 50.0 }, PanelFlags::empty(), |_| ()).is_some());
    assert!(!ctx.window_is_closed("a\0b"));
    ctx.clear();
}
#[test] fn input_scope_moves_mouse() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    ctx.input(|i| { i.motion(10, 10); i.button(Button::Left, 10, 10, true); i.text("hi"); i.key(Key::Enter, true); i.scroll(Vec2 { x: 0.0, y: 1.0 }); });
    let hovered = ctx.window("w", Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 }, PanelFlags::empty(), |w| w.is_hovered());
    assert_eq!(hovered, Some(true));
    ctx.clear();
}
#[test] fn set_focus_on_unknown_name_keeps_windows() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    // Each window is movable, so Nuklear keeps its stored position. A lost window loses its position.
    let frame = |ctx: &mut Context| -> Vec<Option<Vec2>> {
        ["a", "b", "c"].iter().enumerate().map(|(i, n)| ctx.window(n, Rect { x: 0.0, y: 50.0 * i as f32, w: 40.0, h: 40.0 }, PanelFlags::MOVABLE, |w| w.position())).collect()
    };
    frame(&mut ctx);
    ctx.window_set_position("c", Vec2 { x: 5.0, y: 105.0 });
    ctx.window_set_focus("nope");
    ctx.clear();
    // A click in "b" makes Nuklear move "b" to the end of the window list.
    ctx.input(|i| { i.motion(10, 60); i.button(Button::Left, 10, 60, true); });
    frame(&mut ctx);
    ctx.clear();
    let last = frame(&mut ctx);
    assert_eq!(last[2], Some(Vec2 { x: 5.0, y: 105.0 }), "window c is lost");
    assert!(last.iter().all(Option::is_some));
    ctx.clear();
}
