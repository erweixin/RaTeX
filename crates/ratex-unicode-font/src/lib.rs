//! Discover a system Unicode font for fallback rendering of glyphs not present in KaTeX fonts.
//!
//! Discovery entry points:
//! - `load_unicode_font_data()` — respects `RATEX_UNICODE_FONT` (highest priority), then system fonts.
//! - `load_fallback_font_data()` — always discovers a system font, ignoring `RATEX_UNICODE_FONT`.
//!   Useful as a second-level fallback when the primary font doesn't cover a glyph (e.g. emoji
//!   missing from a CJK-only `RATEX_UNICODE_FONT`).
//! - `load_emoji_font_data()` — color / emoji faces (e.g. Apple Color Emoji) when `CjkFallback` still
//!   has no usable outline for a codepoint (common with Arial Unicode + BMP emoji).
//! - `unicode_font_face_index` / `fallback_font_face_index` / `emoji_font_face_index` — TTC face
//!   indices for `FontRef::try_from_slice_and_index` when discovery returns a font collection.
//!
//! Each result is cached in a `OnceLock` and computed at most once per process.

mod emoji_raster;

pub use emoji_raster::{emoji_png_raster_for_char, emoji_raster_for_char, EmojiRasterStrike};

use std::path::Path;
use std::sync::{Arc, OnceLock};
use system_fonts::{find_for_system_locale, FontStyle, FoundFontSource};

/// Shared immutable, owned font-file storage.
///
/// Loading a font copies its contents into a `Vec`, so the returned slices
/// remain valid even if an externally managed font file is later replaced or
/// truncated. Cloning this value never copies the font bytes.
#[derive(Clone, Debug)]
pub struct FontData {
    bytes: Arc<Vec<u8>>,
}

impl FontData {
    fn new(bytes: Vec<u8>) -> Self {
        Self {
            bytes: Arc::new(bytes),
        }
    }

    pub fn as_slice(&self) -> &[u8] {
        self.bytes.as_slice()
    }

    pub fn len(&self) -> usize {
        self.as_slice().len()
    }

    pub fn is_empty(&self) -> bool {
        self.as_slice().is_empty()
    }

    pub fn as_ptr(&self) -> *const u8 {
        self.as_slice().as_ptr()
    }
}

impl AsRef<[u8]> for FontData {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

/// `(full font file bytes, face index within TTC or 0 for single-font / unknown collection face)`.
static UNICODE_FONT: OnceLock<Option<(FontData, u32)>> = OnceLock::new();
static SYSTEM_FALLBACK_FONT: OnceLock<Option<(FontData, u32)>> = OnceLock::new();
/// `(full font file bytes, face index within TTC or 0 for single font)`.
static EMOJI_FONT: OnceLock<Option<(FontData, u32)>> = OnceLock::new();

/// Raw TTF/OTF bytes of a discovered Unicode font, or `None` if no suitable font was found.
///
/// Checks (in order):
/// 1. `RATEX_UNICODE_FONT` environment variable
/// 2. Hard-coded system paths (Linux, macOS, Windows)
/// 3. `fontdb` system font database (SansSerif query, then brute-force)
///
/// The result is cached after the first call.
pub fn load_unicode_font_data() -> Option<FontData> {
    unicode_font_data_ref().cloned()
}

/// Process-lifetime view of the cached primary Unicode font.
///
/// Unlike [`load_unicode_font_data`], this does not clone the `FontData`
/// handle. The returned storage is owned by this crate's global `OnceLock`, so
/// render-scoped font resolvers can safely retain borrowed parsed faces without
/// copying a large TTF/TTC buffer.
pub fn unicode_font_data_ref() -> Option<&'static FontData> {
    UNICODE_FONT
        .get_or_init(load_unicode_fallback_font)
        .as_ref()
        .map(|(bytes, _)| bytes)
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
pub fn load_fallback_font_data() -> Option<FontData> {
    fallback_font_data_ref().cloned()
}

/// Process-lifetime view of the cached secondary Unicode fallback font.
pub fn fallback_font_data_ref() -> Option<&'static FontData> {
    SYSTEM_FALLBACK_FONT
        .get_or_init(discover_system_font)
        .as_ref()
        .map(|(bytes, _)| bytes)
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
pub fn load_emoji_font_data() -> Option<FontData> {
    emoji_font_data_ref().cloned()
}

/// Process-lifetime view of the cached system emoji font.
pub fn emoji_font_data_ref() -> Option<&'static FontData> {
    EMOJI_FONT
        .get_or_init(discover_emoji_font)
        .as_ref()
        .map(|(bytes, _)| bytes)
}

/// Collection index for the cached emoji face (`0` when the font is not a TTC).
pub fn emoji_font_face_index() -> Option<u32> {
    EMOJI_FONT
        .get_or_init(discover_emoji_font)
        .as_ref()
        .map(|(_, i)| *i)
}

/// Fast codepoint filter used before touching the large color-emoji font.
///
/// These ranges cover Unicode's `Emoji=Yes` property. False positives are
/// acceptable because the cmap remains authoritative, but false negatives
/// would make an emoji unreachable through the color-font fallback.
pub fn is_emoji_candidate(ch: char) -> bool {
    let cp = ch as u32;
    EMOJI_RANGES
        .binary_search_by(|&(start, end)| {
            if cp < start {
                std::cmp::Ordering::Greater
            } else if cp > end {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

// Unicode Emoji=Yes ranges, ordered for binary search. Keep this conservative:
// broad ranges are preferable to a false negative that skips emoji fallback.
const EMOJI_RANGES: &[(u32, u32)] = &[
    (0x0023, 0x0023),
    (0x002A, 0x002A),
    (0x0030, 0x0039),
    (0x00A9, 0x00A9),
    (0x00AE, 0x00AE),
    (0x203C, 0x203C),
    (0x2049, 0x2049),
    (0x2122, 0x2122),
    (0x2139, 0x2139),
    (0x2194, 0x2199),
    (0x21A9, 0x21AA),
    (0x231A, 0x231B),
    (0x2328, 0x2328),
    (0x23CF, 0x23CF),
    (0x23E9, 0x23F3),
    (0x23F8, 0x23FA),
    (0x24C2, 0x24C2),
    (0x25AA, 0x25AB),
    (0x25B6, 0x25B6),
    (0x25C0, 0x25C0),
    (0x25FB, 0x25FE),
    (0x2600, 0x2604),
    (0x260E, 0x260E),
    (0x2611, 0x2611),
    (0x2614, 0x2615),
    (0x2618, 0x2618),
    (0x261D, 0x261D),
    (0x2620, 0x2620),
    (0x2622, 0x2623),
    (0x2626, 0x2626),
    (0x262A, 0x262A),
    (0x262E, 0x262F),
    (0x2638, 0x263A),
    (0x2640, 0x2640),
    (0x2642, 0x2642),
    (0x2648, 0x2653),
    (0x265F, 0x2660),
    (0x2663, 0x2663),
    (0x2665, 0x2666),
    (0x2668, 0x2668),
    (0x267B, 0x267B),
    (0x267E, 0x267F),
    (0x2692, 0x2697),
    (0x2699, 0x2699),
    (0x269B, 0x269C),
    (0x26A0, 0x26A1),
    (0x26A7, 0x26A7),
    (0x26AA, 0x26AB),
    (0x26B0, 0x26B1),
    (0x26BD, 0x26BE),
    (0x26C4, 0x26C5),
    (0x26C8, 0x26C8),
    (0x26CE, 0x26CF),
    (0x26D1, 0x26D1),
    (0x26D3, 0x26D4),
    (0x26E9, 0x26EA),
    (0x26F0, 0x26F5),
    (0x26F7, 0x26FA),
    (0x26FD, 0x26FD),
    (0x2702, 0x2702),
    (0x2705, 0x2705),
    (0x2708, 0x270D),
    (0x270F, 0x270F),
    (0x2712, 0x2712),
    (0x2714, 0x2714),
    (0x2716, 0x2716),
    (0x271D, 0x271D),
    (0x2721, 0x2721),
    (0x2728, 0x2728),
    (0x2733, 0x2734),
    (0x2744, 0x2744),
    (0x2747, 0x2747),
    (0x274C, 0x274C),
    (0x274E, 0x274E),
    (0x2753, 0x2755),
    (0x2757, 0x2757),
    (0x2763, 0x2764),
    (0x2795, 0x2797),
    (0x27A1, 0x27A1),
    (0x27B0, 0x27B0),
    (0x27BF, 0x27BF),
    (0x2934, 0x2935),
    (0x2B05, 0x2B07),
    (0x2B1B, 0x2B1C),
    (0x2B50, 0x2B50),
    (0x2B55, 0x2B55),
    (0x3030, 0x3030),
    (0x303D, 0x303D),
    (0x3297, 0x3297),
    (0x3299, 0x3299),
    (0x1F000, 0x1FAFF),
];

#[deprecated(note = "use load_unicode_font_data")]
pub fn load_unicode_font_arc() -> Option<Arc<Vec<u8>>> {
    load_unicode_font_data().map(|data| data.bytes)
}

#[deprecated(note = "use load_fallback_font_data")]
pub fn load_fallback_font_arc() -> Option<Arc<Vec<u8>>> {
    load_fallback_font_data().map(|data| data.bytes)
}

#[deprecated(note = "use load_emoji_font_data")]
pub fn load_emoji_font_arc() -> Option<Arc<Vec<u8>>> {
    load_emoji_font_data().map(|data| data.bytes)
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

fn load_unicode_fallback_font() -> Option<(FontData, u32)> {
    // 1. User-specified font via RATEX_UNICODE_FONT
    if let Ok(spec) = std::env::var("RATEX_UNICODE_FONT") {
        if let Some(font) = load_font_spec(&spec) {
            eprintln!(
                "[ratex-unicode-font] loaded from RATEX_UNICODE_FONT: {}",
                spec
            );
            return Some(font);
        }
    }

    // 2. System font discovery. Reuse the dedicated system-only cache so the
    // primary and secondary share storage when no override is configured, while
    // the secondary remains independent of environment changes after the primary
    // cache has been initialized.
    SYSTEM_FALLBACK_FONT
        .get_or_init(discover_system_font)
        .clone()
}

/// Discover a font from system paths and locale-aware system-fonts presets (does NOT check
/// `RATEX_UNICODE_FONT`).
///
/// Prioritizes fonts with broad Unicode coverage (emoji, symbols, CJK) so that the fallback
/// is useful even when the primary font (e.g. a narrow Korean font) lacks many glyphs.
fn discover_system_font() -> Option<(FontData, u32)> {
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
fn load_font_spec(spec: &str) -> Option<(FontData, u32)> {
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

    let bytes = load_font_file(Path::new(path))?;
    if !is_sfnt_container(bytes.as_slice()) {
        return None;
    }

    let face_index = match selector {
        None => 0,
        Some(FaceSelector::Index(idx)) => {
            let count = ttf_parser::fonts_in_collection(bytes.as_slice()).unwrap_or(1);
            if idx >= count {
                return None;
            }
            idx
        }
        Some(FaceSelector::Family(family)) => {
            if is_sfnt_single_font(bytes.as_slice()) {
                return None;
            }
            find_face_index_by_family(path, family)?
        }
    };

    Some((bytes, face_index))
}

fn load_font_file(path: &Path) -> Option<FontData> {
    let key = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    std::fs::read(key).ok().map(FontData::new)
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

fn discover_emoji_font() -> Option<(FontData, u32)> {
    // Avoid loading the complete system font database for well-known emoji
    // files. In particular, `fontdb::with_face_data(... data.to_vec())` would
    // transiently hold two complete copies of Apple's ~180 MiB TTC.
    #[cfg(target_os = "macos")]
    let direct_candidates: &[&str] = &["/System/Library/Fonts/Apple Color Emoji.ttc#0"];
    #[cfg(target_os = "linux")]
    let direct_candidates: &[&str] = &[
        "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf#0",
        "/usr/share/fonts/opentype/noto/NotoColorEmoji.ttf#0",
        "/usr/share/fonts/noto/NotoColorEmoji.ttf#0",
    ];
    #[cfg(target_os = "windows")]
    let direct_candidates: &[&str] = &["C:\\Windows\\Fonts\\seguiemj.ttf#0"];
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    let direct_candidates: &[&str] = &[];

    for spec in direct_candidates {
        if let Some(font) = load_font_spec(spec) {
            return Some(font);
        }
    }

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
            let Some(face) = db.face(id) else {
                continue;
            };
            let data = match &face.source {
                fontdb::Source::File(path) => load_font_file(path),
                fontdb::Source::SharedFile(_, storage) | fontdb::Source::Binary(storage) => {
                    Some(FontData::new(storage.as_ref().as_ref().to_vec()))
                }
            };
            let Some(data) = data else {
                continue;
            };
            if is_sfnt_container(data.as_slice()) {
                return Some((data, face.index));
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
        let ttf = "/Library/Fonts/Arial Unicode.ttf";
        if std::path::Path::new(ttf).exists() {
            let result = load_font_spec(ttf);
            assert!(result.is_some(), "Should load Arial Unicode.ttf");
            if let Some((bytes, face_index)) = result {
                assert!(!bytes.as_slice().is_empty());
                assert_eq!(face_index, 0);
            }

            let result = load_font_spec(&format!("{ttf}#0"));
            assert!(result.is_some(), "Should load Arial Unicode.ttf#0");
            if let Some((_, face_index)) = result {
                assert_eq!(face_index, 0);
            }

            let result = load_font_spec(&format!("{ttf}#1"));
            assert!(result.is_none(), "Should fail for TTF with index > 0");

            let result = load_font_spec(&format!("{ttf}#Arial Unicode MS"));
            assert!(result.is_none(), "Should fail for TTF with family selector");
        } else {
            eprintln!("skipping Arial Unicode.ttf checks: {ttf} not found");
        }

        let ttc = "/System/Library/Fonts/PingFang.ttc";
        if std::path::Path::new(ttc).exists() {
            let result_family = load_font_spec(&format!("{ttc}#PingFang SC"));
            assert!(
                result_family.is_some(),
                "Should load PingFang.ttc with family name"
            );

            let result_default = load_font_spec(ttc);
            assert!(
                result_default.is_some(),
                "Should load PingFang.ttc without selector"
            );
            if let Some((_, face_index)) = result_default {
                assert_eq!(
                    face_index, 0,
                    "TTC without selector should default to face 0"
                );
            }

            if let Some((_, face_index_family)) = result_family {
                let result_index = load_font_spec(&format!("{ttc}#{face_index_family}"));
                assert!(
                    result_index.is_some(),
                    "Should load PingFang.ttc with index"
                );
                if let Some((_, face_index_idx)) = result_index {
                    assert_eq!(
                        face_index_family, face_index_idx,
                        "Family and index should resolve to same face"
                    );
                }
            }

            let result = load_font_spec(&format!("{ttc}#0"));
            assert!(result.is_some(), "Should load PingFang.ttc#0");

            let result = load_font_spec(&format!("{ttc}#NonExistent Font"));
            assert!(result.is_none(), "Should fail for non-existent family name");
        } else {
            eprintln!("skipping PingFang.ttc checks: {ttc} not found");
        }
    }

    #[test]
    fn emoji_candidate_filter_excludes_normal_math_and_cjk() {
        assert!(!is_emoji_candidate('x'));
        assert!(!is_emoji_candidate('∫'));
        assert!(!is_emoji_candidate('你'));
        assert!(is_emoji_candidate('😊'));
        assert!(is_emoji_candidate('✅'));
    }

    #[test]
    fn emoji_candidate_filter_covers_emoji_property_edges() {
        for ch in ['#', '↔', '⌨', '☀', '⚕', '🛝', '🫨'] {
            assert!(is_emoji_candidate(ch), "{ch:?} should reach emoji fallback");
        }
    }

    #[test]
    fn font_file_bytes_remain_valid_after_path_is_replaced() {
        let path = std::env::temp_dir().join(format!(
            "ratex-unicode-font-owned-bytes-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::write(&path, [1, 2, 3]).expect("write initial bytes");
        let data = load_font_file(&path).expect("read initial bytes");

        std::fs::write(&path, [4]).expect("replace font path");
        assert_eq!(data.as_slice(), [1, 2, 3]);

        std::fs::remove_file(path).expect("remove temporary font file");
    }
}
