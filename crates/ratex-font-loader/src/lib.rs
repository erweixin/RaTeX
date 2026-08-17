use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, OnceLock, RwLock};

use ab_glyph::FontVec;
use ratex_font::FontId;
use ratex_types::display_item::DisplayItem;

pub mod outline_cache;

pub type FontBytes = Arc<Vec<u8>>;
type CachedFont = Option<FontBytes>;

const FONT_MAP: &[(FontId, &str)] = &[
    (FontId::MainRegular, "KaTeX_Main-Regular.ttf"),
    (FontId::MainBold, "KaTeX_Main-Bold.ttf"),
    (FontId::MainItalic, "KaTeX_Main-Italic.ttf"),
    (FontId::MainBoldItalic, "KaTeX_Main-BoldItalic.ttf"),
    (FontId::MathItalic, "KaTeX_Math-Italic.ttf"),
    (FontId::MathBoldItalic, "KaTeX_Math-BoldItalic.ttf"),
    (FontId::AmsRegular, "KaTeX_AMS-Regular.ttf"),
    (FontId::CaligraphicRegular, "KaTeX_Caligraphic-Regular.ttf"),
    (FontId::FrakturRegular, "KaTeX_Fraktur-Regular.ttf"),
    (FontId::FrakturBold, "KaTeX_Fraktur-Bold.ttf"),
    (FontId::SansSerifRegular, "KaTeX_SansSerif-Regular.ttf"),
    (FontId::SansSerifBold, "KaTeX_SansSerif-Bold.ttf"),
    (FontId::SansSerifItalic, "KaTeX_SansSerif-Italic.ttf"),
    (FontId::ScriptRegular, "KaTeX_Script-Regular.ttf"),
    (FontId::TypewriterRegular, "KaTeX_Typewriter-Regular.ttf"),
    (FontId::Size1Regular, "KaTeX_Size1-Regular.ttf"),
    (FontId::Size2Regular, "KaTeX_Size2-Regular.ttf"),
    (FontId::Size3Regular, "KaTeX_Size3-Regular.ttf"),
    (FontId::Size4Regular, "KaTeX_Size4-Regular.ttf"),
];

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum FontSourceKey {
    Embedded,
    Directory(PathBuf),
    SystemUnicode,
    SystemFallback,
    SystemEmoji,
    /// Compatibility bucket for the deprecated legacy outline-cache entry point,
    /// which has no source information to key on.
    Legacy,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    source: FontSourceKey,
    font_id: FontId,
}

/// Cheap per-glyph cache key component identifying a concrete font source.
///
/// Directory/system-font keys are interned once; outline lookups then only
/// hash/copy a `u64` instead of building and hashing a `PathBuf` for every
/// glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutlineSourceId(u64);

static NEXT_OUTLINE_SOURCE_ID: AtomicU64 = AtomicU64::new(1);
static OUTLINE_SOURCE_IDS: LazyLock<RwLock<HashMap<FontSourceKey, u64>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Bound the source-intern table. Clearing it only drops the path-to-id
/// mapping; IDs are allocated monotonically, so previously cached outlines
/// keep their original (still valid) source IDs and simply become cold.
const OUTLINE_SOURCE_CACHE_CAP: usize = 4096;

fn intern_outline_source(source: FontSourceKey) -> OutlineSourceId {
    {
        let sources = OUTLINE_SOURCE_IDS
            .read()
            .expect("outline source cache poisoned");
        if let Some(&id) = sources.get(&source) {
            return OutlineSourceId(id);
        }
    }

    let mut sources = OUTLINE_SOURCE_IDS
        .write()
        .expect("outline source cache poisoned");
    if let Some(&id) = sources.get(&source) {
        return OutlineSourceId(id);
    }
    if sources.len() >= OUTLINE_SOURCE_CACHE_CAP {
        sources.clear();
    }
    let id = NEXT_OUTLINE_SOURCE_ID.fetch_add(1, Ordering::Relaxed);
    sources.insert(source, id);
    OutlineSourceId(id)
}

pub(crate) fn legacy_outline_source_id() -> OutlineSourceId {
    intern_outline_source(FontSourceKey::Legacy)
}

#[derive(Debug, Clone)]
struct ParsedFont {
    font: Arc<FontVec>,
    source_id: OutlineSourceId,
}

#[derive(Debug, Clone)]
enum ParsedFontCacheEntry {
    Parsed(ParsedFont),
    Missing,
}

#[derive(Debug, Clone)]
pub struct FontSet {
    fonts: HashMap<FontId, FontBytes>,
}

impl FontSet {
    pub fn get(&self, id: &FontId) -> Option<&[u8]> {
        self.fonts.get(id).map(|bytes| bytes.as_slice())
    }

    pub fn contains_key(&self, id: &FontId) -> bool {
        self.fonts.contains_key(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&FontId, &[u8])> {
        self.fonts.iter().map(|(id, bytes)| (id, bytes.as_slice()))
    }
}

impl From<HashMap<FontId, Vec<u8>>> for FontSet {
    fn from(fonts: HashMap<FontId, Vec<u8>>) -> Self {
        Self {
            fonts: fonts
                .into_iter()
                .map(|(id, bytes)| (id, Arc::new(bytes)))
                .collect(),
        }
    }
}

/// Parsed-font cache handle shared by PNG and SVG-standalone renderers.
///
/// Each `FontVec` owns its own copy of the TTF/OTF bytes plus pre-parsed cmap
/// and kern subtables, so glyph lookup and outline extraction do not re-parse
/// the font on every render. The global parsed-font cache owns one `Arc`, and
/// this set holds an additional `Arc` clone for the current render. Each entry
/// also carries its interned [`OutlineSourceId`] so outline lookups do not need
/// to rebuild the font source key.
///
/// The accessors intentionally return `ab_glyph::FontVec`, so the public API
/// surface of this crate follows the `ab_glyph` version used by RaTeX. Treat
/// an `ab_glyph` major-version bump as a breaking change for this type.
#[derive(Debug)]
pub struct ParsedFontSet {
    fonts: HashMap<FontId, ParsedFont>,
}

impl ParsedFontSet {
    pub fn get(&self, id: &FontId) -> Option<&FontVec> {
        self.fonts.get(id).map(|parsed| parsed.font.as_ref())
    }

    pub fn contains_key(&self, id: &FontId) -> bool {
        self.fonts.contains_key(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&FontId, &FontVec)> {
        self.fonts
            .iter()
            .map(|(id, parsed)| (id, parsed.font.as_ref()))
    }

    pub fn iter_with_source(&self) -> impl Iterator<Item = (&FontId, &FontVec, OutlineSourceId)> {
        self.fonts
            .iter()
            .map(|(id, parsed)| (id, parsed.font.as_ref(), parsed.source_id))
    }
}

/// Collection face index for fonts that use `ab_glyph::FontVec`.
///
/// This mirrors the private `sfnt_collection_index` helpers in the renderers;
/// keeping it in `ratex-font-loader` lets parsed-font consumers ask the cache
/// layer instead of reaching into `ratex-unicode-font` themselves.
pub fn font_face_index(font_id: FontId) -> u32 {
    match font_id {
        FontId::EmojiFallback => ratex_unicode_font::emoji_font_face_index().unwrap_or(0),
        FontId::CjkRegular => ratex_unicode_font::unicode_font_face_index().unwrap_or(0),
        FontId::CjkFallback => ratex_unicode_font::fallback_font_face_index().unwrap_or(0),
        _ => 0,
    }
}

#[derive(Debug, Clone)]
pub struct FontLoadPlan {
    required: HashSet<FontId>,
    optional: HashSet<FontId>,
}

impl FontLoadPlan {
    pub fn for_display_items(items: &[DisplayItem]) -> Self {
        let mut required = HashSet::new();
        let mut optional = HashSet::new();
        let mut needs_optional_unicode_fallbacks = false;

        for item in items {
            if let DisplayItem::GlyphPath {
                font, char_code, ..
            } = item
            {
                if let Some(font_id) = FontId::parse(font) {
                    match font_id {
                        FontId::CjkRegular | FontId::CjkFallback | FontId::EmojiFallback => {
                            required.insert(font_id);
                            needs_optional_unicode_fallbacks = true;
                        }
                        _ => {
                            required.insert(font_id);
                        }
                    }
                    if may_need_runtime_unicode_fallback(font_id, *char_code) {
                        needs_optional_unicode_fallbacks = true;
                    }
                }
            }
        }

        required.insert(FontId::MainRegular);

        if needs_optional_unicode_fallbacks {
            optional.insert(FontId::CjkRegular);
            optional.insert(FontId::EmojiFallback);
            optional.insert(FontId::CjkFallback);
        }

        Self { required, optional }
    }

    pub fn required(&self) -> &HashSet<FontId> {
        &self.required
    }

    pub fn all(&self) -> HashSet<FontId> {
        self.required.union(&self.optional).copied().collect()
    }
}

fn may_need_runtime_unicode_fallback(font_id: FontId, char_code: u32) -> bool {
    matches!(
        font_id,
        FontId::CjkRegular | FontId::CjkFallback | FontId::EmojiFallback
    ) || (char_code > 0x7f && ratex_font::get_char_metrics(font_id, char_code).is_none())
}

static FONT_CACHE: OnceLock<RwLock<HashMap<CacheKey, CachedFont>>> = OnceLock::new();

/// Bound the global raw/parsed font caches when many distinct `font_dir`
/// values are used. Entries are value objects (`Arc` clones), so eviction is
/// safe for already-returned `FontSet`/`ParsedFontSet` handles; subsequent
/// renders simply reload and reparse.
const FONT_CACHE_CAP: usize = 4096;
const PARSED_FONT_CACHE_CAP: usize = 4096;

/// Evict unrelated entries only when an insertion would exceed `cap`.
///
/// Keeping this decision on the write path is important: a cache that is at
/// capacity can still serve hits indefinitely without becoming cold.
fn make_cache_room_for_insert<K, V>(
    cache: &mut HashMap<K, V>,
    pending: usize,
    cap: usize,
    mut keep: impl FnMut(&K) -> bool,
) {
    if pending == 0 || cache.len().saturating_add(pending) <= cap {
        return;
    }

    cache.retain(|key, _| keep(key));
    if cache.len().saturating_add(pending) > cap {
        cache.clear();
    }
}

fn cache() -> &'static RwLock<HashMap<CacheKey, CachedFont>> {
    FONT_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

static PARSED_FONT_CACHE: LazyLock<RwLock<HashMap<CacheKey, ParsedFontCacheEntry>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

fn parsed_cache() -> &'static RwLock<HashMap<CacheKey, ParsedFontCacheEntry>> {
    &PARSED_FONT_CACHE
}

/// Load fonts referenced by `items`, parsing each TTF once into an owned
/// `ab_glyph::FontVec` and caching the parsed result globally.
///
/// This is the parsed-font counterpart to [`load_fonts_for_items`]; renderers
/// that need glyph outlines or advances can use the returned [`ParsedFontSet`]
/// without paying the `FontRef::try_from_slice` cost on every render.
pub fn load_fonts_for_items_parsed(
    font_dir: &str,
    items: &[DisplayItem],
) -> Result<ParsedFontSet, String> {
    let plan = FontLoadPlan::for_display_items(items);
    load_fonts_for_plan_parsed(font_dir, &plan)
}

/// Parsed-font counterpart to [`load_fonts_for_plan`].
pub fn load_fonts_for_plan_parsed(
    font_dir: &str,
    plan: &FontLoadPlan,
) -> Result<ParsedFontSet, String> {
    let wanted = plan.all();
    let mut out = HashMap::new();
    let cache = parsed_cache();

    {
        let cached = cache
            .read()
            .map_err(|_| "parsed font cache poisoned".to_string())?;
        if collect_cached_parsed(font_dir, &wanted, &cached, &mut out) {
            validate_required_parsed(plan, &out)?;
            return Ok(ParsedFontSet { fonts: out });
        }
    }

    // Ensure raw bytes are available first; then collect the font entries that
    // are still missing from the parsed cache.
    let raw = load_fonts_for_plan(font_dir, plan)?;

    let mut to_parse = Vec::new();
    let mut unavailable = Vec::new();
    {
        let cached = cache
            .read()
            .map_err(|_| "parsed font cache poisoned".to_string())?;
        for &font_id in &wanted {
            let key = cache_key(font_dir, font_id);
            if cached.contains_key(&key) {
                continue;
            }
            match raw.get(&font_id) {
                Some(bytes) => to_parse.push((font_id, bytes.to_vec())),
                None => unavailable.push(font_id),
            }
        }
    }

    // Parse outside the global write lock. Each `FontVec` takes ownership of
    // the prepared byte buffer, so parsing neither serializes with other
    // renderers nor clones that buffer again.
    let mut parsed = Vec::new();
    for (font_id, bytes) in to_parse {
        let font = parse_font_vec(font_id, bytes)?;
        let source_id = intern_outline_source(outline_source_key(font_dir, font_id));
        parsed.push((font_id, ParsedFont { font, source_id }));
    }

    {
        let mut cached = cache
            .write()
            .map_err(|_| "parsed font cache poisoned".to_string())?;
        // Another thread may have inserted entries while we were parsing, so
        // only make room for and fill keys that are still absent.
        let pending = unavailable
            .iter()
            .filter(|&&font_id| !cached.contains_key(&cache_key(font_dir, font_id)))
            .count()
            + parsed
                .iter()
                .filter(|(font_id, _)| !cached.contains_key(&cache_key(font_dir, *font_id)))
                .count();
        let wanted_keys: HashSet<_> = wanted
            .iter()
            .map(|&font_id| cache_key(font_dir, font_id))
            .collect();
        make_cache_room_for_insert(&mut cached, pending, PARSED_FONT_CACHE_CAP, |key| {
            wanted_keys.contains(key)
        });
        for font_id in unavailable {
            cached
                .entry(cache_key(font_dir, font_id))
                .or_insert(ParsedFontCacheEntry::Missing);
        }
        for (font_id, parsed_font) in parsed {
            cached
                .entry(cache_key(font_dir, font_id))
                .or_insert(ParsedFontCacheEntry::Parsed(parsed_font));
        }
        // Re-collect; `out` may already contain entries from the read fast path.
        collect_cached_parsed(font_dir, &wanted, &cached, &mut out);
    }

    validate_required_parsed(plan, &out)?;
    Ok(ParsedFontSet { fonts: out })
}

fn parse_font_vec(font_id: FontId, bytes: Vec<u8>) -> Result<Arc<FontVec>, String> {
    let face_index = font_face_index(font_id);
    FontVec::try_from_vec_and_index(bytes, face_index)
        .map(Arc::new)
        .map_err(|e| format!("Failed to parse font {}: {e:?}", font_id.as_str()))
}

fn collect_cached_parsed(
    font_dir: &str,
    wanted: &HashSet<FontId>,
    cached: &HashMap<CacheKey, ParsedFontCacheEntry>,
    out: &mut HashMap<FontId, ParsedFont>,
) -> bool {
    let mut all_known = true;
    for &font_id in wanted {
        let key = cache_key(font_dir, font_id);
        match cached.get(&key) {
            Some(ParsedFontCacheEntry::Parsed(parsed)) => {
                out.insert(font_id, parsed.clone());
            }
            Some(ParsedFontCacheEntry::Missing) => {}
            None => {
                all_known = false;
            }
        }
    }
    all_known
}

fn validate_required_parsed(
    plan: &FontLoadPlan,
    loaded: &HashMap<FontId, ParsedFont>,
) -> Result<(), String> {
    for &font_id in plan.required() {
        if !loaded.contains_key(&font_id) {
            return Err(format!("Missing required font {}", font_id.as_str()));
        }
    }
    Ok(())
}

pub fn load_fonts_for_items(font_dir: &str, items: &[DisplayItem]) -> Result<FontSet, String> {
    let plan = FontLoadPlan::for_display_items(items);
    load_fonts_for_plan(font_dir, &plan)
}

pub fn load_fonts_for_plan(font_dir: &str, plan: &FontLoadPlan) -> Result<FontSet, String> {
    let wanted = plan.all();
    let mut out = HashMap::new();
    let cache = cache();

    {
        let cached = cache
            .read()
            .map_err(|_| "font cache poisoned".to_string())?;
        if collect_cached(font_dir, &wanted, &cached, &mut out) {
            validate_required(plan, &out)?;
            return Ok(FontSet { fonts: out });
        }
    }

    {
        let mut cached = cache
            .write()
            .map_err(|_| "font cache poisoned".to_string())?;
        let missing: Vec<_> = wanted
            .iter()
            .copied()
            .filter(|&font_id| !cached.contains_key(&cache_key(font_dir, font_id)))
            .collect();
        let wanted_keys: HashSet<_> = wanted
            .iter()
            .map(|&font_id| cache_key(font_dir, font_id))
            .collect();
        make_cache_room_for_insert(&mut cached, missing.len(), FONT_CACHE_CAP, |key| {
            wanted_keys.contains(key)
        });
        for font_id in missing {
            let loaded = load_font_bytes(font_dir, font_id)?;
            cached.insert(cache_key(font_dir, font_id), loaded);
        }
        // Re-collect without clearing `out`: fonts already inserted during the
        // read-lock fast path stay in place (overwritten with identical Arc
        // clones), and newly loaded fonts are added.
        collect_cached(font_dir, &wanted, &cached, &mut out);
    }

    validate_required(plan, &out)?;
    Ok(FontSet { fonts: out })
}

fn collect_cached(
    font_dir: &str,
    wanted: &HashSet<FontId>,
    cached: &HashMap<CacheKey, CachedFont>,
    out: &mut HashMap<FontId, FontBytes>,
) -> bool {
    let mut all_known = true;
    for &font_id in wanted {
        let key = cache_key(font_dir, font_id);
        match cached.get(&key) {
            Some(Some(bytes)) => {
                out.insert(font_id, Arc::clone(bytes));
            }
            Some(None) => {}
            None => {
                all_known = false;
            }
        }
    }
    all_known
}

fn validate_required(
    plan: &FontLoadPlan,
    loaded: &HashMap<FontId, FontBytes>,
) -> Result<(), String> {
    for &font_id in plan.required() {
        if !loaded.contains_key(&font_id) {
            return Err(format!("Missing required font {}", font_id.as_str()));
        }
    }
    Ok(())
}

fn cache_key(font_dir: &str, font_id: FontId) -> CacheKey {
    CacheKey {
        source: source_key(font_dir, font_id),
        font_id,
    }
}

pub(crate) fn source_key(font_dir: &str, font_id: FontId) -> FontSourceKey {
    match font_id {
        FontId::CjkRegular => FontSourceKey::SystemUnicode,
        FontId::CjkFallback => FontSourceKey::SystemFallback,
        FontId::EmojiFallback => FontSourceKey::SystemEmoji,
        _ => katex_source_key(font_dir),
    }
}

/// Cheaper source discriminator for the per-glyph outline cache.
///
/// This uses the same canonical directory identity as the persistent
/// raw/parsed font caches. Callers intern it once per loaded font, avoiding a
/// filesystem lookup on each glyph while preventing stale outline reuse when
/// a relative path or symlink later resolves to a different directory.
pub(crate) fn outline_source_key(font_dir: &str, font_id: FontId) -> FontSourceKey {
    source_key(font_dir, font_id)
}

/// Intern a font source for outline-cache lookups.
///
/// This is intended to be called once per loaded font (not once per glyph) and
/// returns a cheap copyable ID for [`outline_cache::get_or_compute_outline_with_source_id`].
pub fn outline_source_id(font_dir: &str, font_id: FontId) -> OutlineSourceId {
    intern_outline_source(outline_source_key(font_dir, font_id))
}

#[cfg(feature = "embed-fonts")]
fn katex_source_key(_font_dir: &str) -> FontSourceKey {
    FontSourceKey::Embedded
}

#[cfg(not(feature = "embed-fonts"))]
fn katex_source_key(font_dir: &str) -> FontSourceKey {
    FontSourceKey::Directory(normalize_font_dir(font_dir))
}

#[cfg(not(feature = "embed-fonts"))]
fn normalize_font_dir(font_dir: &str) -> PathBuf {
    let path = std::path::Path::new(font_dir);
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

fn load_font_bytes(font_dir: &str, font_id: FontId) -> Result<Option<FontBytes>, String> {
    match font_id {
        FontId::CjkRegular => Ok(ratex_unicode_font::load_unicode_font_arc()),
        FontId::CjkFallback => Ok(ratex_unicode_font::load_fallback_font_arc()),
        FontId::EmojiFallback => Ok(ratex_unicode_font::load_emoji_font_arc()),
        _ => load_katex_font(font_dir, font_id),
    }
}

#[cfg(not(feature = "embed-fonts"))]
fn load_katex_font(font_dir: &str, font_id: FontId) -> Result<Option<FontBytes>, String> {
    let Some(filename) = FONT_MAP
        .iter()
        .find(|(id, _)| *id == font_id)
        .map(|(_, f)| *f)
    else {
        return Ok(None);
    };
    let path = std::path::Path::new(font_dir).join(filename);
    if !path.exists() {
        return Ok(None);
    }
    std::fs::read(&path)
        .map(|bytes| Some(Arc::new(bytes)))
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))
}

#[cfg(feature = "embed-fonts")]
fn load_katex_font(_font_dir: &str, font_id: FontId) -> Result<Option<FontBytes>, String> {
    let Some(filename) = FONT_MAP
        .iter()
        .find(|(id, _)| *id == font_id)
        .map(|(_, f)| *f)
    else {
        return Ok(None);
    };
    Ok(ratex_katex_fonts::ttf_bytes(filename).map(|cow| Arc::new(cow.into_owned())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratex_types::color::Color;

    fn glyph(font: FontId, char_code: u32) -> DisplayItem {
        DisplayItem::GlyphPath {
            x: 0.0,
            y: 0.0,
            scale: 1.0,
            font: font.as_str().to_string(),
            char_code,
            color: Color::BLACK,
        }
    }

    #[test]
    fn ascii_katex_glyph_does_not_request_unicode_fallbacks() {
        let plan = FontLoadPlan::for_display_items(&[glyph(FontId::MainRegular, 'x' as u32)]);

        assert!(plan.required.contains(&FontId::MainRegular));
        assert!(!plan.optional.contains(&FontId::CjkRegular));
        assert!(!plan.optional.contains(&FontId::EmojiFallback));
        assert!(!plan.optional.contains(&FontId::CjkFallback));
    }

    #[test]
    fn non_ascii_without_katex_metrics_requests_optional_unicode_fallbacks() {
        let plan = FontLoadPlan::for_display_items(&[glyph(FontId::MainRegular, '⌘' as u32)]);

        assert!(plan.required.contains(&FontId::MainRegular));
        assert!(plan.optional.contains(&FontId::CjkRegular));
        assert!(plan.optional.contains(&FontId::EmojiFallback));
        assert!(plan.optional.contains(&FontId::CjkFallback));
        assert!(!plan.required.contains(&FontId::CjkRegular));
    }

    #[test]
    fn explicit_cjk_glyph_requires_primary_cjk_font() {
        let plan = FontLoadPlan::for_display_items(&[glyph(FontId::CjkRegular, '你' as u32)]);

        assert!(plan.required.contains(&FontId::CjkRegular));
        assert!(plan.optional.contains(&FontId::EmojiFallback));
        assert!(plan.optional.contains(&FontId::CjkFallback));
    }

    #[test]
    fn cached_missing_optional_font_counts_as_known() {
        let font_dir = "/tmp/ratex-font-loader-test-missing-optional";
        let mut wanted = HashSet::new();
        wanted.insert(FontId::EmojiFallback);

        let mut cached = HashMap::new();
        cached.insert(cache_key(font_dir, FontId::EmojiFallback), None);

        let mut out = HashMap::new();
        assert!(collect_cached(font_dir, &wanted, &cached, &mut out));
        assert!(!out.contains_key(&FontId::EmojiFallback));
    }

    #[test]
    fn cached_missing_optional_parsed_font_counts_as_known() {
        let font_dir = "/tmp/ratex-font-loader-test-missing-optional";
        let mut wanted = HashSet::new();
        wanted.insert(FontId::EmojiFallback);

        let mut cached = HashMap::new();
        cached.insert(
            cache_key(font_dir, FontId::EmojiFallback),
            ParsedFontCacheEntry::Missing,
        );

        let mut out: HashMap<FontId, ParsedFont> = HashMap::new();
        assert!(collect_cached_parsed(font_dir, &wanted, &cached, &mut out));
        assert!(!out.contains_key(&FontId::EmojiFallback));
    }

    #[test]
    fn full_cache_hit_does_not_trigger_eviction() {
        let mut cached = HashMap::from([(1_u8, "one"), (2, "two")]);

        make_cache_room_for_insert(&mut cached, 0, 2, |_| false);

        assert_eq!(cached.len(), 2);
        assert_eq!(cached.get(&1), Some(&"one"));
        assert_eq!(cached.get(&2), Some(&"two"));
    }

    #[test]
    fn cache_miss_evicts_only_unrelated_entries() {
        let mut cached = HashMap::from([(1_u8, "keep"), (2, "evict")]);

        make_cache_room_for_insert(&mut cached, 1, 2, |key| *key == 1);

        assert_eq!(cached, HashMap::from([(1, "keep")]));
    }

    #[cfg(all(not(feature = "embed-fonts"), unix))]
    #[test]
    fn outline_source_id_changes_when_symlink_target_changes() {
        use std::os::unix::fs::symlink;

        let root = std::env::temp_dir().join(format!(
            "ratex-outline-source-test-{}-{}",
            std::process::id(),
            line!()
        ));
        let target_a = root.join("a");
        let target_b = root.join("b");
        let link = root.join("fonts");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&target_a).expect("create first font directory");
        std::fs::create_dir_all(&target_b).expect("create second font directory");
        symlink(&target_a, &link).expect("create font directory symlink");

        let link = link.to_string_lossy().to_string();
        let source_a = outline_source_id(&link, FontId::MainRegular);

        std::fs::remove_file(&link).expect("remove old font directory symlink");
        symlink(&target_b, &link).expect("retarget font directory symlink");
        let source_b = outline_source_id(&link, FontId::MainRegular);

        assert_ne!(source_a, source_b);
        let _ = std::fs::remove_dir_all(root);
    }

    #[cfg(not(feature = "embed-fonts"))]
    #[test]
    fn parsed_loader_caches_unavailable_optional_font() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source_font = manifest_dir.join("../../fonts/KaTeX_Main-Regular.ttf");
        if !source_font.exists() {
            eprintln!("SKIP parsed_loader_caches_unavailable_optional_font: fonts not present");
            return;
        }

        let font_dir = std::env::temp_dir().join(format!(
            "ratex-parsed-loader-test-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&font_dir);
        std::fs::create_dir_all(&font_dir).expect("create temp font dir");
        std::fs::copy(&source_font, font_dir.join("KaTeX_Main-Regular.ttf"))
            .expect("copy Main-Regular");

        let plan = FontLoadPlan {
            required: HashSet::from([FontId::MainRegular]),
            optional: HashSet::from([FontId::Size1Regular]),
        };
        let font_dir = font_dir.to_string_lossy().to_string();

        let fonts = load_fonts_for_plan_parsed(&font_dir, &plan).expect("first parsed load");
        assert!(fonts.contains_key(&FontId::MainRegular));
        assert!(!fonts.contains_key(&FontId::Size1Regular));

        let cached = parsed_cache().read().unwrap();
        assert!(matches!(
            cached.get(&cache_key(&font_dir, FontId::Size1Regular)),
            Some(ParsedFontCacheEntry::Missing)
        ));

        // A second load must observe the same cached result (the missing
        // optional font is a known `None`, so the fast path can be used).
        let fonts = load_fonts_for_plan_parsed(&font_dir, &plan).expect("second parsed load");
        assert!(fonts.contains_key(&FontId::MainRegular));
        assert!(!fonts.contains_key(&FontId::Size1Regular));

        let _ = std::fs::remove_dir_all(font_dir);
    }
}
