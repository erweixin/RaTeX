//! Discover a system Unicode font for fallback rendering of glyphs not present in KaTeX fonts.
//!
//! Discovery entry points:
//! - `load_unicode_font_arc()` — respects `RATEX_UNICODE_FONT` (highest priority), then system fonts.
//! - `load_fallback_font_arc()` — always discovers a system font, ignoring `RATEX_UNICODE_FONT`.
//!   Useful as a second-level fallback when the primary font doesn't cover a glyph (e.g. emoji
//!   missing from a CJK-only `RATEX_UNICODE_FONT`).
//! - `load_emoji_font_arc()` — color / emoji faces (e.g. Apple Color Emoji) when `CjkFallback` still
//!   has no usable outline for a codepoint (common with Arial Unicode + BMP emoji).
//! - `unicode_font_face_index` / `fallback_font_face_index` / `emoji_font_face_index` — TTC face
//!   indices for `FontRef::try_from_slice_and_index` when discovery returns a font collection.
//!
//! Each result is cached in a `OnceLock` and computed at most once per process.

mod emoji_raster;

pub use emoji_raster::{emoji_png_raster_for_char, emoji_raster_for_char, EmojiRasterStrike};

use std::sync::{Arc, OnceLock};
use system_fonts::{find_for_system_locale, FontStyle, FoundFontSource};

/// `(full font file bytes, face index within TTC or 0 for single-font / unknown collection face)`.
static UNICODE_FONT: OnceLock<Option<(Arc<Vec<u8>>, u32)>> = OnceLock::new();
static SYSTEM_FALLBACK_FONT: OnceLock<Option<(Arc<Vec<u8>>, u32)>> = OnceLock::new();
/// `(full font file bytes, face index within TTC or 0 for single font)`.
static EMOJI_FONT: OnceLock<Option<(Arc<Vec<u8>>, u32)>> = OnceLock::new();

/// Raw TTF/OTF bytes of a discovered Unicode font, or `None` if no suitable font was found.
///
/// Checks (in order):
/// 1. `RATEX_UNICODE_FONT` environment variable
/// 2. Hard-coded system paths (Linux, macOS, Windows)
/// 3. `fontdb` system font database (SansSerif query, then brute-force)
///
/// The result is cached after the first call.
pub fn load_unicode_font_arc() -> Option<Arc<Vec<u8>>> {
    UNICODE_FONT
        .get_or_init(load_unicode_fallback_font)
        .as_ref()
        .map(|(bytes, _)| Arc::clone(bytes))
}

/// Collection index for the cached primary Unicode face (`0` when not a collection).
pub fn unicode_font_face_index() -> Option<u32> {
    UNICODE_FONT
        .get_or_init(load_unicode_fallback_font)
        .as_ref()
        .map(|(_, i)| *i)
}

/// System fallback font for characters not covered by the primary unicode font.
///
/// Always skips `RATEX_UNICODE_FONT` and discovers a font from system paths / fontdb.
/// Intended for use as `CjkFallback` — a second-level fallback when a glyph is `.notdef`
/// in the primary CJK font (e.g. emoji when `RATEX_UNICODE_FONT` points to a CJK-only font).
///
/// The result is cached after the first call.
pub fn load_fallback_font_arc() -> Option<Arc<Vec<u8>>> {
    SYSTEM_FALLBACK_FONT
        .get_or_init(discover_system_font)
        .as_ref()
        .map(|(bytes, _)| Arc::clone(bytes))
}

/// Collection index for the cached fallback Unicode face (`0` when not a collection).
pub fn fallback_font_face_index() -> Option<u32> {
    SYSTEM_FALLBACK_FONT
        .get_or_init(discover_system_font)
        .as_ref()
        .map(|(_, i)| *i)
}

/// Raw font bytes for a system emoji face (color font), or `None` if none was found.
///
/// Uses well-known paths (`.ttc` / `.ttf`) via `fontdb::Database::load_font_file`, then
/// `load_system_fonts` and family queries. Ignores `RATEX_UNICODE_FONT`.
///
/// **Note:** Many emoji fonts are bitmap/COLR-only; outline rasterization may still yield empty
/// paths for some codepoints. PDF embedding of color fonts may also be limited.
///
/// The result is cached after the first call.
pub fn load_emoji_font_arc() -> Option<Arc<Vec<u8>>> {
    EMOJI_FONT
        .get_or_init(discover_emoji_font)
        .as_ref()
        .map(|(bytes, _)| Arc::clone(bytes))
}

/// Collection index for the cached emoji face (`0` when the font is not a TTC).
pub fn emoji_font_face_index() -> Option<u32> {
    EMOJI_FONT
        .get_or_init(discover_emoji_font)
        .as_ref()
        .map(|(_, i)| *i)
}

/// TrueType / OpenType **single** font (not `.ttc`). For collections see [`is_sfnt_container`].
fn is_sfnt_single_font(bytes: &[u8]) -> bool {
    bytes.len() >= 4
        && (bytes[..4] == [0x00, 0x01, 0x00, 0x00]
            || bytes[..4] == [0x4F, 0x54, 0x54, 0x4F]
            || bytes[..4] == [0x74, 0x72, 0x75, 0x65])
}

/// Single font or TrueType **collection** (`ttcf`).
fn is_sfnt_container(bytes: &[u8]) -> bool {
    is_sfnt_single_font(bytes) || bytes.get(0..4) == Some(b"ttcf")
}

fn load_unicode_fallback_font() -> Option<(Arc<Vec<u8>>, u32)> {
    // 1. User-specified font via RATEX_UNICODE_FONT
    if let Ok(spec) = std::env::var("RATEX_UNICODE_FONT") {
        if let Some(font) = load_font_spec(&spec) {
            eprintln!("[ratex-unicode-font] loaded from RATEX_UNICODE_FONT: {}", spec);
            return Some(font);
        }
    }

    // 2. System font discovery
    discover_system_font()
}

/// Discover a font from system paths and locale-aware system-fonts presets (does NOT check
/// `RATEX_UNICODE_FONT`).
///
/// Prioritizes fonts with broad Unicode coverage (emoji, symbols, CJK) so that the fallback
/// is useful even when the primary font (e.g. a narrow Korean font) lacks many glyphs.
fn discover_system_font() -> Option<(Arc<Vec<u8>>, u32)> {
    // 1. Typical system paths with broad Unicode coverage
    #[rustfmt::skip]
    let candidates: &[&str] = &[
        // Linux
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc#Noto Sans CJK SC",
        // macOS
        "/Library/Fonts/Arial Unicode.ttf",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        // Windows
        "C:\\Windows\\Fonts\\NotoSansSC-VF.ttf",
        "C:\\Windows\\Fonts\\msyh.ttc#Microsoft YaHei",
    ];

    for &spec in candidates {
        if let Some(font) = load_font_spec(spec) {
            eprintln!("[ratex-unicode-font] found via builtin path: {}", spec);
            return Some(font);
        }
    }

    // 2. Locale-aware prioritized candidates from system-fonts.
    let (_locale, region, fonts) = find_for_system_locale(FontStyle::Sans);
    for found in fonts {
        let FoundFontSource::Path(path) = found.source else {
            continue;
        };

        let spec = if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("ttc"))
        {
            format!("{}#{}", path.display(), found.family)
        } else {
            path.display().to_string()
        };

        if let Some(font) = load_font_spec(&spec) {
            eprintln!(
                "[ratex-unicode-font] found via system-fonts: {} ({region:?})",
                spec
            );
            return Some(font);
        }
    }

    eprintln!("[ratex-unicode-font] no Unicode font found");
    None
}

enum FaceSelector<'a> {
    Index(u32),
    Family(&'a str),
}

/// Parse and load a font spec: `path` or `path#index` or `path#FamilyName`.
fn load_font_spec(spec: &str) -> Option<(Arc<Vec<u8>>, u32)> {
    let (path, selector) = if let Some((p, suffix)) = spec.rsplit_once('#') {
        if p.is_empty() || suffix.is_empty() {
            (spec, None)
        } else if let Ok(index) = suffix.parse::<u32>() {
            (p, Some(FaceSelector::Index(index)))
        } else {
            (p, Some(FaceSelector::Family(suffix)))
        }
    } else {
        (spec, None)
    };

    let bytes = std::fs::read(std::path::Path::new(path)).ok()?;
    if !is_sfnt_container(&bytes) {
        return None;
    }

    let face_index = match selector {
        None => 0,
        Some(FaceSelector::Index(idx)) => {
            let count = ttf_parser::fonts_in_collection(&bytes).unwrap_or(1);
            if idx >= count {
                return None;
            }
            idx
        }
        Some(FaceSelector::Family(family)) => {
            if is_sfnt_single_font(&bytes) {
                return None;
            }
            find_face_index_by_family(path, family)?
        }
    };

    Some((Arc::new(bytes), face_index))
}

fn find_face_index_by_family(path: &str, family_hint: &str) -> Option<u32> {
    let mut db = fontdb::Database::new();
    db.load_font_file(path).ok()?;
    let face_index = db.faces().find_map(|face| {
        face.families
            .iter()
            .any(|(name, _)| name == family_hint)
            .then_some(face.index)
    });
    face_index
}

fn discover_emoji_font() -> Option<(Arc<Vec<u8>>, u32)> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    #[cfg(target_os = "macos")]
    let emoji_families: &[&str] = &["Apple Color Emoji"];
    #[cfg(target_os = "linux")]
    let emoji_families: &[&str] = &["Noto Color Emoji", "Noto Emoji"];
    #[cfg(target_os = "windows")]
    let emoji_families: &[&str] = &["Segoe UI Emoji"];
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let emoji_families: &[&str] = &[];

    for family in emoji_families {
        let query = fontdb::Query {
            families: &[fontdb::Family::Name(family)],
            weight: fontdb::Weight::NORMAL,
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        };
        if let Some(id) = db.query(&query) {
            if let Some(pair) = db
                .with_face_data(id, |data, index| {
                    is_sfnt_container(data).then(|| (data.to_vec(), index))
                })
                .flatten()
            {
                let bytes = Arc::new(pair.0);
                return Some((bytes, pair.1));
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn test_load_font_spec_macos() {
        let result = load_font_spec("/Library/Fonts/Arial Unicode.ttf");
        assert!(result.is_some(), "Should load Arial Unicode.ttf");
        if let Some((bytes, face_index)) = result {
            assert!(!bytes.is_empty());
            assert_eq!(face_index, 0);
        }

        let result = load_font_spec("/Library/Fonts/Arial Unicode.ttf#0");
        assert!(result.is_some(), "Should load Arial Unicode.ttf#0");
        if let Some((_, face_index)) = result {
            assert_eq!(face_index, 0);
        }

        let result = load_font_spec("/Library/Fonts/Arial Unicode.ttf#1");
        assert!(result.is_none(), "Should fail for TTF with index > 0");

        let result = load_font_spec("/Library/Fonts/Arial Unicode.ttf#Arial Unicode MS");
        assert!(result.is_none(), "Should fail for TTF with family selector");

        let result_family = load_font_spec("/System/Library/Fonts/PingFang.ttc#PingFang SC");
        assert!(result_family.is_some(), "Should load PingFang.ttc with family name");

        let result_default = load_font_spec("/System/Library/Fonts/PingFang.ttc");
        assert!(result_default.is_some(), "Should load PingFang.ttc without selector");
        if let Some((_, face_index)) = result_default {
            assert_eq!(face_index, 0, "TTC without selector should default to face 0");
        }

        if let Some((_, face_index_family)) = result_family {
            let result_index =
                load_font_spec(&format!("/System/Library/Fonts/PingFang.ttc#{}", face_index_family));
            assert!(result_index.is_some(), "Should load PingFang.ttc with index");
            if let Some((_, face_index_idx)) = result_index {
                assert_eq!(
                    face_index_family, face_index_idx,
                    "Family and index should resolve to same face"
                );
            }
        }

        let result = load_font_spec("/System/Library/Fonts/PingFang.ttc#0");
        assert!(result.is_some(), "Should load PingFang.ttc#0");

        let result = load_font_spec("/System/Library/Fonts/PingFang.ttc#NonExistent Font");
        assert!(result.is_none(), "Should fail for non-existent family name");
    }
}
