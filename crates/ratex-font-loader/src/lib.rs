use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex, OnceLock, RwLock};

use ab_glyph::FontVec;
use ratex_font::FontId;
use ratex_types::display_item::DisplayItem;

pub mod outline_cache;

#[derive(Debug, Clone)]
enum FontBytes {
    Owned(Arc<Vec<u8>>),
    System(ratex_unicode_font::FontData),
}

impl FontBytes {
    fn as_slice(&self) -> &[u8] {
        match self {
            Self::Owned(bytes) => bytes.as_slice(),
            Self::System(bytes) => bytes.as_slice(),
        }
    }

    fn len(&self) -> usize {
        self.as_slice().len()
    }

    fn to_vec(&self) -> Vec<u8> {
        self.as_slice().to_vec()
    }
}

#[derive(Debug, Clone)]
struct LoadedFont {
    bytes: FontBytes,
    source_id: OutlineSourceId,
}

impl LoadedFont {
    fn new(bytes: FontBytes) -> Self {
        Self {
            bytes,
            source_id: fresh_outline_source_id(),
        }
    }
}

type CachedFont = Option<LoadedFont>;

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

/// Cheap per-glyph cache key component identifying a concrete loaded font.
///
/// Loader-created IDs identify the generation of the raw bytes, so reloading
/// a replaced file cannot reuse outlines from an older cache entry. The
/// source-interning compatibility API below also returns this type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutlineSourceId(u64);

static NEXT_OUTLINE_SOURCE_ID: AtomicU64 = AtomicU64::new(1);
static OUTLINE_SOURCE_IDS: LazyLock<RwLock<HashMap<FontSourceKey, u64>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Bound the source-intern table. Clearing it only drops the path-to-id
/// mapping; IDs are allocated monotonically, so previously cached outlines
/// keep their original (still valid) source IDs and simply become cold.
const OUTLINE_SOURCE_CACHE_CAP: usize = 4096;

fn fresh_outline_source_id() -> OutlineSourceId {
    OutlineSourceId(NEXT_OUTLINE_SOURCE_ID.fetch_add(1, Ordering::Relaxed))
}

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
    let source_id = fresh_outline_source_id();
    sources.insert(source, source_id.0);
    source_id
}

pub(crate) fn legacy_outline_source_id() -> OutlineSourceId {
    intern_outline_source(FontSourceKey::Legacy)
}

#[derive(Debug, Clone)]
struct ParsedFont {
    font: Arc<FontVec>,
    source_id: OutlineSourceId,
}

impl ParsedFont {
    fn byte_len(&self) -> usize {
        self.font.as_slice().len()
    }
}

#[derive(Debug, Clone)]
enum ParsedFontCacheEntry {
    Parsed(ParsedFont),
    Missing,
}

impl ParsedFontCacheEntry {
    fn byte_len(&self) -> usize {
        match self {
            Self::Parsed(parsed) => parsed.byte_len(),
            Self::Missing => 0,
        }
    }
}

#[derive(Default)]
struct ParsedFontCache {
    entries: HashMap<CacheKey, ParsedFontCacheEntry>,
    bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ParseFlightKey {
    cache_key: CacheKey,
    source_id: OutlineSourceId,
}

type ParseFlight = Arc<OnceLock<Result<ParsedFont, String>>>;

#[derive(Debug, Clone)]
pub struct FontSet {
    fonts: HashMap<FontId, LoadedFont>,
}

impl FontSet {
    pub fn get(&self, id: &FontId) -> Option<&[u8]> {
        self.fonts.get(id).map(|font| font.bytes.as_slice())
    }

    pub fn contains_key(&self, id: &FontId) -> bool {
        self.fonts.contains_key(id)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&FontId, &[u8])> {
        self.fonts
            .iter()
            .map(|(id, font)| (id, font.bytes.as_slice()))
    }

    pub fn iter_with_source(&self) -> impl Iterator<Item = (&FontId, &[u8], OutlineSourceId)> {
        self.fonts
            .iter()
            .map(|(id, font)| (id, font.bytes.as_slice(), font.source_id))
    }

    /// Load one system fallback into this set only when a renderer proves it
    /// is needed. Normal math and successful primary-CJK paths never call this.
    pub fn ensure_system_font(&mut self, font_id: FontId) -> Result<bool, String> {
        if self.contains_key(&font_id) {
            return Ok(true);
        }
        if !is_system_font_id(font_id) {
            return Err(format!(
                "{} is not a system fallback font",
                font_id.as_str()
            ));
        }

        let plan = FontLoadPlan {
            required: HashSet::new(),
            optional: HashSet::from([font_id]),
        };
        let loaded = load_fonts_for_plan("", &plan)?;
        self.fonts.extend(loaded.fonts);
        Ok(self.contains_key(&font_id))
    }
}

impl From<HashMap<FontId, Vec<u8>>> for FontSet {
    fn from(fonts: HashMap<FontId, Vec<u8>>) -> Self {
        Self {
            fonts: fonts
                .into_iter()
                .map(|(id, bytes)| (id, LoadedFont::new(FontBytes::Owned(Arc::new(bytes)))))
                .collect(),
        }
    }
}

/// Parsed-font cache handle shared by PNG and SVG-standalone renderers.
///
/// Small fonts are represented by `FontVec`, which owns a copied byte buffer
/// plus pre-parsed cmap and kern subtables. Large fonts and the system CJK and
/// emoji fallbacks retain the raw cache's shared bytes instead; renderers borrow
/// those bytes as `FontRef`, avoiding a second whole-file allocation. Every
/// entry carries the raw-font generation's [`OutlineSourceId`], preventing
/// outlines from a replaced and reloaded font file from being reused.
///
/// The accessors intentionally return `ab_glyph::FontVec`, so the public API
/// surface of this crate follows the `ab_glyph` version used by RaTeX. Treat
/// an `ab_glyph` major-version bump as a breaking change for this type.
#[derive(Debug)]
pub struct ParsedFontSet {
    fonts: HashMap<FontId, ParsedFont>,
    raw_fonts: HashMap<FontId, LoadedFont>,
}

impl ParsedFontSet {
    pub fn get(&self, id: &FontId) -> Option<&FontVec> {
        self.fonts.get(id).map(|parsed| parsed.font.as_ref())
    }

    pub fn contains_key(&self, id: &FontId) -> bool {
        self.fonts.contains_key(id) || self.raw_fonts.contains_key(id)
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

    /// Large/system fonts kept in shared read-only storage (normally mmap).
    /// Renderers borrow these bytes as `FontRef` instead of copying the whole
    /// file into a long-lived `Vec` or `FontVec` allocation.
    pub fn iter_raw_with_source(&self) -> impl Iterator<Item = (&FontId, &[u8], OutlineSourceId)> {
        self.raw_fonts
            .iter()
            .map(|(id, font)| (id, font.bytes.as_slice(), font.source_id))
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
        let optional = HashSet::new();

        for item in items {
            if let DisplayItem::GlyphPath { font, .. } = item {
                if let Some(font_id) = FontId::parse(font) {
                    required.insert(font_id);
                }
            }
        }

        required.insert(FontId::MainRegular);

        Self { required, optional }
    }

    pub fn required(&self) -> &HashSet<FontId> {
        &self.required
    }

    pub fn all(&self) -> HashSet<FontId> {
        self.required.union(&self.optional).copied().collect()
    }
}

static FONT_CACHE: OnceLock<RwLock<HashMap<CacheKey, CachedFont>>> = OnceLock::new();

/// Bound the global raw/parsed font caches when many distinct `font_dir`
/// values are used. Entries are value objects (`Arc` clones), so eviction is
/// safe for already-returned `FontSet`/`ParsedFontSet` handles; subsequent
/// renders simply reload and reparse.
const FONT_CACHE_CAP: usize = 4096;
const PARSED_FONT_CACHE_CAP: usize = 4096;

/// Only small fonts are duplicated into an owned `FontVec`. System CJK and
/// emoji faces are always raw-only because their TTF/TTC containers can be
/// tens or hundreds of MiB, while KaTeX's complete font set is about 540 KiB.
const PARSED_FONT_MAX_BYTES: usize = 4 * 1024 * 1024;

/// Aggregate byte budget for owned parsed-font buffers. The entry cap still
/// bounds missing-font markers and map overhead; this cap bounds real font
/// payload independently of entry count.
const PARSED_FONT_CACHE_BYTE_CAP: usize = 32 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsedFontPolicy {
    Parsed,
    RawOnly,
}

fn is_system_font_id(font_id: FontId) -> bool {
    matches!(
        font_id,
        FontId::CjkRegular | FontId::CjkFallback | FontId::EmojiFallback
    )
}

fn parsed_font_policy(font_id: FontId, byte_len: usize) -> ParsedFontPolicy {
    if is_system_font_id(font_id) || byte_len > PARSED_FONT_MAX_BYTES {
        ParsedFontPolicy::RawOnly
    } else {
        ParsedFontPolicy::Parsed
    }
}

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

static PARSED_FONT_CACHE: LazyLock<RwLock<ParsedFontCache>> =
    LazyLock::new(|| RwLock::new(ParsedFontCache::default()));

/// In-flight parse operations keyed by raw-font generation. The flight is
/// removed after its result has been inserted into `PARSED_FONT_CACHE`, so it
/// coordinates cold callers without becoming a second unbounded font cache.
static PARSE_FLIGHTS: LazyLock<Mutex<HashMap<ParseFlightKey, ParseFlight>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(test)]
static TEST_PARSE_COUNTS: LazyLock<Mutex<HashMap<CacheKey, usize>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn parsed_cache() -> &'static RwLock<ParsedFontCache> {
    &PARSED_FONT_CACHE
}

/// Load fonts referenced by `items`, parsing small fonts once into an owned
/// `ab_glyph::FontVec` and keeping large/system fonts as shared raw bytes.
///
/// This is the parsed-font counterpart to [`load_fonts_for_items`]; renderers
/// that need glyph outlines or advances can use the returned [`ParsedFontSet`]
/// while avoiding whole-file copies of large CJK and emoji fonts.
pub fn load_fonts_for_items_parsed(
    font_dir: &str,
    items: &[DisplayItem],
) -> Result<ParsedFontSet, String> {
    let plan = FontLoadPlan::for_display_items(items);
    load_fonts_for_plan_parsed(font_dir, &plan)
}

/// Load exactly one system fallback for an already-failed glyph lookup.
///
/// This keeps CJK and emoji edge cases out of the normal render plan while
/// still sharing the global raw/outline caches when fallback is necessary.
pub fn load_system_font_parsed(font_id: FontId) -> Result<Option<ParsedFontSet>, String> {
    if !is_system_font_id(font_id) {
        return Err(format!(
            "{} is not a system fallback font",
            font_id.as_str()
        ));
    }
    let plan = FontLoadPlan {
        required: HashSet::new(),
        optional: HashSet::from([font_id]),
    };
    let fonts = load_fonts_for_plan_parsed("", &plan)?;
    Ok(fonts.contains_key(&font_id).then_some(fonts))
}

/// Parsed-font counterpart to [`load_fonts_for_plan`].
pub fn load_fonts_for_plan_parsed(
    font_dir: &str,
    plan: &FontLoadPlan,
) -> Result<ParsedFontSet, String> {
    let wanted = plan.all();
    let mut parsed_out = HashMap::new();
    let mut raw_out = HashMap::new();

    // Consult the raw cache first so parsed entries can be matched against the
    // exact byte generation they were built from. A path-only parsed-cache hit
    // is unsafe after the raw entry has been evicted and the file replaced.
    let raw = load_fonts_for_plan(font_dir, plan)?;

    {
        let cached = parsed_cache()
            .read()
            .map_err(|_| "parsed font cache poisoned".to_string())?;
        if collect_cached_layered(
            font_dir,
            &wanted,
            &raw,
            &cached,
            &mut parsed_out,
            &mut raw_out,
        ) {
            validate_required_layered(plan, &parsed_out, &raw_out)?;
            return Ok(ParsedFontSet {
                fonts: parsed_out,
                raw_fonts: raw_out,
            });
        }
    }

    for &font_id in &wanted {
        let key = cache_key(font_dir, font_id);
        match raw.fonts.get(&font_id) {
            Some(loaded)
                if parsed_font_policy(font_id, loaded.bytes.len()) == ParsedFontPolicy::RawOnly =>
            {
                remove_parsed_cache_entry(&key)?;
                raw_out.insert(font_id, loaded.clone());
            }
            Some(loaded) => {
                if let std::collections::hash_map::Entry::Vacant(slot) = parsed_out.entry(font_id) {
                    let parsed = get_or_parse_cached(font_id, key, loaded)?;
                    slot.insert(parsed);
                }
            }
            None => {
                insert_parsed_cache_entry(key, ParsedFontCacheEntry::Missing)?;
            }
        }
    }

    validate_required_layered(plan, &parsed_out, &raw_out)?;
    Ok(ParsedFontSet {
        fonts: parsed_out,
        raw_fonts: raw_out,
    })
}

fn parse_font_vec(font_id: FontId, bytes: Vec<u8>) -> Result<Arc<FontVec>, String> {
    let face_index = font_face_index(font_id);
    FontVec::try_from_vec_and_index(bytes, face_index)
        .map(Arc::new)
        .map_err(|e| format!("Failed to parse font {}: {e:?}", font_id.as_str()))
}

fn cached_parsed_for_generation(
    key: &CacheKey,
    loaded: &LoadedFont,
) -> Result<Option<ParsedFont>, String> {
    let cached = parsed_cache()
        .read()
        .map_err(|_| "parsed font cache poisoned".to_string())?;
    Ok(cached
        .entries
        .get(key)
        .filter(|entry| parsed_entry_matches_raw(entry, Some(loaded)))
        .and_then(|entry| match entry {
            ParsedFontCacheEntry::Parsed(parsed) => Some(parsed.clone()),
            ParsedFontCacheEntry::Missing => None,
        }))
}

fn get_or_parse_cached(
    font_id: FontId,
    key: CacheKey,
    loaded: &LoadedFont,
) -> Result<ParsedFont, String> {
    if let Some(parsed) = cached_parsed_for_generation(&key, loaded)? {
        return Ok(parsed);
    }

    let flight_key = ParseFlightKey {
        cache_key: key.clone(),
        source_id: loaded.source_id,
    };
    let flight = {
        let mut flights = PARSE_FLIGHTS
            .lock()
            .map_err(|_| "parsed font flight cache poisoned".to_string())?;
        Arc::clone(
            flights
                .entry(flight_key.clone())
                .or_insert_with(|| Arc::new(OnceLock::new())),
        )
    };

    let result = flight
        .get_or_init(|| {
            if let Some(parsed) = cached_parsed_for_generation(&key, loaded)? {
                return Ok(parsed);
            }

            // This is the only whole-file copy in the parsed path, and the
            // per-generation flight guarantees only one concurrent caller
            // performs it.
            #[cfg(test)]
            {
                *TEST_PARSE_COUNTS
                    .lock()
                    .expect("test parse count poisoned")
                    .entry(key.clone())
                    .or_default() += 1;
            }
            let font = parse_font_vec(font_id, loaded.bytes.to_vec())?;
            let parsed = ParsedFont {
                font,
                source_id: loaded.source_id,
            };
            insert_parsed_cache_entry(key.clone(), ParsedFontCacheEntry::Parsed(parsed.clone()))?;
            Ok(parsed)
        })
        .clone();

    let mut flights = PARSE_FLIGHTS
        .lock()
        .map_err(|_| "parsed font flight cache poisoned".to_string())?;
    if flights
        .get(&flight_key)
        .is_some_and(|current| Arc::ptr_eq(current, &flight))
    {
        flights.remove(&flight_key);
    }
    result
}

fn insert_parsed_cache_entry(key: CacheKey, entry: ParsedFontCacheEntry) -> Result<(), String> {
    let entry_bytes = entry.byte_len();
    if entry_bytes > PARSED_FONT_CACHE_BYTE_CAP {
        return Ok(());
    }

    let mut cached = parsed_cache()
        .write()
        .map_err(|_| "parsed font cache poisoned".to_string())?;
    if let Some(previous) = cached.entries.remove(&key) {
        cached.bytes = cached.bytes.saturating_sub(previous.byte_len());
    }
    if cached.entries.len() >= PARSED_FONT_CACHE_CAP
        || cached.bytes.saturating_add(entry_bytes) > PARSED_FONT_CACHE_BYTE_CAP
    {
        cached.entries.clear();
        cached.bytes = 0;
    }
    cached.bytes += entry_bytes;
    cached.entries.insert(key, entry);
    Ok(())
}

fn remove_parsed_cache_entry(key: &CacheKey) -> Result<(), String> {
    let mut cached = parsed_cache()
        .write()
        .map_err(|_| "parsed font cache poisoned".to_string())?;
    if let Some(previous) = cached.entries.remove(key) {
        cached.bytes = cached.bytes.saturating_sub(previous.byte_len());
    }
    Ok(())
}

fn collect_cached_layered(
    font_dir: &str,
    wanted: &HashSet<FontId>,
    raw: &FontSet,
    cached: &ParsedFontCache,
    parsed_out: &mut HashMap<FontId, ParsedFont>,
    raw_out: &mut HashMap<FontId, LoadedFont>,
) -> bool {
    let mut all_known = true;
    for &font_id in wanted {
        let key = cache_key(font_dir, font_id);
        match raw.fonts.get(&font_id) {
            Some(loaded)
                if parsed_font_policy(font_id, loaded.bytes.len()) == ParsedFontPolicy::RawOnly =>
            {
                raw_out.insert(font_id, loaded.clone());
                // Force the write path once when this key still has an entry
                // from an older parsed/missing generation, so its byte budget
                // and stale Arc are released.
                if cached.entries.contains_key(&key) {
                    all_known = false;
                }
            }
            Some(loaded) => match cached.entries.get(&key) {
                Some(ParsedFontCacheEntry::Parsed(parsed))
                    if parsed.source_id == loaded.source_id =>
                {
                    parsed_out.insert(font_id, parsed.clone());
                }
                _ => all_known = false,
            },
            None if matches!(
                cached.entries.get(&key),
                Some(ParsedFontCacheEntry::Missing)
            ) => {}
            None => {
                all_known = false;
            }
        }
    }
    all_known
}

fn parsed_entry_matches_raw(entry: &ParsedFontCacheEntry, raw: Option<&LoadedFont>) -> bool {
    match (entry, raw) {
        (ParsedFontCacheEntry::Parsed(parsed), Some(loaded)) => {
            parsed.source_id == loaded.source_id
        }
        (ParsedFontCacheEntry::Missing, None) => true,
        _ => false,
    }
}

fn validate_required_layered(
    plan: &FontLoadPlan,
    parsed: &HashMap<FontId, ParsedFont>,
    raw: &HashMap<FontId, LoadedFont>,
) -> Result<(), String> {
    for &font_id in plan.required() {
        if !parsed.contains_key(&font_id) && !raw.contains_key(&font_id) {
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
            let loaded = load_font_bytes(font_dir, font_id)?.map(LoadedFont::new);
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
    out: &mut HashMap<FontId, LoadedFont>,
) -> bool {
    let mut all_known = true;
    for &font_id in wanted {
        let key = cache_key(font_dir, font_id);
        match cached.get(&key) {
            Some(Some(font)) => {
                out.insert(font_id, font.clone());
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
    loaded: &HashMap<FontId, LoadedFont>,
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
        FontId::CjkRegular => {
            Ok(ratex_unicode_font::load_unicode_font_data().map(FontBytes::System))
        }
        FontId::CjkFallback => {
            Ok(ratex_unicode_font::load_fallback_font_data().map(FontBytes::System))
        }
        FontId::EmojiFallback => {
            Ok(ratex_unicode_font::load_emoji_font_data().map(FontBytes::System))
        }
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
        .map(|bytes| Some(FontBytes::Owned(Arc::new(bytes))))
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
    Ok(ratex_katex_fonts::ttf_bytes(filename)
        .map(|cow| FontBytes::Owned(Arc::new(cow.into_owned()))))
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
    fn non_ascii_fallback_is_not_prefetched() {
        let plan = FontLoadPlan::for_display_items(&[glyph(FontId::MainRegular, '⌘' as u32)]);

        assert!(plan.required.contains(&FontId::MainRegular));
        assert!(!plan.optional.contains(&FontId::CjkRegular));
        assert!(!plan.optional.contains(&FontId::EmojiFallback));
        assert!(!plan.optional.contains(&FontId::CjkFallback));
        assert!(!plan.required.contains(&FontId::CjkRegular));
    }

    #[test]
    fn explicit_cjk_glyph_requires_primary_cjk_font() {
        let plan = FontLoadPlan::for_display_items(&[glyph(FontId::CjkRegular, '你' as u32)]);

        assert!(plan.required.contains(&FontId::CjkRegular));
        assert!(!plan.optional.contains(&FontId::CjkRegular));
        assert!(!plan.optional.contains(&FontId::EmojiFallback));
        assert!(!plan.optional.contains(&FontId::CjkFallback));
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
        let raw = FontSet::from(HashMap::<FontId, Vec<u8>>::new());

        let mut cached = ParsedFontCache::default();
        cached.entries.insert(
            cache_key(font_dir, FontId::EmojiFallback),
            ParsedFontCacheEntry::Missing,
        );

        let mut parsed_out = HashMap::new();
        let mut raw_out = HashMap::new();
        assert!(collect_cached_layered(
            font_dir,
            &wanted,
            &raw,
            &cached,
            &mut parsed_out,
            &mut raw_out,
        ));
        assert!(!parsed_out.contains_key(&FontId::EmojiFallback));
        assert!(!raw_out.contains_key(&FontId::EmojiFallback));
    }

    #[test]
    fn raw_only_font_invalidates_an_older_parsed_cache_marker() {
        let font_dir = "/tmp/ratex-font-loader-test-raw-only-reload";
        let wanted = HashSet::from([FontId::EmojiFallback]);
        let raw = FontSet::from(HashMap::from([(FontId::EmojiFallback, vec![0])]));
        let mut cached = ParsedFontCache::default();
        cached.entries.insert(
            cache_key(font_dir, FontId::EmojiFallback),
            ParsedFontCacheEntry::Missing,
        );

        let mut parsed_out = HashMap::new();
        let mut raw_out = HashMap::new();
        assert!(!collect_cached_layered(
            font_dir,
            &wanted,
            &raw,
            &cached,
            &mut parsed_out,
            &mut raw_out,
        ));
        assert!(raw_out.contains_key(&FontId::EmojiFallback));
    }

    #[test]
    fn parsed_font_policy_keeps_large_and_system_fallback_fonts_raw() {
        assert_eq!(
            parsed_font_policy(FontId::MainRegular, PARSED_FONT_MAX_BYTES),
            ParsedFontPolicy::Parsed
        );
        assert_eq!(
            parsed_font_policy(FontId::MainRegular, PARSED_FONT_MAX_BYTES + 1),
            ParsedFontPolicy::RawOnly
        );
        assert_eq!(
            parsed_font_policy(FontId::CjkRegular, 1),
            ParsedFontPolicy::RawOnly
        );
        assert_eq!(
            parsed_font_policy(FontId::CjkFallback, 1),
            ParsedFontPolicy::RawOnly
        );
        assert_eq!(
            parsed_font_policy(FontId::EmojiFallback, 1),
            ParsedFontPolicy::RawOnly
        );
    }

    #[cfg(not(feature = "embed-fonts"))]
    #[test]
    fn concurrent_cold_loads_share_one_parsed_font_generation() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let source_font = manifest_dir.join("../../fonts/KaTeX_Main-Regular.ttf");
        if !source_font.exists() {
            eprintln!("SKIP concurrent parsed load: fonts not present");
            return;
        }

        let font_dir = std::env::temp_dir().join(format!(
            "ratex-parsed-flight-test-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&font_dir);
        std::fs::create_dir_all(&font_dir).expect("create temp font dir");
        std::fs::copy(&source_font, font_dir.join("KaTeX_Main-Regular.ttf"))
            .expect("copy Main-Regular");

        let font_dir = font_dir.to_string_lossy().to_string();
        let key = cache_key(&font_dir, FontId::MainRegular);
        cache().write().unwrap().remove(&key);
        remove_parsed_cache_entry(&key).unwrap();
        TEST_PARSE_COUNTS.lock().unwrap().remove(&key);

        let plan = Arc::new(FontLoadPlan {
            required: HashSet::from([FontId::MainRegular]),
            optional: HashSet::new(),
        });
        let barrier = Arc::new(std::sync::Barrier::new(9));
        let workers: Vec<_> = (0..8)
            .map(|_| {
                let plan = Arc::clone(&plan);
                let barrier = Arc::clone(&barrier);
                let font_dir = font_dir.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    let fonts = load_fonts_for_plan_parsed(&font_dir, &plan)
                        .expect("concurrent parsed load");
                    assert!(fonts.get(&FontId::MainRegular).is_some());
                })
            })
            .collect();

        barrier.wait();
        for worker in workers {
            worker.join().expect("worker panicked");
        }
        assert_eq!(TEST_PARSE_COUNTS.lock().unwrap().get(&key), Some(&1));

        cache().write().unwrap().remove(&key);
        remove_parsed_cache_entry(&key).unwrap();
        TEST_PARSE_COUNTS.lock().unwrap().remove(&key);
        let _ = std::fs::remove_dir_all(font_dir);
    }

    #[test]
    fn reloaded_font_bytes_get_a_fresh_outline_source() {
        use ab_glyph::{Font, FontRef};

        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let regular = std::fs::read(manifest_dir.join("../../fonts/KaTeX_Main-Regular.ttf"))
            .expect("read Main-Regular");
        let bold = std::fs::read(manifest_dir.join("../../fonts/KaTeX_Main-Bold.ttf"))
            .expect("read Main-Bold");

        // Model two loads of the same logical path/FontId before and after its
        // raw cache entry has been evicted and the file replaced.
        let first = FontSet::from(HashMap::from([(FontId::MainRegular, regular)]));
        let second = FontSet::from(HashMap::from([(FontId::MainRegular, bold)]));
        let (_, first_bytes, first_source) = first.iter_with_source().next().unwrap();
        let (_, second_bytes, second_source) = second.iter_with_source().next().unwrap();
        assert_ne!(first_source, second_source);

        let first_font = FontRef::try_from_slice(first_bytes).expect("parse first font");
        let second_font = FontRef::try_from_slice(second_bytes).expect("parse second font");
        let first_glyph = first_font.glyph_id('x');
        let second_glyph = second_font.glyph_id('x');
        assert_eq!(first_glyph, second_glyph);

        let first_outline = outline_cache::get_or_compute_outline_with_source_id(
            FontId::MainRegular,
            &first_font,
            first_source,
            first_glyph,
        )
        .expect("first outline");
        let second_outline = outline_cache::get_or_compute_outline_with_source_id(
            FontId::MainRegular,
            &second_font,
            second_source,
            second_glyph,
        )
        .expect("second outline");

        assert!(!Arc::ptr_eq(&first_outline, &second_outline));
        assert_ne!(format!("{first_outline:?}"), format!("{second_outline:?}"));
    }

    #[cfg(not(feature = "embed-fonts"))]
    #[test]
    fn parsed_cache_rejects_an_evicted_and_replaced_raw_font() {
        use ab_glyph::Font;

        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let regular = manifest_dir.join("../../fonts/KaTeX_Main-Regular.ttf");
        let bold = manifest_dir.join("../../fonts/KaTeX_Main-Bold.ttf");
        let font_dir = std::env::temp_dir().join(format!(
            "ratex-font-generation-test-{}-{}",
            std::process::id(),
            line!()
        ));
        let _ = std::fs::remove_dir_all(&font_dir);
        std::fs::create_dir_all(&font_dir).expect("create temp font directory");
        let installed = font_dir.join("KaTeX_Main-Regular.ttf");
        std::fs::copy(&regular, &installed).expect("install first font generation");

        let plan = FontLoadPlan {
            required: HashSet::from([FontId::MainRegular]),
            optional: HashSet::new(),
        };
        let font_dir = font_dir.to_string_lossy().to_string();
        let first = load_fonts_for_plan_parsed(&font_dir, &plan).expect("load first generation");
        let (_, first_font, first_source) = first.iter_with_source().next().unwrap();
        let first_glyph = first_font.glyph_id('x');
        let first_outline = outline_cache::get_or_compute_outline_fontvec(
            FontId::MainRegular,
            first_font,
            first_source,
            first_glyph,
        )
        .expect("first outline");

        // Simulate the bounded raw cache evicting this directory while its
        // parsed entry remains hot, then replace the file at the same path.
        cache()
            .write()
            .unwrap()
            .remove(&cache_key(&font_dir, FontId::MainRegular));
        std::fs::copy(&bold, &installed).expect("install second font generation");

        let second = load_fonts_for_plan_parsed(&font_dir, &plan).expect("load second generation");
        let (_, second_font, second_source) = second.iter_with_source().next().unwrap();
        assert_ne!(first_source, second_source);
        let second_glyph = second_font.glyph_id('x');
        assert_eq!(first_glyph, second_glyph);
        let second_outline = outline_cache::get_or_compute_outline_fontvec(
            FontId::MainRegular,
            second_font,
            second_source,
            second_glyph,
        )
        .expect("second outline");
        assert_ne!(format!("{first_outline:?}"), format!("{second_outline:?}"));

        cache()
            .write()
            .unwrap()
            .remove(&cache_key(&font_dir, FontId::MainRegular));
        remove_parsed_cache_entry(&cache_key(&font_dir, FontId::MainRegular)).unwrap();
        let _ = std::fs::remove_dir_all(font_dir);
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

        {
            let cached = parsed_cache().read().unwrap();
            assert!(matches!(
                cached
                    .entries
                    .get(&cache_key(&font_dir, FontId::Size1Regular)),
                Some(ParsedFontCacheEntry::Missing)
            ));
        }

        // A second load must observe the same cached result (the missing
        // optional font is a known `None`, so the fast path can be used).
        let fonts = load_fonts_for_plan_parsed(&font_dir, &plan).expect("second parsed load");
        assert!(fonts.contains_key(&FontId::MainRegular));
        assert!(!fonts.contains_key(&FontId::Size1Regular));

        let _ = std::fs::remove_dir_all(font_dir);
    }
}
