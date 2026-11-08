use nuklear::*;

#[test] fn default_font_bakes_and_drives_a_context() {
    let mut b = FontAtlasBuilder::new();
    let id = b.add_default(13.0).unwrap();
    let baked = b.bake(AtlasFormat::Rgba32).unwrap();
    assert!(baked.width() > 0 && baked.height() > 0);
    assert_eq!(baked.pixels().len(), (baked.width() * baked.height() * 4) as usize);
    let (atlas, null) = baked.end(Handle::id(7));
    assert_eq!(null.texture.as_id(), 7);
    let font = atlas.font(id).unwrap();
    assert!(font.height() > 0.0);
    let mut ctx = Context::new(&font).unwrap();
    ctx.window("w", Rect { x: 0.0, y: 0.0, w: 100.0, h: 100.0 }, PanelFlags::TITLE, |w| { w.layout_row_dynamic(20.0, 1); w.label("x", TextAlign::LEFT); });
    assert!(ctx.commands().any(|c| matches!(c, Command::Text { .. })));
    ctx.clear();
}
#[test] fn empty_builder_bake_returns_none() {
    assert!(FontAtlasBuilder::new().bake(AtlasFormat::Alpha8).is_none());
}
#[test] fn baked_atlas_drop_without_end_is_clean() {
    let mut b = FontAtlasBuilder::new();
    b.add_default(13.0).unwrap();
    let baked = b.bake(AtlasFormat::Alpha8).unwrap();
    assert_eq!(baked.pixels().len(), (baked.width() * baked.height()) as usize);
    drop(baked);
}
#[test] fn font_id_from_another_atlas_is_none() {
    let mut two = FontAtlasBuilder::new();
    two.add_default(13.0).unwrap();
    let second = two.add_default(14.0).unwrap();
    let mut one = FontAtlasBuilder::new();
    one.add_default(13.0).unwrap();
    let (atlas, _) = one.bake(AtlasFormat::Alpha8).unwrap().end(Handle::id(1));
    assert_eq!(atlas.len(), 1);
    assert!(atlas.font(second).is_none());
}
#[test] fn height_outside_the_range_is_rejected() {
    let mut b = FontAtlasBuilder::new();
    for h in [0.0, -1.0, f32::NAN, f32::INFINITY, 512.5, 1.0e9] {
        assert!(b.add_default(h).is_none(), "height {h}");
        assert!(b.add_ttf(&[], h).is_none(), "height {h}");
    }
    assert!(b.bake(AtlasFormat::Alpha8).is_none());
}
#[test] fn empty_ttf_is_rejected() {
    let mut b = FontAtlasBuilder::new();
    assert!(b.add_ttf(&[], 13.0).is_none());
}
#[test]
#[cfg_attr(not(windows), ignore = "needs C:/Windows/Fonts/arial.ttf")]
fn ttf_from_the_system_bakes() {
    let ttf = std::fs::read("C:/Windows/Fonts/arial.ttf").expect("C:/Windows/Fonts/arial.ttf ships with Windows");
    let mut b = FontAtlasBuilder::new();
    let id = b.add_ttf(&ttf, 16.0).unwrap();
    drop(ttf); // The atlas copies the bytes.
    let (atlas, _) = b.bake(AtlasFormat::Alpha8).unwrap().end(Handle::id(1));
    assert_eq!(atlas.font(id).unwrap().height(), 16.0);
}
fn advance(font: &BakedFont, c: char) -> f32 {
    let raw = font.raw();
    let mut buf = [0u8; 4];
    let s = c.encode_utf8(&mut buf);
    // SAFETY: the callback and the userdata come from the same baked font, and `s` is valid for its length.
    unsafe { (raw.width.unwrap())(raw.userdata, raw.height, s.as_ptr().cast(), s.len() as i32) }
}
#[test] fn config_sets_the_range_and_the_snap() {
    let cfg = FontConfig { pixel_snap: true, ranges: vec!['a'..='b', '0'..='9'] };
    let mut b = FontAtlasBuilder::new();
    // At 13.5 pixels the built-in font has a fractional advance, so the snap is visible.
    let snapped = b.add_default_with(13.5, &cfg).unwrap();
    let plain = b.add_default(13.5).unwrap();
    let (atlas, _) = b.bake(AtlasFormat::Alpha8).unwrap().end(Handle::id(1));
    let (snapped, plain) = (atlas.font(snapped).unwrap(), atlas.font(plain).unwrap());
    assert_ne!(advance(&plain, 'a').fract(), 0.0);
    assert_eq!(advance(&snapped, 'a').fract(), 0.0);
    // 'z' is outside the ranges, so it draws as the fallback glyph '?', which is also outside the ranges.
    assert_eq!(advance(&snapped, 'z'), advance(&snapped, '?'));
}
#[test] fn bad_ranges_are_rejected() {
    let mut b = FontAtlasBuilder::new();
    // One bad range rejects the whole config, even next to a good one.
    for ranges in [vec!['a'..='b', 'b'..='a'], vec!['a'..='b', '\0'..='z'], vec!['\u{1}'..=char::MAX, '\u{1}'..=char::MAX]] {
        let cfg = FontConfig { pixel_snap: false, ranges };
        assert!(b.add_default_with(13.0, &cfg).is_none(), "{cfg:?}");
    }
    assert!(b.bake(AtlasFormat::Alpha8).is_none());
}
// The atlas code uses C globals. The crate holds a mutex around them, so atlas builds on many threads must not race.
#[test] fn atlas_builds_on_many_threads() {
    let threads: Vec<_> = (0..8)
        .map(|_| {
            std::thread::spawn(|| {
                for _ in 0..20 {
                    let mut b = FontAtlasBuilder::new();
                    let id = b.add_default(13.0).unwrap();
                    let (atlas, _) = b.bake(AtlasFormat::Alpha8).unwrap().end(Handle::id(1));
                    assert_eq!(atlas.font(id).unwrap().height(), 13.0);
                }
            })
        })
        .collect();
    for t in threads {
        t.join().unwrap();
    }
}
