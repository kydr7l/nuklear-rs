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

const BOUNDS: Rect = Rect { x: 0.0, y: 0.0, w: 200.0, h: 200.0 };

fn draw(ctx: &mut Context<'_>, name: &str, labels: usize) -> Option<()> {
    ctx.window(name, BOUNDS, PanelFlags::BORDER, |w| {
        w.layout_row_dynamic(30.0, 1);
        (0..labels).for_each(|_| w.label("x", TextAlign::LEFT));
    })
}

/// The commands of a frame that draws only the window "b".
fn walk_of_b_alone() -> Vec<String> {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    draw(&mut ctx, "b", 4);
    ctx.commands().map(|c| format!("{c:?}")).collect()
}

#[test] fn window_hidden_at_begin_then_shown_draws_nothing() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    draw(&mut ctx, "a", 2);
    ctx.clear();
    ctx.window_show("a", ShowState::Hidden);
    assert!(draw(&mut ctx, "a", 2).is_none());
    draw(&mut ctx, "b", 4);
    ctx.window_show("a", ShowState::Shown);
    let walk: Vec<String> = ctx.commands().map(|c| format!("{c:?}")).collect();
    assert_eq!(walk, walk_of_b_alone());
    ctx.clear();
}
#[test] fn window_closed_at_begin_then_shown_draws_nothing() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    draw(&mut ctx, "a", 2);
    ctx.clear();
    ctx.window_close("a");
    assert!(draw(&mut ctx, "a", 2).is_none());
    draw(&mut ctx, "b", 4);
    ctx.window_show("a", ShowState::Shown);
    let walk: Vec<String> = ctx.commands().map(|c| format!("{c:?}")).collect();
    assert_eq!(walk, walk_of_b_alone());
    ctx.clear();
}
