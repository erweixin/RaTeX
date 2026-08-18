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

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, OnceLock, RwLock};
use system_fonts::{find_for_system_locale, FontStyle, FoundFontSource};

/// Shared immutable font-file storage.
///
/// System fonts are normally backed by a read-only file mapping, so even very
/// large TTCs consume physical memory only for pages actually touched by the
/// parser or rasterizer. Cloning this value never copies the font bytes.
#[derive(Clone)]
pub struct FontData(Arc<dyn AsRef<[u8]> + Send + Sync>);

impl FontData {
    fn new(storage: impl AsRef<[u8]> + Send + Sync + 'static) -> Self {
        Self(Arc::new(storage))
    }

    fn from_shared(storage: Arc<dyn AsRef<[u8]> + Send + Sync>) -> Self {
        Self(storage)
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_ref().as_ref()
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

impl std::fmt::Debug for FontData {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("FontData")
            .field("len", &self.as_slice().len())
            .finish()
    }
}

/// `(full font file bytes, face index within TTC or 0 for single-font / unknown collection face)`.
static UNICODE_FONT: OnceLock<Option<(FontData, u32)>> = OnceLock::new();
static SYSTEM_FALLBACK_FONT: OnceLock<Option<(FontData, u32)>> = OnceLock::new();
/// `(full font file bytes, face index within TTC or 0 for single font)`.
static EMOJI_FONT: OnceLock<Option<(FontData, u32)>> = OnceLock::new();

/// Canonical path → shared mapping. This also deduplicates primary and fallback
/// discovery when both resolve to the same physical font file.
static FONT_FILE_CACHE: LazyLock<RwLock<HashMap<PathBuf, FontData>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
const FONT_FILE_CACHE_CAP: usize = 16;

/// Raw TTF/OTF bytes of a discovered Unicode font, or `None` if no suitable font was found.
///
/// Checks (in order):
/// 1. `RATEX_UNICODE_FONT` environment variable
/// 2. Hard-coded system paths (Linux, macOS, Windows)
/// 3. `fontdb` system font database (SansSerif query, then brute-force)
///
/// The result is cached after the first call.
pub fn load_unicode_font_data() -> Option<FontData> {
    UNICODE_FONT
        .get_or_init(load_unicode_fallback_font)
        .as_ref()
        .map(|(bytes, _)| bytes.clone())
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
    SYSTEM_FALLBACK_FONT
        .get_or_init(load_secondary_fallback_font)
        .as_ref()
        .map(|(bytes, _)| bytes.clone())
}

/// Collection index for the cached fallback Unicode face (`0` when not a collection).
pub fn fallback_font_face_index() -> Option<u32> {
    SYSTEM_FALLBACK_FONT
        .get_or_init(load_secondary_fallback_font)
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
    EMOJI_FONT
        .get_or_init(discover_emoji_font)
        .as_ref()
        .map(|(bytes, _)| bytes.clone())
}

/// Collection index for the cached emoji face (`0` when the font is not a TTC).
pub fn emoji_font_face_index() -> Option<u32> {
    EMOJI_FONT
        .get_or_init(discover_emoji_font)
        .as_ref()
        .map(|(_, i)| *i)
}

/// Fast codepoint filter used before touching the large color-emoji font.
/// False positives are acceptable (the cmap is still authoritative), while
/// ordinary CJK and mathematical symbols must remain false so they never map
/// the emoji TTC as a side effect of fallback probing.
pub fn is_emoji_candidate(ch: char) -> bool {
    matches!(
        ch as u32,
        0x1F000..=0x1FAFF
            | 0x2300..=0x23FF
            | 0x2600..=0x27BF
            | 0x2B00..=0x2BFF
            | 0x00A9
            | 0x00AE
            | 0x203C
            | 0x2049
            | 0x2122
            | 0x2139
            | 0x3030
            | 0x303D
            | 0x3297
            | 0x3299
    )
}

// Compatibility shims for callers that still require owned bytes. RaTeX's
// renderers use the `FontData` APIs above, so these copies are never created on
// the normal path.
static LEGACY_UNICODE_FONT: OnceLock<Option<Arc<Vec<u8>>>> = OnceLock::new();
static LEGACY_FALLBACK_FONT: OnceLock<Option<Arc<Vec<u8>>>> = OnceLock::new();
static LEGACY_EMOJI_FONT: OnceLock<Option<Arc<Vec<u8>>>> = OnceLock::new();

#[deprecated(note = "use load_unicode_font_data to avoid copying large system fonts")]
pub fn load_unicode_font_arc() -> Option<Arc<Vec<u8>>> {
    LEGACY_UNICODE_FONT
        .get_or_init(|| load_unicode_font_data().map(|data| Arc::new(data.as_slice().to_vec())))
        .clone()
}

#[deprecated(note = "use load_fallback_font_data to avoid copying large system fonts")]
pub fn load_fallback_font_arc() -> Option<Arc<Vec<u8>>> {
    LEGACY_FALLBACK_FONT
        .get_or_init(|| load_fallback_font_data().map(|data| Arc::new(data.as_slice().to_vec())))
        .clone()
}

#[deprecated(note = "use load_emoji_font_data to avoid copying large system fonts")]
pub fn load_emoji_font_arc() -> Option<Arc<Vec<u8>>> {
    LEGACY_EMOJI_FONT
        .get_or_init(|| load_emoji_font_data().map(|data| Arc::new(data.as_slice().to_vec())))
        .clone()
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

    // 2. System font discovery
    discover_system_font()
}

fn load_secondary_fallback_font() -> Option<(FontData, u32)> {
    // With no user override the primary is already the preferred system font;
    // reuse it instead of repeating discovery and creating another mapping.
    if std::env::var_os("RATEX_UNICODE_FONT").is_none() {
        return UNICODE_FONT.get_or_init(load_unicode_fallback_font).clone();
    }
    discover_system_font()
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

    let bytes = map_font_file(Path::new(path))?;
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

fn map_font_file(path: &Path) -> Option<FontData> {
    let key = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if let Some(cached) = FONT_FILE_CACHE.read().ok()?.get(&key).cloned() {
        return Some(cached);
    }

    let file = std::fs::File::open(&key).ok()?;
    // SAFETY: the mapping is read-only and `FontData` owns it for the full
    // lifetime of every returned slice. System font files are immutable in
    // normal operation; replacing a path creates a new mapping on the next
    // process rather than mutating this one.
    let mapped = unsafe { memmap2::MmapOptions::new().map(&file).ok()? };
    let data = FontData::new(mapped);

    let mut cached = FONT_FILE_CACHE.write().ok()?;
    if let Some(existing) = cached.get(&key) {
        return Some(existing.clone());
    }
    if cached.len() >= FONT_FILE_CACHE_CAP {
        cached.clear();
    }
    cached.insert(key, data.clone());
    Some(data)
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
                fontdb::Source::File(path) => map_font_file(path),
                fontdb::Source::SharedFile(_, storage) | fontdb::Source::Binary(storage) => {
                    Some(FontData::from_shared(Arc::clone(storage)))
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
    #[cfg(target_os = "macos")]
    fn canonical_font_paths_share_one_mapping() {
        let alias = Path::new("/Library/Fonts/Arial Unicode.ttf");
        let target = Path::new("/System/Library/Fonts/Supplemental/Arial Unicode.ttf");
        if !alias.exists() || !target.exists() {
            return;
        }
        let first = map_font_file(alias).expect("map alias");
        let second = map_font_file(target).expect("map target");
        assert_eq!(first.as_slice().as_ptr(), second.as_slice().as_ptr());
    }
}
