//! Global outline cache shared by PNG, SVG-standalone, PDF, and Cairo renderers.
//!
//! `ab_glyph::Font::outline()` parses the TrueType `glyf` table on every call.
//! The same glyphs appear repeatedly within a formula (e.g. three `2`s in
//! `x^2 + y^2 = z^2`) and across consecutive renders — caching eliminates
//! redundant glyf parsing.
//!
//! Cache keys include an interned font-source id, the TTC face index, and the
//! glyph id, so two different fonts installed under the same `FontId` do not
//! share outlines.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

use ab_glyph::{Font, FontRef, FontVec, GlyphId, OutlineCurve, VariableFont};
use ratex_font::FontId;

use crate::{font_face_index, legacy_outline_source_id, OutlineSourceId};

type OutlineData = Arc<[OutlineCurve]>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct OutlineCacheKey {
    source: OutlineSourceId,
    font_id: FontId,
    face_index: u32,
    glyph_id: GlyphId,
}

fn outline_cache_key(
    source: OutlineSourceId,
    font_id: FontId,
    glyph_id: GlyphId,
) -> OutlineCacheKey {
    OutlineCacheKey {
        source,
        font_id,
        face_index: font_face_index(font_id),
        glyph_id,
    }
}

static OUTLINE_CACHE: LazyLock<RwLock<HashMap<OutlineCacheKey, OutlineData>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Upper bound on cached glyph outlines. The source-aware cache key includes
/// an interned font source ID, so bounding entries prevents old source IDs
/// from retaining outline data indefinitely in long-running processes.
const OUTLINE_CACHE_CAP: usize = 16_384;

fn insert_outline(
    cache: &mut HashMap<OutlineCacheKey, OutlineData>,
    key: OutlineCacheKey,
    curves: OutlineData,
    cap: usize,
) -> OutlineData {
    if let Some(existing) = cache.get(&key) {
        return Arc::clone(existing);
    }
    if cache.len() >= cap {
        cache.clear();
    }
    let result = Arc::clone(&curves);
    cache.insert(key, curves);
    result
}

/// Retrieve cached outline curves, or compute + cache them via `font.outline()`.
///
/// Position and scale are **not** applied — callers must transform the curves
/// with their own `px`, `py`, and `em` values before rasterising or serializing.
///
/// For variable fonts, sets `wght=400` (Regular) if the axis exists and supports it.
///
/// Source-aware raw-font outline lookup using a loader-provided generation ID.
///
/// Obtain `source` from `ParsedFontSet::iter_raw_with_source`,
/// `FontSet::iter_with_source`, or `ResolvedSystemFont::source_id`. A loaded
/// generation, rather than a path, is required so replacing a font file in
/// place cannot return an outline cached from the old bytes.
pub fn get_or_compute_outline_with_source_id(
    font_id: FontId,
    font: &FontRef<'_>,
    source: OutlineSourceId,
    glyph_id: GlyphId,
) -> Option<Arc<[OutlineCurve]>> {
    get_or_compute_outline_with_key(outline_cache_key(source, font_id, glyph_id), font)
}

/// Deprecated compatibility entry point.
///
/// Prefer [`get_or_compute_outline_with_source_id`]: this wrapper has no font
/// source information, so all calls through it share a legacy cache bucket.
#[deprecated(
    note = "use `get_or_compute_outline_with_source_id` with a loader-provided generation ID"
)]
pub fn get_or_compute_outline(
    font_id: FontId,
    font: &FontRef<'_>,
    glyph_id: GlyphId,
) -> Option<Arc<[OutlineCurve]>> {
    get_or_compute_outline_with_source_id(font_id, font, legacy_outline_source_id(), glyph_id)
}

fn get_or_compute_outline_with_key(
    key: OutlineCacheKey,
    font: &FontRef<'_>,
) -> Option<Arc<[OutlineCurve]>> {
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
        instance.outline(key.glyph_id)?
    } else {
        font.outline(key.glyph_id)?
    };
    let curves: Arc<[OutlineCurve]> = outline.curves.into();

    let mut cache = OUTLINE_CACHE.write().unwrap();
    // Double-check: another thread may have inserted while we computed
    Some(insert_outline(&mut cache, key, curves, OUTLINE_CACHE_CAP))
}

/// `FontVec` counterpart to [`get_or_compute_outline_with_source_id`].
///
/// `FontVec` owns the parsed font data and cannot be cloned for the variable
/// font path, so variable fonts are re-parsed as `FontRef` only when the font
/// actually exposes a `wght` axis. Non-variable fonts use the owned `FontVec`
/// directly and avoid per-render font parsing entirely.
pub fn get_or_compute_outline_fontvec(
    font_id: FontId,
    font: &FontVec,
    source: OutlineSourceId,
    glyph_id: GlyphId,
) -> Option<Arc<[OutlineCurve]>> {
    let face_index = font_face_index(font_id);
    let key = outline_cache_key(source, font_id, glyph_id);

    {
        let cache = OUTLINE_CACHE.read().unwrap();
        if let Some(cached) = cache.get(&key) {
            return Some(Arc::clone(cached));
        }
    }

    let needs_variation = font.variations().iter().any(|axis| &axis.tag == b"wght");
    if needs_variation {
        // Preserve the existing `FontRef` behavior for variable fonts.
        let font_ref = FontRef::try_from_slice_and_index(font.as_slice(), face_index).ok()?;
        return get_or_compute_outline_with_source_id(font_id, &font_ref, source, glyph_id);
    }

    let outline = font.outline(glyph_id)?;
    let curves: Arc<[OutlineCurve]> = outline.curves.into();

    let mut cache = OUTLINE_CACHE.write().unwrap();
    Some(insert_outline(&mut cache, key, curves, OUTLINE_CACHE_CAP))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outline_cache_key_includes_source_face_and_glyph() {
        let source_a = crate::fresh_outline_source_id();
        let source_b = crate::fresh_outline_source_id();
        let base = outline_cache_key(source_a, FontId::MainRegular, GlyphId(10));
        let same = outline_cache_key(source_a, FontId::MainRegular, GlyphId(10));
        let other_source = outline_cache_key(source_b, FontId::MainRegular, GlyphId(10));
        let other_glyph = outline_cache_key(source_a, FontId::MainRegular, GlyphId(11));

        assert_eq!(base, same);
        assert_ne!(base, other_source);
        assert_ne!(base, other_glyph);
    }

    #[test]
    fn outline_cache_clears_before_exceeding_capacity() {
        let source = crate::fresh_outline_source_id();
        let key_a = outline_cache_key(source, FontId::MainRegular, GlyphId(10));
        let key_b = outline_cache_key(source, FontId::MainRegular, GlyphId(11));
        let curves: OutlineData = Arc::from([]);
        let mut cache = HashMap::new();

        insert_outline(&mut cache, key_a.clone(), Arc::clone(&curves), 1);
        assert!(cache.contains_key(&key_a));

        insert_outline(&mut cache, key_b.clone(), curves, 1);
        assert_eq!(cache.len(), 1);
        assert!(!cache.contains_key(&key_a));
        assert!(cache.contains_key(&key_b));
    }
}
