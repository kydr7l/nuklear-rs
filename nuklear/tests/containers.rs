mod common;
use nuklear::*;
const WIN: Rect = Rect { x: 0.0, y: 0.0, w: 400.0, h: 400.0 };

fn frame<R>(f: impl FnOnce(&mut Window<'_, '_>) -> R) -> Option<R> {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let r = ctx.window("w", WIN, PanelFlags::empty(), f);
    ctx.clear();
    r
}

#[test] fn group_and_tree_bodies_run() {
    let r = frame(|w| {
        w.layout_row_dynamic(200.0, 1);
        let g = w.group("g", PanelFlags::BORDER, |g| { g.layout_row_dynamic(20.0, 1); g.label("in group", TextAlign::LEFT); 1 });
        let t = w.tree(TreeType::Tab, "t", CollapseState::Maximized, 1, |t| { t.layout_row_dynamic(20.0, 1); 2 });
        let closed = w.tree(TreeType::Tab, "closed", CollapseState::Minimized, 2, |_| 3);
        (g, t, closed)
    });
    assert_eq!(r, Some((Some(1), Some(2), None)));
}
#[test] fn nested_popup_is_rejected() {
    let r = frame(|w| {
        w.layout_row_dynamic(30.0, 1);
        w.popup(PopupType::Static, "outer", PanelFlags::empty(), Rect { x: 10.0, y: 10.0, w: 200.0, h: 200.0 }, |p| {
            p.layout_row_dynamic(30.0, 1);
            let inner = p.popup(PopupType::Static, "inner", PanelFlags::empty(), Rect { x: 20.0, y: 20.0, w: 50.0, h: 50.0 }, |_| ());
            let tip = p.tooltip_with(50.0, |_| ());
            (inner, tip)
        })
    });
    assert_eq!(r, Some(Some((None, None))));
}
#[test] fn popup_depth_resets_after_scope() {
    let r = frame(|w| {
        w.layout_row_dynamic(30.0, 1);
        w.popup(PopupType::Static, "p", PanelFlags::empty(), Rect { x: 10.0, y: 10.0, w: 100.0, h: 100.0 }, |p| p.close());
        w.popup(PopupType::Static, "q", PanelFlags::empty(), Rect { x: 10.0, y: 10.0, w: 100.0, h: 100.0 }, |_| 5)
    });
    assert_eq!(r, Some(Some(5)));
}
#[test] fn menubar_after_layout_is_rejected() {
    let r = frame(|w| {
        w.layout_row_dynamic(30.0, 1);
        w.menubar(|_| 1)
    });
    assert_eq!(r, Some(None));
}
#[test] fn closed_combo_and_menu_return_none() {
    let mut sel = 9usize;
    let r = frame(|w| {
        let bar = w.menubar(|m| { m.layout_row_dynamic(25.0, 1); m.menu_label("File", TextAlign::LEFT, Vec2 { x: 100.0, y: 100.0 }, |_| ()) });
        assert_eq!(bar, Some(None));
        w.layout_row_dynamic(30.0, 1);
        let changed = w.combo(&["a", "b"], &mut sel, 25, Vec2 { x: 100.0, y: 100.0 });
        let empty = w.combo(&[], &mut sel, 25, Vec2 { x: 100.0, y: 100.0 });
        let chart = w.chart(ChartType::Lines, 3, 0.0, 1.0, |c| { c.push(0.5); c.push(0.7) });
        let no_chart = w.chart(ChartType::Lines, 0, 0.0, 1.0, |_| ());
        (changed, empty, chart.is_some(), no_chart)
    });
    assert_eq!(r, Some((false, false, true, None)));
    assert_eq!(sel, 1);
}

// A static popup stays active after its scope. A tooltip then fails inside `nk_popup_begin`,
// and Nuklear writes the tooltip type into the parent panel. The wrapper must restore the type, or the
// second popup asserts that popups cannot have popups.
#[test] fn tooltip_and_popup_after_open_popup_do_not_abort() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let r = ctx.window("w", WIN, PanelFlags::empty(), |w| {
        w.layout_row_dynamic(30.0, 1);
        let p = w.popup(PopupType::Static, "p", PanelFlags::empty(), Rect { x: 10.0, y: 10.0, w: 100.0, h: 100.0 }, |_| 1);
        w.tooltip("t");
        let tip = w.tooltip_with(50.0, |_| 2);
        let q = w.popup(PopupType::Static, "q", PanelFlags::empty(), Rect { x: 10.0, y: 10.0, w: 100.0, h: 100.0 }, |_| 3);
        (p, tip, q)
    });
    assert_eq!(r, Some((Some(1), None, None)));
    assert!(ctx.commands().count() > 0);
    ctx.clear();
}

// `nk_group_scrolled_offset_begin` ends a minimized group itself and still returns a true value.
// The body must not run, and the wrapper must not call `nk_group_end` a second time (that asserts).
#[test] fn minimized_group_skips_body_and_end() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let flags = PanelFlags::TITLE | PanelFlags::MINIMIZABLE;
    let body = |ctx: &mut Context<'_>| {
        ctx.window("w", WIN, PanelFlags::empty(), |w| {
            w.layout_row_dynamic(200.0, 1);
            let b = w.widget_bounds();
            let g = w.group("g", flags, |g| { g.layout_row_dynamic(20.0, 1); g.label("x", TextAlign::LEFT); 1 });
            (b, g)
        })
        .unwrap()
    };
    let (b, g) = body(&mut ctx);
    ctx.clear();
    assert_eq!(g, Some(1));
    // The header is 12 + 4 * 4 pixels high. The minimize button is 21 pixels wide, 4 pixels from the right edge.
    let (x, y) = ((b.x + b.w - 15.0) as i32, (b.y + 14.0) as i32);
    ctx.input(|i| { i.motion(x, y); i.button(Button::Left, x, y, true); });
    let (_, g) = body(&mut ctx);
    ctx.clear();
    assert_eq!(g, None);
    ctx.input(|i| i.button(Button::Left, x, y, false));
    let (_, g) = body(&mut ctx);
    ctx.clear();
    assert_eq!(g, Some(1));
}

#[test] fn other_scopes_balance() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let size = Vec2 { x: 100.0, y: 100.0 };
    let r = ctx.window("w", WIN, PanelFlags::empty(), |w| {
        w.layout_row_dynamic(100.0, 1);
        let mut scroll = (0u32, 0u32);
        let gs = w.group_scrolled(&mut scroll, "gs", PanelFlags::BORDER, |g| { g.layout_row_dynamic(20.0, 1); 1 });
        let mut state = CollapseState::Maximized;
        let ts = w.tree_state(TreeType::Node, "ts", &mut state, |_| 2);
        let mut selected = false;
        let te = w.tree_element(TreeType::Node, "te", CollapseState::Maximized, &mut selected, 3, |_| 3);
        w.layout_row_dynamic(30.0, 1);
        let cc = w.combo_color(Color { r: 1, g: 2, b: 3, a: 4 }, size, |_| ());
        let cs = w.combo_symbol_label("s", Symbol::Plus, size, |_| ());
        let ci = w.combo_image_label("i", Image::new(Handle::id(1), 8, 8), size, |_| ());
        let cx = w.contextual(PanelFlags::empty(), size, WIN, |_| ());
        let ms = w.menu_symbol_label("m", TextAlign::LEFT, Symbol::Plus, size, |_| ());
        let mi = w.menu_image_label("n", TextAlign::LEFT, Image::new(Handle::id(1), 8, 8), size, |_| ());
        let nan_tip = w.tooltip_with(f32::NAN, |_| ());
        let tip = w.tooltip_with(50.0, |t| { t.layout_row_dynamic(20.0, 1); 4 });
        let nan_chart = w.chart(ChartType::Column, 3, f32::NAN, 1.0, |_| ());
        let colored = w.chart_colored(ChartType::Column, Color::default(), Color::default(), 2, 0.0, 1.0, |c| c.push(f32::NAN));
        (gs, ts, te, state, (cc, cs, ci, cx, ms, mi), nan_tip, tip, nan_chart, colored)
    });
    assert_eq!(
        r,
        Some((Some(1), Some(2), Some(3), CollapseState::Maximized, (None, None, None, None, None, None), None, Some(4), None, Some(ChartEvents::empty())))
    );
    ctx.clear();
}

// `nk_panel_begin` reads the mouse of a MOVABLE panel through a null input pointer when NO_INPUT is set.
fn move_no_input() -> PanelFlags { PanelFlags::MOVABLE | PanelFlags::NO_INPUT | PanelFlags::TITLE }

/// Run three frames. Frame 1 presses the left button at (50, 50), and frame 2 drags the mouse to (60, 60).
fn three_frames(mut body: impl FnMut(&mut Context<'_>)) {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    for frame in 0..3 {
        match frame {
            1 => ctx.input(|i| { i.motion(50, 50); i.button(Button::Left, 50, 50, true); }),
            2 => ctx.input(|i| i.motion(60, 60)),
            _ => {}
        }
        body(&mut ctx);
        ctx.clear();
    }
}
#[test] fn movable_no_input_window() {
    three_frames(|ctx| { ctx.window("a", WIN, move_no_input(), |_| ()); });
}
#[test] fn movable_no_input_group() {
    three_frames(|ctx| { ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(200.0, 1); w.group("g", move_no_input(), |_| ()); }); });
}
#[test] fn movable_no_input_group_scrolled() {
    let mut scroll = (0, 0);
    three_frames(|ctx| { ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(200.0, 1); w.group_scrolled(&mut scroll, "g", move_no_input(), |_| ()); }); });
}
#[test] fn movable_no_input_popup() {
    let bounds = Rect { x: 10.0, y: 10.0, w: 200.0, h: 200.0 };
    three_frames(|ctx| { ctx.window("w", WIN, PanelFlags::empty(), |w| { w.layout_row_dynamic(30.0, 1); w.popup(PopupType::Static, "p", move_no_input(), bounds, |_| ()); }); });
}
#[test] fn movable_no_input_contextual() {
    let font = UserFont::new(common::FixedFont);
    let mut ctx = Context::new(&font).unwrap();
    let mut opened = Vec::new();
    for frame in 0..3 {
        if frame == 1 {
            ctx.input(|i| { i.motion(50, 50); i.button(Button::Right, 50, 50, true); i.button(Button::Right, 50, 50, false); });
        }
        ctx.window("w", WIN, PanelFlags::empty(), |w| {
            w.layout_row_dynamic(30.0, 1);
            opened.push(w.contextual(move_no_input(), Vec2 { x: 100.0, y: 100.0 }, WIN, |_| ()).is_some());
        });
        ctx.clear();
    }
    assert_eq!(opened, [false, true, true]);
}
