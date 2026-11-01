mod common;
use nuklear::*;

#[test] fn label_emits_text_command() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    ctx.window("w", Rect { x: 0.0, y: 0.0, w: 200.0, h: 200.0 }, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); w.label("hello", TextAlign::LEFT); });
    let cmds: Vec<Command<'_>> = ctx.commands().collect();
    assert!(cmds.iter().any(|c| matches!(c, Command::RectFilled { .. })));
    assert!(cmds.iter().any(|c| matches!(c, Command::Text { text, font, .. } if text == "hello" && font.as_id() == 1)));
    assert!(cmds.iter().any(|c| matches!(c, Command::Scissor { .. })));
    drop(cmds);
    ctx.clear();
}
#[test] fn commands_can_be_iterated_twice_per_frame() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    ctx.window("w", Rect { x: 0.0, y: 0.0, w: 200.0, h: 200.0 }, PanelFlags::BORDER, |_| ());
    let a = ctx.commands().count();
    let b = ctx.commands().count();
    assert!(a > 0 && a == b);
    ctx.clear();
    assert_eq!(ctx.commands().count(), 0);
}
#[test] fn window_after_commands_is_refused_until_clear() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let bounds = Rect { x: 0.0, y: 0.0, w: 200.0, h: 200.0 };
    ctx.window("w", bounds, PanelFlags::BORDER, |_| ());
    let a = ctx.commands().count();
    assert!(ctx.window("v", bounds, PanelFlags::BORDER, |_| ()).is_none());
    assert!(ctx.window_titled("v", "v", bounds, PanelFlags::BORDER, |_| ()).is_none());
    assert_eq!(ctx.commands().count(), a);
    ctx.clear();
    assert!(ctx.window("w", bounds, PanelFlags::BORDER, |_| ()).is_some());
    ctx.clear();
}
