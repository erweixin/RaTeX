//! Discover a system Unicode font for fallback rendering of glyphs not present in KaTeX fonts.
//!
//! Discovery entry points:
//! - `load_unicode_font()` — respects `RATEX_UNICODE_FONT` (highest priority), then system fonts.
//! - `load_fallback_font()` — always discovers a system font, ignoring `RATEX_UNICODE_FONT`.
//!   Useful as a second-level fallback when the primary font doesn't cover a glyph (e.g. emoji
//!   missing from a CJK-only `RATEX_UNICODE_FONT`).
//! - `load_emoji_font()` — color / emoji faces (e.g. Apple Color Emoji) when `CjkFallback` still
//!   has no usable outline for a codepoint (common with Arial Unicode + BMP emoji).
//!
//! Each result is cached in a `OnceLock` and computed at most once per process.

mod emoji_raster;

pub use emoji_raster::{emoji_png_raster_for_char, emoji_raster_for_char, EmojiRasterStrike};

use std::sync::OnceLock;

static UNICODE_FONT: OnceLock<Option<Vec<u8>>> = OnceLock::new();
static SYSTEM_FALLBACK_FONT: OnceLock<Option<Vec<u8>>> = OnceLock::new();
/// `(full font file bytes, face index within TTC or 0 for single font)`.
static EMOJI_FONT: OnceLock<Option<(Vec<u8>, u32)>> = OnceLock::new();

/// Raw TTF/OTF bytes of a discovered Unicode font, or `None` if no suitable font was found.
///
/// Checks (in order):
/// 1. `RATEX_UNICODE_FONT` environment variable
/// 2. Hard-coded system paths (Linux, macOS, Windows)
/// 3. `fontdb` system font database (SansSerif query, then brute-force)
///
/// The result is cached after the first call.
pub fn load_unicode_font() -> Option<&'static [u8]> {
    UNICODE_FONT
        .get_or_init(load_unicode_fallback_font)
        .as_ref()
        .map(|v| v.as_slice())
}

/// System fallback font for characters not covered by the primary unicode font.
///
/// Always skips `RATEX_UNICODE_FONT` and discovers a font from system paths / fontdb.
/// Intended for use as `CjkFallback` — a second-level fallback when a glyph is `.notdef`
/// in the primary CJK font (e.g. emoji when `RATEX_UNICODE_FONT` points to a CJK-only font).
///
/// The result is cached after the first call.
pub fn load_fallback_font() -> Option<&'static [u8]> {
    SYSTEM_FALLBACK_FONT
        .get_or_init(discover_system_font)
        .as_ref()
        .map(|v| v.as_slice())
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
pub fn load_emoji_font() -> Option<&'static [u8]> {
    load_emoji_font_with_index().map(|(b, _)| b)
}

/// Same bytes as [`load_emoji_font`], plus the `fontdb` / OpenType **collection** index (use with
/// `ttf_parser::Face::parse(data, index)` and `ab_glyph::FontRef::try_from_slice_and_index`).
pub fn load_emoji_font_with_index() -> Option<(&'static [u8], u32)> {
    EMOJI_FONT
        .get_or_init(discover_emoji_font)
        .as_ref()
        .map(|(v, i)| (v.as_slice(), *i))
}

/// Collection index for the cached emoji face (`0` when the font is not a TTC).
pub fn emoji_font_face_index() -> Option<u32> {
    load_emoji_font_with_index().map(|(_, i)| i)
}

fn is_valid_font(bytes: &[u8]) -> bool {
    is_sfnt_single_font(bytes)
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

fn load_unicode_fallback_font() -> Option<Vec<u8>> {
    // 1. User-specified font via RATEX_UNICODE_FONT
    if let Ok(p) = std::env::var("RATEX_UNICODE_FONT") {
        if let Ok(bytes) = std::fs::read(std::path::Path::new(&p)) {
            if is_sfnt_container(&bytes) {
                return Some(bytes);
            }
        }
    }

    // 2. System font discovery
    discover_system_font()
}

/// Discover a font from system paths and fontdb (does NOT check `RATEX_UNICODE_FONT`).
///
/// Prioritizes fonts with broad Unicode coverage (emoji, symbols, CJK) so that the fallback
/// is useful even when the primary font (e.g. a narrow Korean font) lacks many glyphs.
fn discover_system_font() -> Option<Vec<u8>> {
    // 1. Typical system paths with broad Unicode coverage
    #[rustfmt::skip]
    let candidates: &[&str] = &[
        // Linux
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/opentype/noto/NotoSans-Regular.otf",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.otf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        // macOS — broad-coverage fonts first
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/System/Library/Fonts/Supplemental/Apple Symbols.ttf",
        "/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/Library/Fonts/Arial.ttf",
        // Windows
        "C:\\Windows\\Fonts\\segoeui.ttf",
        "C:\\Windows\\Fonts\\arial.ttf",
    ];

    for path in candidates {
        if let Ok(bytes) = std::fs::read(std::path::Path::new(path)) {
            if is_valid_font(&bytes) {
                return Some(bytes);
            }
        }
    }

    // 2. fontdb — search for well-known broad-coverage families first.
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    #[cfg(target_os = "macos")]
    let fallback_families: &[&str] = &[
        // Broad text coverage first — emoji-only faces lack most CJK ideographs (issue: AppleGothic + 氧/碳).
        "Arial Unicode MS",
        "PingFang SC",
        "PingFang TC",
        "Hiragino Sans",
        "Apple Symbols",
        "Apple Color Emoji",
    ];
    #[cfg(target_os = "linux")]
    let fallback_families: &[&str] = &[
        "Noto Sans CJK SC",
        "Noto Sans CJK TC",
        "Noto Sans CJK JP",
        "Noto Sans Symbols",
        "DejaVu Sans",
        "Liberation Sans",
        "Noto Color Emoji",
    ];
    #[cfg(target_os = "windows")]
    let fallback_families: &[&str] = &[
        "Arial Unicode MS",
        "Microsoft YaHei",
        "Segoe UI Symbol",
        "Segoe UI Emoji",
    ];
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let fallback_families: &[&str] = &[];

    for family in fallback_families {
        let query = fontdb::Query {
            families: &[fontdb::Family::Name(family)],
            weight: fontdb::Weight::NORMAL,
            stretch: fontdb::Stretch::Normal,
            style: fontdb::Style::Normal,
        };
        if let Some(id) = db.query(&query) {
            if let Some((bytes, _)) = db
                .with_face_data(id, |data, index| {
                    is_sfnt_container(data).then(|| (data.to_vec(), index))
                })
                .flatten()
            {
                return Some(bytes);
            }
        }
    }

    // 3. Generic SansSerif query.
    let query = fontdb::Query {
        families: &[fontdb::Family::SansSerif],
        weight: fontdb::Weight::NORMAL,
        stretch: fontdb::Stretch::Normal,
        style: fontdb::Style::Normal,
    };
    if let Some(id) = db.query(&query) {
        if let Some((bytes, _)) = db
            .with_face_data(id, |data, index| {
                is_sfnt_container(data).then(|| (data.to_vec(), index))
            })
            .flatten()
        {
            return Some(bytes);
        }
    }

    // 4. Brute-force fontdb scan (last resort).
    for face in db.faces() {
        if let Some((bytes, _)) = db
            .with_face_data(face.id, |data, index| {
                is_sfnt_container(data).then(|| (data.to_vec(), index))
            })
            .flatten()
        {
            return Some(bytes);
        }
    }

    None
}

fn discover_emoji_font() -> Option<(Vec<u8>, u32)> {
    let mut db = fontdb::Database::new();

    // Prefer explicit paths so `.ttc` collections (e.g. Apple Color Emoji) are loaded reliably.
    #[cfg(target_os = "macos")]
    {
        let _ = db.load_font_file(std::path::Path::new(
            "/System/Library/Fonts/Apple Color Emoji.ttc",
        ));
    }
    #[cfg(target_os = "linux")]
    {
        let _ = db.load_font_file(std::path::Path::new(
            "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
        ));
    }
    #[cfg(target_os = "windows")]
    {
        let _ = db.load_font_file(std::path::Path::new(r"C:\Windows\Fonts\seguiemj.ttf"));
    }

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
                return Some(pair);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{is_sfnt_container, is_valid_font};

    #[test]
    fn valid_truetype_magic() {
        assert!(is_valid_font(&[0x00, 0x01, 0x00, 0x00]));
    }

    #[test]
    fn valid_ttc_magic() {
        assert!(is_sfnt_container(b"ttcfxxxx"));
        assert!(!is_valid_font(b"ttcfxxxx"));
    }

    #[test]
    fn valid_otto_magic() {
        assert!(is_valid_font(&[0x4F, 0x54, 0x54, 0x4F]));
    }

    #[test]
    fn valid_apple_truetype_magic() {
        assert!(is_valid_font(&[0x74, 0x72, 0x75, 0x65]));
    }

    #[test]
    fn invalid_empty_slice() {
        assert!(!is_valid_font(&[]));
    }

    #[test]
    fn invalid_wrong_magic() {
        assert!(!is_valid_font(b"ABCD"));
    }

    #[test]
    fn invalid_woff_magic() {
        assert!(!is_valid_font(b"wOFF"));
    }

    #[test]
    fn invalid_too_short() {
        assert!(!is_valid_font(&[0x00, 0x01, 0x00]));
    }
}
