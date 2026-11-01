//! Helpers that the integration tests share.
use nuklear::*;

/// A font with a fixed cell size: 12 pixels high and 6 pixels wide per character.
pub struct FixedFont;

impl Font for FixedFont {
    fn height(&self) -> f32 { 12.0 }
    fn width(&self, _h: f32, t: &str) -> f32 { 6.0 * t.chars().count() as f32 }
    fn glyph(&self, _h: f32, _c: char, _n: char) -> Glyph { Glyph { width: 6.0, height: 12.0, xadvance: 6.0, ..Default::default() } }
    fn texture(&self) -> Handle { Handle::id(1) }
}
