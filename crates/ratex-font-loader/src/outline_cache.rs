//! Global outline cache shared by PNG, SVG-standalone, and PDF renderers.
//!
//! `ab_glyph::Font::outline()` parses the TrueType `glyf` table on every call.
//! The same glyphs appear repeatedly within a formula (e.g. three `2`s in
//! `x^2 + y^2 = z^2`) and across consecutive renders — caching eliminates
//! redundant glyf parsing.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

use ab_glyph::{Font, FontRef, GlyphId, OutlineCurve, VariableFont};
use ratex_font::FontId;

type OutlineData = Arc<[OutlineCurve]>;

static OUTLINE_CACHE: LazyLock<RwLock<HashMap<(FontId, GlyphId), OutlineData>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Retrieve cached outline curves, or compute + cache them via `font.outline()`.
///
/// Position and scale are **not** applied — callers must transform the curves
/// with their own `px`, `py`, and `em` values before rasterising or serializing.
///
/// For variable fonts, sets `wght=400` (Regular) if the axis exists and supports it.
pub fn get_or_compute_outline(
    font_id: FontId,
    font: &FontRef<'_>,
    glyph_id: GlyphId,
) -> Option<Arc<[OutlineCurve]>> {
    let key = (font_id, glyph_id);

    // Fast path: read-lock
    {
        let cache = OUTLINE_CACHE.read().unwrap();
        if let Some(cached) = cache.get(&key) {
            return Some(Arc::clone(cached));
        }
    }

    // Slow path: compute outline + write-lock.
    // For variable fonts, clone + pin to wght=400; non-variable fonts use the original directly.
    // Keep in sync with `variable_weight` in ratex-pdf/src/fonts.rs.
    let needs_variation = font.variations().iter().any(|axis| &axis.tag == b"wght");

    let outline = if needs_variation {
        let mut instance = font.clone();
        for axis in instance.variations() {
            if &axis.tag == b"wght" {
                let w = if axis.min_value <= 400.0 && 400.0 <= axis.max_value {
                    400.0
                } else {
                    axis.default_value
                };
                instance.set_variation(b"wght", w);
                break;
            }
        }
        instance.outline(glyph_id)?
    } else {
        font.outline(glyph_id)?
    };
    let curves: Arc<[OutlineCurve]> = outline.curves.into();

    let mut cache = OUTLINE_CACHE.write().unwrap();
    // Double-check: another thread may have inserted while we computed
    if let Some(existing) = cache.get(&key) {
        return Some(Arc::clone(existing));
    }
    let result = Arc::clone(&curves);
    cache.insert(key, curves);
    Some(result)
}
