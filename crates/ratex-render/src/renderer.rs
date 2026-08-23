use std::cell::OnceCell;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

use ab_glyph::{Font, FontRef, FontVec};
use ratex_font::FontId;
use ratex_font_loader::{OutlineSourceId, ParsedFontSet, SystemFontResolver};
use ratex_types::color::Color;
use ratex_types::display_item::{DisplayItem, DisplayList};
use tiny_skia::{
    FillRule, FilterQuality, Paint, PathBuilder, Pixmap, PixmapPaint, Stroke, Transform,
};

/// Options controlling PNG output.
pub struct RenderOptions {
    pub font_size: f32,
    pub padding: f32,
    /// Background fill color for the output PNG. Set alpha to 0.0 for transparency.
    pub background_color: Color,
    /// Directory containing KaTeX `.ttf` files. Used only when `embed-fonts` is disabled.
    pub font_dir: String,
    /// Multiplies pixels-per-em (and padding) so the same layout renders at higher resolution
    /// (e.g. 2.0 to align RaTeX PNG pixel density with Puppeteer `deviceScaleFactor: 2` refs).
    pub device_pixel_ratio: f32,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            font_size: 40.0,
            padding: 10.0,
            background_color: Color::WHITE,
            font_dir: String::new(),
            device_pixel_ratio: 1.0,
        }
    }
}

pub fn render_to_png(
    display_list: &DisplayList,
    options: &RenderOptions,
) -> Result<Vec<u8>, String> {
    let em = options.font_size;
    let pad = options.padding;
    let dpr = options.device_pixel_ratio.clamp(0.01, 16.0);
    let em_px = em * dpr;
    let pad_px = pad * dpr;

    let total_h = display_list.height + display_list.depth;
    let img_w = (display_list.width as f32 * em_px + 2.0 * pad_px).ceil() as u32;
    let img_h = (total_h as f32 * em_px + 2.0 * pad_px).ceil() as u32;

    let img_w = img_w.max(1);
    let img_h = img_h.max(1);

    let mut pixmap = Pixmap::new(img_w, img_h)
        .ok_or_else(|| format!("Failed to create pixmap {}x{}", img_w, img_h))?;

    pixmap.fill(to_tiny_skia_color(options.background_color));

    // Lazy font loading is shared across renderers and source-aware by font_dir.
    render_with_fonts(&mut pixmap, display_list, options, em_px, pad_px, dpr)?;

    encode_png(&pixmap)
}

/// Load fonts lazily and render the DisplayList.
fn render_with_fonts(
    pixmap: &mut Pixmap,
    display_list: &DisplayList,
    options: &RenderOptions,
    em_px: f32,
    pad_px: f32,
    dpr: f32,
) -> Result<(), String> {
    let fonts =
        ratex_font_loader::load_fonts_for_items_parsed(&options.font_dir, &display_list.items)?;
    let font_refs = build_font_refs(&fonts);
    let system_fonts = SystemFontResolver::new();
    render_display_list(
        pixmap,
        display_list,
        &font_refs,
        &system_fonts,
        em_px,
        pad_px,
        dpr,
    );
    Ok(())
}

fn to_tiny_skia_color(color: Color) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba(
        color.r.clamp(0.0, 1.0),
        color.g.clamp(0.0, 1.0),
        color.b.clamp(0.0, 1.0),
        color.a.clamp(0.0, 1.0),
    )
    .unwrap_or(tiny_skia::Color::TRANSPARENT)
}

/// Quantize a [`Color`] to the exact RGBA8 bytes used for painting.
///
/// RGB uses saturating truncation and alpha uses round-to-nearest, matching
/// the pre-cache renderer behavior. Glyph-mask cache keys must use this same
/// quantization, otherwise two different colors can collide on one key and a
/// cache hit would paint the wrong color.
fn color_to_rgba8(color: &Color) -> [u8; 4] {
    [
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        (color.a.clamp(0.0, 1.0) * 255.0).round() as u8,
    ]
}

fn paint_for_color(color: &Color) -> Paint<'static> {
    let [r, g, b, a] = color_to_rgba8(color);
    let mut paint = Paint::default();
    paint.set_color_rgba8(r, g, b, a);
    paint
}

fn normalized_alpha(alpha: f32) -> f32 {
    if alpha.is_finite() {
        alpha.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

enum RendererFont<'a> {
    Parsed(&'a FontVec),
    Raw(Box<LazyRawFont<'a>>),
}

struct LazyRawFont<'a> {
    bytes: &'a [u8],
    face_index: u32,
    parsed: OnceCell<Result<FontRef<'a>, ab_glyph::InvalidFont>>,
}

impl<'a> LazyRawFont<'a> {
    fn get(&self) -> Option<&FontRef<'a>> {
        self.parsed
            .get_or_init(|| FontRef::try_from_slice_and_index(self.bytes, self.face_index))
            .as_ref()
            .ok()
    }
}

trait RendererFontOps: Font {
    fn cached_outline(
        &self,
        font_id: FontId,
        source_id: OutlineSourceId,
        glyph_id: ab_glyph::GlyphId,
    ) -> Option<Arc<[ab_glyph::OutlineCurve]>>;
}

impl RendererFontOps for FontVec {
    fn cached_outline(
        &self,
        font_id: FontId,
        source_id: OutlineSourceId,
        glyph_id: ab_glyph::GlyphId,
    ) -> Option<Arc<[ab_glyph::OutlineCurve]>> {
        ratex_font_loader::outline_cache::get_or_compute_outline_fontvec(
            font_id, self, source_id, glyph_id,
        )
    }
}

impl RendererFontOps for FontRef<'_> {
    fn cached_outline(
        &self,
        font_id: FontId,
        source_id: OutlineSourceId,
        glyph_id: ab_glyph::GlyphId,
    ) -> Option<Arc<[ab_glyph::OutlineCurve]>> {
        ratex_font_loader::outline_cache::get_or_compute_outline_with_source_id(
            font_id, self, source_id, glyph_id,
        )
    }
}

struct RendererFontRef<'a> {
    font: RendererFont<'a>,
    source_id: OutlineSourceId,
}

/// Build a map combining cached small `FontVec`s with borrowed raw large fonts.
///
/// Raw `FontRef`s borrow the `ParsedFontSet`'s `Arc`-backed bytes and therefore
/// do not copy large CJK/emoji TTF/TTC containers.
fn build_font_refs(data: &ParsedFontSet) -> HashMap<FontId, RendererFontRef<'_>> {
    let mut refs = HashMap::new();
    for (id, font, source_id) in data.iter_with_source() {
        refs.insert(
            *id,
            RendererFontRef {
                font: RendererFont::Parsed(font),
                source_id,
            },
        );
    }
    for (id, bytes, source_id) in data.iter_raw_with_source() {
        refs.insert(
            *id,
            RendererFontRef {
                font: RendererFont::Raw(Box::new(LazyRawFont {
                    bytes,
                    face_index: ratex_font_loader::font_face_index(*id),
                    parsed: OnceCell::new(),
                })),
                source_id,
            },
        );
    }
    refs
}

#[allow(clippy::too_many_arguments)]
fn render_char_with_entry(
    pixmap: &mut Pixmap,
    px: f32,
    py: f32,
    font_id: FontId,
    ch: char,
    color: &Color,
    em: f32,
    entry: &RendererFontRef<'_>,
) -> bool {
    match &entry.font {
        RendererFont::Parsed(font) => render_char_with_font(
            pixmap,
            px,
            py,
            font_id,
            ch,
            color,
            em,
            *font,
            entry.source_id,
        ),
        RendererFont::Raw(raw) => raw.get().is_some_and(|font| {
            render_char_with_font(
                pixmap,
                px,
                py,
                font_id,
                ch,
                color,
                em,
                font,
                entry.source_id,
            )
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn render_char_with_system_fallback(
    pixmap: &mut Pixmap,
    px: f32,
    py: f32,
    font_id: FontId,
    ch: char,
    color: &Color,
    em: f32,
    font_cache: &HashMap<FontId, RendererFontRef<'_>>,
    system_fonts: &SystemFontResolver,
) -> bool {
    if let Some(entry) = font_cache.get(&font_id) {
        return render_char_with_entry(pixmap, px, py, font_id, ch, color, em, entry);
    }

    let Ok(Some(font)) = system_fonts.get(font_id) else {
        return false;
    };
    render_char_with_font(
        pixmap,
        px,
        py,
        font_id,
        ch,
        color,
        em,
        font.font(),
        font.source_id(),
    )
}

#[allow(clippy::too_many_arguments)]
fn render_char_with_font<F: RendererFontOps + ?Sized>(
    pixmap: &mut Pixmap,
    px: f32,
    py: f32,
    font_id: FontId,
    ch: char,
    color: &Color,
    em: f32,
    font: &F,
    source_id: OutlineSourceId,
) -> bool {
    let glyph_id = font.glyph_id(ch);
    glyph_id.0 != 0
        && render_glyph_with_font(
            pixmap,
            px,
            py,
            FontGlyph {
                font_id,
                font,
                source_id,
                glyph_id,
            },
            color,
            em,
        )
}

/// Render all items in the DisplayList using the given font cache.
fn render_display_list(
    pixmap: &mut Pixmap,
    display_list: &DisplayList,
    font_cache: &HashMap<FontId, RendererFontRef<'_>>,
    system_fonts: &SystemFontResolver,
    em_px: f32,
    pad_px: f32,
    dpr: f32,
) {
    let mut font_id_cache: HashMap<&str, FontId> = HashMap::new();
    for item in &display_list.items {
        match item {
            DisplayItem::GlyphPath {
                x,
                y,
                scale,
                font,
                char_code,
                color,
            } => {
                let glyph_em = em_px * *scale as f32;
                let font_id = *font_id_cache
                    .entry(font.as_str())
                    .or_insert_with(|| FontId::parse(font).unwrap_or(FontId::MainRegular));
                render_glyph(
                    pixmap,
                    *x as f32 * em_px + pad_px,
                    *y as f32 * em_px + pad_px,
                    font_id,
                    *char_code,
                    color,
                    font_cache,
                    system_fonts,
                    glyph_em,
                );
            }
            DisplayItem::Line {
                x,
                y,
                width,
                thickness,
                color,
                dashed,
            } => {
                render_line(
                    pixmap,
                    *x as f32 * em_px + pad_px,
                    *y as f32 * em_px + pad_px,
                    *width as f32 * em_px,
                    *thickness as f32 * em_px,
                    color,
                    *dashed,
                );
            }
            DisplayItem::Rect {
                x,
                y,
                width,
                height,
                color,
            } => {
                render_rect(
                    pixmap,
                    *x as f32 * em_px + pad_px,
                    *y as f32 * em_px + pad_px,
                    *width as f32 * em_px,
                    *height as f32 * em_px,
                    color,
                );
            }
            DisplayItem::Path {
                x,
                y,
                commands,
                fill,
                color,
            } => {
                render_path(
                    pixmap,
                    *x as f32 * em_px + pad_px,
                    *y as f32 * em_px + pad_px,
                    commands,
                    *fill,
                    color,
                    em_px,
                    1.5 * dpr,
                );
            }
        }
    }
}

/// After `.notdef` or a cmap slot with **no drawable outline** (common for emoji in text fonts),
/// try KaTeX Main → `CjkRegular` → **Emoji** (color font, vector + sbix bitmap) → `CjkFallback`.
///
/// Emoji is tried **before** the broad text fallback so supplementary-plane / color glyphs are not
/// stuck behind Arial-style faces that often lack drawable outlines for emoji.
///
/// When `skip_main_regular` is `true`, skips `Main-Regular` (caller already tried that face).
#[allow(clippy::too_many_arguments)]
fn try_system_unicode_fallback(
    pixmap: &mut Pixmap,
    px: f32,
    py: f32,
    ch: char,
    color: &Color,
    em: f32,
    font_cache: &HashMap<FontId, RendererFontRef<'_>>,
    system_fonts: &SystemFontResolver,
    skip_main_regular: bool,
) -> bool {
    if !skip_main_regular {
        if let Some(fallback) = font_cache.get(&FontId::MainRegular) {
            if render_char_with_entry(pixmap, px, py, FontId::MainRegular, ch, color, em, fallback)
            {
                return true;
            }
        }
    }
    if render_char_with_system_fallback(
        pixmap,
        px,
        py,
        FontId::CjkRegular,
        ch,
        color,
        em,
        font_cache,
        system_fonts,
    ) {
        return true;
    }
    if try_emoji_vector_then_bitmap(pixmap, px, py, ch, color, em, font_cache, system_fonts) {
        return true;
    }
    render_char_with_system_fallback(
        pixmap,
        px,
        py,
        FontId::CjkFallback,
        ch,
        color,
        em,
        font_cache,
        system_fonts,
    )
}

/// Color fonts (e.g. Apple Color Emoji) often expose a minimal `glyf` outline for COLR masking
/// while the visible glyph lives in `sbix` / `CBDT`. `ab_glyph` then "succeeds" with an
/// effectively invisible path — so **raster strike first**, then outline.
#[allow(clippy::too_many_arguments)]
fn try_emoji_vector_then_bitmap(
    pixmap: &mut Pixmap,
    px: f32,
    py: f32,
    ch: char,
    color: &Color,
    em: f32,
    font_cache: &HashMap<FontId, RendererFontRef<'_>>,
    system_fonts: &SystemFontResolver,
) -> bool {
    if !ratex_unicode_font::is_emoji_candidate(ch) {
        return false;
    }
    if try_blit_emoji_raster_fallback(pixmap, px, py, em, ch, color) {
        return true;
    }
    render_char_with_system_fallback(
        pixmap,
        px,
        py,
        FontId::EmojiFallback,
        ch,
        color,
        em,
        font_cache,
        system_fonts,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_glyph(
    pixmap: &mut Pixmap,
    px: f32,
    py: f32,
    font_id: FontId,
    char_code: u32,
    color: &Color,
    font_cache: &HashMap<FontId, RendererFontRef<'_>>,
    system_fonts: &SystemFontResolver,
    em: f32,
) {
    let font_entry = match font_cache.get(&font_id) {
        Some(entry) => entry,
        None => match font_cache.get(&FontId::MainRegular) {
            Some(entry) => entry,
            None => return,
        },
    };

    match &font_entry.font {
        RendererFont::Parsed(font) => render_glyph_from_font(
            pixmap,
            px,
            py,
            font_id,
            char_code,
            color,
            font_cache,
            system_fonts,
            em,
            *font,
            font_entry.source_id,
        ),
        RendererFont::Raw(raw) => {
            if let Some(font) = raw.get() {
                render_glyph_from_font(
                    pixmap,
                    px,
                    py,
                    font_id,
                    char_code,
                    color,
                    font_cache,
                    system_fonts,
                    em,
                    font,
                    font_entry.source_id,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_glyph_from_font<F: RendererFontOps + ?Sized>(
    pixmap: &mut Pixmap,
    px: f32,
    py: f32,
    font_id: FontId,
    char_code: u32,
    color: &Color,
    font_cache: &HashMap<FontId, RendererFontRef<'_>>,
    system_fonts: &SystemFontResolver,
    em: f32,
    font: &F,
    source_id: OutlineSourceId,
) {
    let ch = ratex_font::katex_ttf_glyph_char(font_id, char_code);
    let glyph_id = font.glyph_id(ch);

    if glyph_id.0 == 0 {
        let _ = try_system_unicode_fallback(
            pixmap,
            px,
            py,
            ch,
            color,
            em,
            font_cache,
            system_fonts,
            false,
        );
        return;
    }

    if font_id == FontId::EmojiFallback {
        if try_blit_emoji_raster_fallback(pixmap, px, py, em, ch, color) {
            return;
        }
        let _ = render_glyph_with_font(
            pixmap,
            px,
            py,
            FontGlyph {
                font_id,
                font,
                source_id,
                glyph_id,
            },
            color,
            em,
        );
        return;
    }

    // `RATEX_UNICODE_FONT` may map a codepoint to a non-.notdef glyph with no outlines; try system fallback.
    if font_id == FontId::CjkRegular {
        if render_glyph_with_font(
            pixmap,
            px,
            py,
            FontGlyph {
                font_id: FontId::CjkRegular,
                font,
                source_id,
                glyph_id,
            },
            color,
            em,
        ) {
            return;
        }
        if try_emoji_vector_then_bitmap(pixmap, px, py, ch, color, em, font_cache, system_fonts) {
            return;
        }
        if render_char_with_system_fallback(
            pixmap,
            px,
            py,
            FontId::CjkFallback,
            ch,
            color,
            em,
            font_cache,
            system_fonts,
        ) {
            return;
        }
        return;
    }

    if font_id == FontId::CjkFallback {
        if render_glyph_with_font(
            pixmap,
            px,
            py,
            FontGlyph {
                font_id: FontId::CjkFallback,
                font,
                source_id,
                glyph_id,
            },
            color,
            em,
        ) {
            return;
        }
        let _ =
            try_emoji_vector_then_bitmap(pixmap, px, py, ch, color, em, font_cache, system_fonts);
        return;
    }

    if render_glyph_with_font(
        pixmap,
        px,
        py,
        FontGlyph {
            font_id,
            font,
            source_id,
            glyph_id,
        },
        color,
        em,
    ) {
        return;
    }
    // cmap had a non-zero GID but no `glyf` outline (e.g. blank text-font slot for emoji).
    let skip_main = font_id == FontId::MainRegular;
    let _ = try_system_unicode_fallback(
        pixmap,
        px,
        py,
        ch,
        color,
        em,
        font_cache,
        system_fonts,
        skip_main,
    );
}

struct FontGlyph<'a, F: ?Sized> {
    font_id: FontId,
    font: &'a F,
    source_id: OutlineSourceId,
    glyph_id: ab_glyph::GlyphId,
}

struct RasterGlyphParams {
    px: f32,
    py: f32,
    em: f32,
    ch: char,
    opacity: f32,
}

/// Cache key for decoded color-emoji raster strikes.
///
/// The font bytes are held in shared immutable storage inside `ratex-unicode-font`, so the
/// pointer/length pair is stable for the process lifetime and avoids cloning
/// or hashing the font data on every lookup. This cache must only be fed from
/// that process-lifetime `OnceLock<FontData>`; transient buffers could be
/// freed and their address reused by a different font.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EmojiRasterCacheKey {
    font_ptr: usize,
    font_len: usize,
    face_index: u32,
    glyph_id: u16,
    pixels_per_em: u16,
}

struct CachedEmojiRaster {
    pixmap: Arc<Pixmap>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    pixels_per_em: f32,
}

#[derive(Default)]
struct EmojiRasterCache {
    entries: HashMap<EmojiRasterCacheKey, Arc<CachedEmojiRaster>>,
    /// Decoded RGBA pixel bytes retained by `entries`.
    bytes: usize,
}

impl EmojiRasterCache {
    fn clear(&mut self) {
        self.entries.clear();
        self.bytes = 0;
    }

    fn insert_with_limits(
        &mut self,
        key: EmojiRasterCacheKey,
        entry: Arc<CachedEmojiRaster>,
        entry_cap: usize,
        byte_cap: usize,
    ) {
        // Another thread may have decoded the same strike while this caller
        // was working. A duplicate must not evict the existing hot entry.
        if self.entries.contains_key(&key) {
            return;
        }

        let entry_bytes = entry.pixmap.data().len();
        // Do not flush useful entries for a single raster that cannot fit in
        // the cache by itself.
        if entry_bytes > byte_cap {
            return;
        }
        if self.entries.len() >= entry_cap || self.bytes.saturating_add(entry_bytes) > byte_cap {
            self.clear();
        }
        self.bytes += entry_bytes;
        self.entries.insert(key, entry);
    }
}

static EMOJI_RASTER_CACHE: LazyLock<RwLock<EmojiRasterCache>> =
    LazyLock::new(|| RwLock::new(EmojiRasterCache::default()));

/// Upper bounds on cached decoded emoji strikes. The entry cap limits key
/// diversity while the byte cap bounds the decoded RGBA pixel memory retained
/// by large color-font strikes.
const EMOJI_RASTER_CACHE_CAP: usize = 4096;
const EMOJI_RASTER_CACHE_BYTE_CAP: usize = 64 * 1024 * 1024;

/// Cache key for rasterized outline-glyph masks.
///
/// Rasterizing a glyph outline (curve flattening + anti-aliased scanline fill)
/// is the dominant PNG render cost and scales with the outline's curve count,
/// not its pixel area. Repeated renders of the same formula (live preview,
/// batch re-renders, benchmarks) rasterize the same (font, glyph, size,
/// position-phase, color) combinations, so the rasterized result is cached and
/// later draws become plain pixel blits.
///
/// Glyph size and the sub-pixel phase of the glyph position are part of the
/// key **exactly** (as float bit patterns), so every cache hit reproduces the
/// first caller's rasterization pixel-for-pixel: the same (font, glyph, size,
/// phase) combination yields the same anti-aliased coverage regardless of the
/// integer part of the position. Glyphs at integer-aligned positions
/// additionally share masks within a single formula.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct GlyphMaskKey {
    source: OutlineSourceId,
    font_id: FontId,
    glyph_id: ab_glyph::GlyphId,
    /// Exact `em` size as `f32` bits.
    size_bits: u32,
    /// Exact fractional x phase of `px` as `f32` bits.
    frac_x: u32,
    /// Exact fractional y phase of `py` as `f32` bits.
    frac_y: u32,
    color: u32,
}

/// Upper bound on cached glyph masks. The entry cap limits key diversity while
/// the byte cap bounds the actual pixel memory retained by the cache.
const GLYPH_MASK_CACHE_CAP: usize = 8192;
const GLYPH_MASK_CACHE_BYTE_CAP: usize = 64 * 1024 * 1024;

struct GlyphMaskCache {
    entries: HashMap<GlyphMaskKey, Arc<Pixmap>>,
    bytes: usize,
}

static GLYPH_MASK_CACHE: LazyLock<RwLock<GlyphMaskCache>> = LazyLock::new(|| {
    RwLock::new(GlyphMaskCache {
        entries: HashMap::new(),
        bytes: 0,
    })
});

fn pack_color_u32(color: &Color) -> u32 {
    let [r, g, b, a] = color_to_rgba8(color);
    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | a as u32
}

/// tiny-skia's low-precision pipeline computes `div255(v)` as `(v + 255) >> 8`.
///
/// The pre-cache renderer used `Pixmap::fill_path` directly, whose solid-color
/// anti-aliasing pipeline uses this exact arithmetic. The glyph-mask cache
/// stores the rasterized source and then composites it manually, so the blit
/// path must reproduce this rounding exactly to remain pixel-identical with
/// `fill_path`.
#[inline]
fn div255(v: u32) -> u8 {
    ((v + 255) >> 8) as u8
}

/// Composite one premultiplied mask pixel over a destination pixel using the
/// same arithmetic as tiny-skia's solid-color `fill_path` pipeline.
///
/// `paint_rgba` is the straight, quantized RGBA8 paint color that produced
/// `mask`. For an opaque paint, tiny-skia strength-reduces `SourceOver` to
/// `Source` and antialiased pixels are `lerp(dst, src, coverage)`; in that
/// case `mask.alpha` is exactly the coverage value. For a non-opaque paint,
/// the path pipeline pre-scales the source by coverage, and the stored mask
/// pixel already equals that pre-scaled source, so the remaining operation is
/// lowp `SourceOver`: `src + div255(dst * inv(src.a))`.
fn blend_mask_over(
    src: tiny_skia::PremultipliedColorU8,
    dst: tiny_skia::PremultipliedColorU8,
    paint_rgba: [u8; 4],
) -> tiny_skia::PremultipliedColorU8 {
    let out = if paint_rgba[3] == 255 {
        let coverage = src.alpha() as u32;
        let inv = 255 - coverage;
        [
            div255(dst.red() as u32 * inv + paint_rgba[0] as u32 * coverage),
            div255(dst.green() as u32 * inv + paint_rgba[1] as u32 * coverage),
            div255(dst.blue() as u32 * inv + paint_rgba[2] as u32 * coverage),
            div255(dst.alpha() as u32 * inv + 255 * coverage),
        ]
    } else {
        let inv = 255 - src.alpha() as u32;
        [
            (src.red() as u32 + ((dst.red() as u32 * inv + 255) >> 8)) as u8,
            (src.green() as u32 + ((dst.green() as u32 * inv + 255) >> 8)) as u8,
            (src.blue() as u32 + ((dst.blue() as u32 * inv + 255) >> 8)) as u8,
            (src.alpha() as u32 + ((dst.alpha() as u32 * inv + 255) >> 8)) as u8,
        ]
    };
    tiny_skia::PremultipliedColorU8::from_rgba(out[0], out[1], out[2], out[3])
        .expect("glyph mask source-over blend must stay premultiplied")
}

/// Draw a cached glyph mask at the given integer anchor, reproducing the
/// pre-cache `fill_path` compositing arithmetic pixel-for-pixel.
fn blit_glyph_mask(
    pixmap: &mut Pixmap,
    mask: &Pixmap,
    dst_x: i32,
    dst_y: i32,
    paint_rgba: [u8; 4],
) {
    let dst_right = (dst_x as i64 + mask.width() as i64)
        .min(pixmap.width() as i64)
        .max(0) as u32;
    let dst_bottom = (dst_y as i64 + mask.height() as i64)
        .min(pixmap.height() as i64)
        .max(0) as u32;
    let dst_left = (dst_x as i64).max(0) as u32;
    let dst_top = (dst_y as i64).max(0) as u32;
    if dst_left >= dst_right || dst_top >= dst_bottom {
        return;
    }

    let dst_width = pixmap.width() as usize;
    let dst_pixels = pixmap.pixels_mut();
    let src_pixels = mask.pixels();
    for y in dst_top..dst_bottom {
        let src_y = y as i64 - dst_y as i64;
        for x in dst_left..dst_right {
            let src_x = x as i64 - dst_x as i64;
            let src_idx = src_y as usize * mask.width() as usize + src_x as usize;
            let src = src_pixels[src_idx];
            if src.alpha() == 0 {
                continue;
            }
            let dst_idx = y as usize * dst_width + x as usize;
            dst_pixels[dst_idx] = blend_mask_over(src, dst_pixels[dst_idx], paint_rgba);
        }
    }
}

/// Bounding box of all outline points transformed to device space.
///
/// Matches tiny-skia `Path::bounds()` (control-point hull) for the path this
/// function's caller builds from the same curves, so mask anchors are
/// consistent between cache hits (bbox only) and misses (built path).
fn outline_bbox(
    curves: &[ab_glyph::OutlineCurve],
    px: f32,
    py: f32,
    scale: f32,
) -> (f32, f32, f32, f32) {
    use ab_glyph::OutlineCurve;
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    );
    let mut acc = |p: ab_glyph::Point| {
        let x = px + p.x * scale;
        let y = py - p.y * scale;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    };
    for curve in curves {
        match curve {
            OutlineCurve::Line(p0, p1) => {
                acc(*p0);
                acc(*p1);
            }
            OutlineCurve::Quad(p0, p1, p2) => {
                acc(*p0);
                acc(*p1);
                acc(*p2);
            }
            OutlineCurve::Cubic(p0, p1, p2, p3) => {
                acc(*p0);
                acc(*p1);
                acc(*p2);
                acc(*p3);
            }
        }
    }
    (min_x, min_y, max_x, max_y)
}

fn build_outline_path(
    curves: &[ab_glyph::OutlineCurve],
    px: f32,
    py: f32,
    scale: f32,
) -> Option<tiny_skia::Path> {
    let mut builder = PathBuilder::new();
    let mut last_end: Option<(f32, f32)> = None;

    for curve in curves {
        use ab_glyph::OutlineCurve;
        let (start, end) = match curve {
            OutlineCurve::Line(p0, p1) => (
                (px + p0.x * scale, py - p0.y * scale),
                (px + p1.x * scale, py - p1.y * scale),
            ),
            OutlineCurve::Quad(p0, _, p2) => (
                (px + p0.x * scale, py - p0.y * scale),
                (px + p2.x * scale, py - p2.y * scale),
            ),
            OutlineCurve::Cubic(p0, _, _, p3) => (
                (px + p0.x * scale, py - p0.y * scale),
                (px + p3.x * scale, py - p3.y * scale),
            ),
        };

        let need_move = match last_end {
            None => true,
            Some((lx, ly)) => (lx - start.0).abs() > 0.01 || (ly - start.1).abs() > 0.01,
        };
        if need_move {
            if last_end.is_some() {
                builder.close();
            }
            builder.move_to(start.0, start.1);
        }

        match curve {
            OutlineCurve::Line(_, p1) => {
                builder.line_to(px + p1.x * scale, py - p1.y * scale);
            }
            OutlineCurve::Quad(_, p1, p2) => {
                builder.quad_to(
                    px + p1.x * scale,
                    py - p1.y * scale,
                    px + p2.x * scale,
                    py - p2.y * scale,
                );
            }
            OutlineCurve::Cubic(_, p1, p2, p3) => {
                builder.cubic_to(
                    px + p1.x * scale,
                    py - p1.y * scale,
                    px + p2.x * scale,
                    py - p2.y * scale,
                    px + p3.x * scale,
                    py - p3.y * scale,
                );
            }
        }
        last_end = Some(end);
    }

    if last_end.is_some() {
        builder.close();
    }
    builder.finish()
}

fn glyph_mask_fully_inside(
    pixmap: &Pixmap,
    dst_x: i32,
    dst_y: i32,
    mask_w: u32,
    mask_h: u32,
) -> bool {
    let dst_x = i64::from(dst_x);
    let dst_y = i64::from(dst_y);
    dst_x >= 0
        && dst_y >= 0
        && dst_x + i64::from(mask_w) <= i64::from(pixmap.width())
        && dst_y + i64::from(mask_h) <= i64::from(pixmap.height())
}

fn render_glyph_with_font<F: RendererFontOps + ?Sized>(
    pixmap: &mut Pixmap,
    px: f32,
    py: f32,
    g: FontGlyph<'_, F>,
    color: &Color,
    em: f32,
) -> bool {
    let curves = match g.font.cached_outline(g.font_id, g.source_id, g.glyph_id) {
        Some(c) => c,
        None => return false,
    };
    if curves.is_empty() {
        return false;
    }

    let units_per_em = g.font.units_per_em().unwrap_or(1000.0);
    let mut scale = em / units_per_em;

    // Emoji outline fallback has no KaTeX metrics; scale it to the 1.0em width that layout
    // allocates for missing emoji so Windows vector fallback does not overflow.
    if g.font_id == FontId::EmojiFallback {
        let actual_advance = g.font.h_advance_unscaled(g.glyph_id);
        let actual_advance_em = actual_advance / units_per_em;
        let assumed_width = 1.0;
        if actual_advance_em > 0.01 && actual_advance_em > assumed_width * 1.01 {
            scale *= assumed_width / actual_advance_em;
        }
    }

    let paint_rgba = color_to_rgba8(color);

    // Key includes the exact size and sub-pixel phase so a cache hit is
    // guaranteed to reproduce pixel-identical anti-aliased coverage.
    let cache_key = GlyphMaskKey {
        source: g.source_id,
        font_id: g.font_id,
        glyph_id: g.glyph_id,
        size_bits: em.to_bits(),
        frac_x: px.fract().to_bits(),
        frac_y: py.fract().to_bits(),
        color: pack_color_u32(color),
    };

    // Mask geometry: bounds of all outline points (same point set as the path
    // built below) plus a 1 px anti-aliasing margin, anchored at an integer
    // device position.
    let (min_x, min_y, max_x, max_y) = outline_bbox(&curves, px, py, scale);
    let left = min_x.floor() - 1.0;
    let top = min_y.floor() - 1.0;
    let mask_w = ((max_x.ceil() + 1.0) - left).max(1.0) as u32;
    let mask_h = ((max_y.ceil() + 1.0) - top).max(1.0) as u32;
    let dst_x = left as i32;
    let dst_y = top as i32;
    let cacheable = glyph_mask_fully_inside(pixmap, dst_x, dst_y, mask_w, mask_h);

    if cacheable {
        let cache = GLYPH_MASK_CACHE.read().unwrap();
        if let Some(mask) = cache.entries.get(&cache_key) {
            let mask = Arc::clone(mask);
            drop(cache);
            blit_glyph_mask(pixmap, &mask, dst_x, dst_y, paint_rgba);
            return true;
        }
    }

    let Some(path) = build_outline_path(&curves, px, py, scale) else {
        return false;
    };

    if !cacheable {
        // tiny-skia's antialiasing at a destination edge is not always
        // pixel-identical to rasterizing the full glyph offscreen and clipping
        // during the blit. Preserve the direct-render result for clipped
        // glyphs; they are uncommon and cannot safely share an offscreen mask.
        let mut paint = paint_for_color(color);
        paint.anti_alias = true;
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
        return true;
    }

    // Rasterize into a mask pixmap sized to the glyph's bounds plus a 1 px
    // anti-aliasing margin. The path is translated by the mask's integer
    // anchor so the stored pixels are position-independent; the blit at
    // (dst_x, dst_y) restores the device position.
    let Some(mut mask) = Pixmap::new(mask_w, mask_h) else {
        return false;
    };
    mask.fill(tiny_skia::Color::TRANSPARENT);

    let mut paint = paint_for_color(color);
    paint.anti_alias = true;
    mask.fill_path(
        &path,
        &paint,
        tiny_skia::FillRule::Winding,
        Transform::from_translate(-left, -top),
        None,
    );

    blit_glyph_mask(pixmap, &mask, dst_x, dst_y, paint_rgba);

    // Insert without replacing an existing entry: another thread may have
    // rasterized the same glyph while we were working.
    let entry = Arc::new(mask);
    let entry_bytes = entry.data().len();
    let mut cache = GLYPH_MASK_CACHE.write().unwrap();
    if !cache.entries.contains_key(&cache_key) && entry_bytes <= GLYPH_MASK_CACHE_BYTE_CAP {
        if cache.entries.len() >= GLYPH_MASK_CACHE_CAP
            || cache.bytes.saturating_add(entry_bytes) > GLYPH_MASK_CACHE_BYTE_CAP
        {
            cache.entries.clear();
            cache.bytes = 0;
        }
        cache.bytes += entry_bytes;
        cache.entries.insert(cache_key, entry);
    }
    true
}

/// Color emoji (sbix / CBDT / etc.) often have no `glyf` outlines; `ttf-parser` embedded strikes + PNG.
fn try_blit_emoji_raster_fallback(
    pixmap: &mut Pixmap,
    px: f32,
    py: f32,
    em: f32,
    ch: char,
    color: &Color,
) -> bool {
    let Some(bytes) = ratex_unicode_font::load_emoji_font_data() else {
        return false;
    };
    let idx = ratex_unicode_font::emoji_font_face_index().unwrap_or(0);
    try_blit_raster_glyph(
        pixmap,
        RasterGlyphParams {
            px,
            py,
            em,
            ch,
            opacity: normalized_alpha(color.a),
        },
        bytes.as_slice(),
        idx,
    )
}

fn try_blit_raster_glyph(
    pixmap: &mut Pixmap,
    params: RasterGlyphParams,
    font_bytes: &[u8],
    face_index: u32,
) -> bool {
    let face = match ttf_parser::Face::parse(font_bytes, face_index) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let gid = match face.glyph_index(params.ch) {
        Some(g) => g,
        None => return false,
    };
    let requested_strike = params.em.round().clamp(8.0, 256.0) as u16;
    let img = face
        .glyph_raster_image(gid, requested_strike)
        .or_else(|| face.glyph_raster_image(gid, u16::MAX));
    let Some(img) = img else {
        return false;
    };
    // Multiple requested sizes can resolve to the same bitmap strike. Cache by
    // the strike that was actually returned so those requests share one decoded
    // pixmap; the requested `params.em` is still applied when drawing below.
    let key = EmojiRasterCacheKey {
        font_ptr: font_bytes.as_ptr() as usize,
        font_len: font_bytes.len(),
        face_index,
        glyph_id: gid.0,
        pixels_per_em: img.pixels_per_em,
    };
    let cached_entry = {
        let cache = EMOJI_RASTER_CACHE.read().unwrap();
        cache.entries.get(&key).cloned()
    };
    if let Some(entry) = cached_entry {
        return blit_cached_emoji_raster(pixmap, &params, &entry);
    }

    let glyph_pm = match raster_glyph_image_to_pixmap(&img) {
        Some(p) => p,
        None => return false,
    };
    let entry = Arc::new(CachedEmojiRaster {
        pixmap: Arc::new(glyph_pm),
        x: f32::from(img.x),
        y: f32::from(img.y),
        width: f32::from(img.width),
        height: f32::from(img.height),
        pixels_per_em: f32::from(img.pixels_per_em.max(1)),
    });
    let result = blit_cached_emoji_raster(pixmap, &params, &entry);

    // Insert without replacing an existing entry: another thread may have
    // decoded the same strike while we were working.
    let mut cache = EMOJI_RASTER_CACHE.write().unwrap();
    cache.insert_with_limits(
        key,
        entry,
        EMOJI_RASTER_CACHE_CAP,
        EMOJI_RASTER_CACHE_BYTE_CAP,
    );
    result
}

/// Draw a decoded emoji raster strike, using the same geometry as the
/// uncached path in [`try_blit_raster_glyph`].
fn blit_cached_emoji_raster(
    pixmap: &mut Pixmap,
    params: &RasterGlyphParams,
    entry: &CachedEmojiRaster,
) -> bool {
    let ppm = entry.pixels_per_em.max(1.0);
    let mut scale = params.em / ppm;
    // Scale emoji to fit 1.0em layout width if it's wider (prevents overflow).
    let actual_width_em = entry.width / ppm;
    let assumed_width = 1.0;
    if actual_width_em > 0.01 && actual_width_em > assumed_width * 1.01 {
        scale *= assumed_width / actual_width_em;
    }
    let top_x = params.px + entry.x * scale;
    // `ttf-parser` / OpenType: `RasterGlyphImage::{x,y}` are in strike pixels; `y` is the
    // **bottom** edge of the bitmap in y-up coordinates (sbix yOffset to bottom; CBDT normalized
    // the same way). Top edge = y + height — using `y` alone shifts the glyph down by ~full height.
    let mut top_y = params.py - (entry.y + entry.height) * scale;
    // sbix places the bitmap bottom on the math baseline, but tall (~1em) color strikes put the
    // ink centroid near 0.5em above baseline. Binary/relation glyphs (+, =) are centered on the
    // math axis (~0.25em). Nudge the bitmap so its vertical center matches the axis — matches
    // mixed `\text{emoji} … formula` rows without changing layout baselines.
    let center_strike = (entry.y + entry.height / 2.0) / ppm;
    let axis = ratex_font::get_global_metrics(0).axis_height as f32;
    top_y += (center_strike - axis) * params.em;
    let paint = PixmapPaint {
        opacity: params.opacity,
        quality: FilterQuality::Bilinear,
        ..Default::default()
    };
    let transform = Transform::from_row(scale, 0.0, 0.0, scale, top_x, top_y);
    pixmap.draw_pixmap(0, 0, (*entry.pixmap).as_ref(), &paint, transform, None);
    true
}

#[allow(clippy::chunks_exact_to_as_chunks)]
fn raster_glyph_image_to_pixmap(img: &ttf_parser::RasterGlyphImage<'_>) -> Option<Pixmap> {
    use ttf_parser::RasterImageFormat;
    let w = u32::from(img.width);
    let h = u32::from(img.height);
    let size = tiny_skia::IntSize::from_wh(w, h)?;
    match img.format {
        RasterImageFormat::PNG => Pixmap::decode_png(img.data).ok(),
        RasterImageFormat::BitmapPremulBgra32 => {
            let expected = 4usize * w as usize * h as usize;
            if img.data.len() != expected {
                return None;
            }
            let mut v = Vec::with_capacity(expected);
            for px in img.data.chunks_exact(4) {
                let b = px[0];
                let g = px[1];
                let r = px[2];
                let a = px[3];
                v.extend_from_slice(&[r, g, b, a]);
            }
            Pixmap::from_vec(v, size)
        }
        RasterImageFormat::BitmapGray8 => {
            let mut v = Vec::with_capacity(4 * img.data.len());
            for &g in img.data {
                v.extend_from_slice(&[g, g, g, 255]);
            }
            Pixmap::from_vec(v, size)
        }
        _ => None,
    }
}

fn render_line(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    width: f32,
    thickness: f32,
    color: &Color,
    dashed: bool,
) {
    let t = thickness.max(1.0);
    let paint = paint_for_color(color);

    if dashed {
        // Draw a dashed line: dash length = 4t, gap = 4t.
        let dash_len = (4.0 * t).max(2.0);
        let gap_len = (4.0 * t).max(2.0);
        let period = dash_len + gap_len;
        let top = y - t / 2.0;
        let mut cur_x = x;
        while cur_x < x + width {
            let seg_width = (dash_len).min(x + width - cur_x);
            let seg_width = seg_width.max(2.0);
            if let Some(rect) = tiny_skia::Rect::from_xywh(cur_x, top, seg_width, t) {
                pixmap.fill_rect(rect, &paint, Transform::identity(), None);
            }
            cur_x += period;
        }
    } else if let Some(rect) = tiny_skia::Rect::from_xywh(x, y - t / 2.0, width, t) {
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

fn render_rect(pixmap: &mut Pixmap, x: f32, y: f32, width: f32, height: f32, color: &Color) {
    // tiny-skia's fill_rect fast path requires a full interior pixel. Preserve
    // sub-2px TeX rules by routing them through the anti-aliased path filler.
    if width < 2.0 || height < 2.0 {
        let Some(rect) = tiny_skia::Rect::from_xywh(x, y, width, height) else {
            return;
        };
        let path = PathBuilder::from_rect(rect);
        let mut paint = paint_for_color(color);
        paint.anti_alias = true;
        pixmap.fill_path(
            &path,
            &paint,
            FillRule::Winding,
            Transform::identity(),
            None,
        );
        return;
    }
    let rect = tiny_skia::Rect::from_xywh(x, y, width, height);
    if let Some(rect) = rect {
        let paint = paint_for_color(color);
        pixmap.fill_rect(rect, &paint, Transform::identity(), None);
    }
}

#[allow(clippy::too_many_arguments)]
fn render_path(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    commands: &[ratex_types::path_command::PathCommand],
    fill: bool,
    color: &Color,
    em: f32,
    stroke_width_px: f32,
) {
    // For filled paths, render each subpath (delimited by MoveTo) as a separate
    // fill_path call.  KaTeX stretchy arrows are assembled from multiple path
    // components (e.g. "lefthook" + "rightarrow") whose winding directions can
    // be opposite.  Combining them into a single fill_path with FillRule::Winding
    // causes the shaft region to cancel out (net winding = 0 → unfilled).
    // Drawing each subpath independently avoids cross-component winding interactions.
    if fill {
        let mut start = 0;
        for i in 1..commands.len() {
            if matches!(
                commands[i],
                ratex_types::path_command::PathCommand::MoveTo { .. }
            ) {
                render_path_segment(
                    pixmap,
                    x,
                    y,
                    &commands[start..i],
                    fill,
                    color,
                    em,
                    stroke_width_px,
                );
                start = i;
            }
        }
        render_path_segment(
            pixmap,
            x,
            y,
            &commands[start..],
            fill,
            color,
            em,
            stroke_width_px,
        );
        return;
    }
    render_path_segment(pixmap, x, y, commands, fill, color, em, stroke_width_px);
}

#[allow(clippy::too_many_arguments)]
fn render_path_segment(
    pixmap: &mut Pixmap,
    x: f32,
    y: f32,
    commands: &[ratex_types::path_command::PathCommand],
    fill: bool,
    color: &Color,
    em: f32,
    stroke_width_px: f32,
) {
    let mut builder = PathBuilder::new();
    for cmd in commands {
        match cmd {
            ratex_types::path_command::PathCommand::MoveTo { x: cx, y: cy } => {
                builder.move_to(x + *cx as f32 * em, y + *cy as f32 * em);
            }
            ratex_types::path_command::PathCommand::LineTo { x: cx, y: cy } => {
                builder.line_to(x + *cx as f32 * em, y + *cy as f32 * em);
            }
            ratex_types::path_command::PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x: cx,
                y: cy,
            } => {
                builder.cubic_to(
                    x + *x1 as f32 * em,
                    y + *y1 as f32 * em,
                    x + *x2 as f32 * em,
                    y + *y2 as f32 * em,
                    x + *cx as f32 * em,
                    y + *cy as f32 * em,
                );
            }
            ratex_types::path_command::PathCommand::QuadTo {
                x1,
                y1,
                x: cx,
                y: cy,
            } => {
                builder.quad_to(
                    x + *x1 as f32 * em,
                    y + *y1 as f32 * em,
                    x + *cx as f32 * em,
                    y + *cy as f32 * em,
                );
            }
            ratex_types::path_command::PathCommand::Close => {
                builder.close();
            }
        }
    }

    if let Some(path) = builder.finish() {
        let mut paint = paint_for_color(color);
        if fill {
            paint.anti_alias = true;
            // Even-odd: KaTeX `tallDelim` vert uses two subpaths (outline + stem); nonzero winding
            // double-fills the stem and inflates ink vs reference PNGs.
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::EvenOdd,
                Transform::identity(),
                None,
            );
        } else {
            let stroke = Stroke {
                width: stroke_width_px,
                ..Default::default()
            };
            pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
        }
    }
}

/// Convert tiny-skia's premultiplied RGBA pixels to straight RGBA for PNG.
///
/// This mirrors `tiny_skia::Pixmap::encode_png`, which demultiplies with the
/// same `value / alpha + 0.5` rounding before encoding.
#[allow(clippy::chunks_exact_to_as_chunks)]
fn demultiply_rgba(data: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    for px in out.chunks_exact_mut(4) {
        let alpha = px[3];
        if alpha == 0 {
            px[0] = 0;
            px[1] = 0;
            px[2] = 0;
        } else if alpha != 255 {
            let a = alpha as f64 / 255.0;
            px[0] = ((px[0] as f64 / a) + 0.5) as u8;
            px[1] = ((px[1] as f64 / a) + 0.5) as u8;
            px[2] = ((px[2] as f64 / a) + 0.5) as u8;
        }
    }
    out
}

fn encode_png(pixmap: &Pixmap) -> Result<Vec<u8>, String> {
    // `Pixmap::encode_png` clones the whole pixmap before demultiplying it.
    // We still need one demultiplying copy, but can write the PNG rows directly
    // from that buffer. png 0.17 already defaults to `Compression::Fast`,
    // `FilterType::Sub`, and non-adaptive filtering; the explicit settings
    // below keep that contract even if the defaults change.
    let data = demultiply_rgba(pixmap.data());
    // Grow with the compressed stream instead of reserving one byte per raw
    // pixel. Formula PNGs are highly compressible, so a width*height reserve
    // can add tens of MiB to peak RSS for large images.
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, pixmap.width(), pixmap.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Fast);
        encoder.set_adaptive_filter(png::AdaptiveFilterType::NonAdaptive);
        encoder.set_filter(png::FilterType::Sub);
        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("PNG encode error: {e}"))?;
        writer
            .write_image_data(&data)
            .map_err(|e| format!("PNG encode error: {e}"))?;
    }
    // Trim the encoder's normal growth slack before returning. Starting from
    // an empty Vec keeps peak output-buffer memory proportional to compressed
    // data rather than raw pixel count.
    out.shrink_to_fit();
    Ok(out)
}

#[cfg(test)]
mod glyph_mask_cache_tests {
    use super::*;

    #[test]
    fn mask_blit_matches_direct_fill_path_for_opaque_and_alpha_paint() {
        let w = 32;
        let h = 32;
        let backgrounds: [[u8; 4]; 5] = [
            [255, 255, 255, 255],
            [238, 238, 238, 255],
            [17, 17, 17, 255],
            [0, 0, 0, 0],
            [51, 102, 153, 128],
        ];
        let colors: [[u8; 4]; 4] = [
            [0, 0, 0, 255],
            [51, 102, 153, 255],
            [51, 102, 153, 128],
            [255, 0, 0, 128],
        ];

        let mut pb = PathBuilder::new();
        pb.move_to(2.3, 3.7);
        pb.line_to(20.7, 2.9);
        pb.line_to(16.2, 22.6);
        pb.line_to(5.1, 17.2);
        pb.close();
        let path = pb.finish().expect("path");

        for bg in backgrounds {
            let mut base = Pixmap::new(w, h).unwrap();
            base.fill(tiny_skia::Color::from_rgba8(bg[0], bg[1], bg[2], bg[3]));
            for rgba in colors {
                let mut direct = base.clone();
                let mut paint = Paint::default();
                paint.set_color_rgba8(rgba[0], rgba[1], rgba[2], rgba[3]);
                paint.anti_alias = true;
                direct.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );

                let bounds = path.bounds();
                let left = bounds.left().floor() - 1.0;
                let top = bounds.top().floor() - 1.0;
                let mask_w = ((bounds.right().ceil() + 1.0) - left).max(1.0) as u32;
                let mask_h = ((bounds.bottom().ceil() + 1.0) - top).max(1.0) as u32;
                let mut mask = Pixmap::new(mask_w, mask_h).unwrap();
                mask.fill(tiny_skia::Color::TRANSPARENT);
                mask.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    Transform::from_translate(-left, -top),
                    None,
                );

                let mut cached = base.clone();
                blit_glyph_mask(&mut cached, &mask, left as i32, top as i32, rgba);
                assert_eq!(
                    direct.data(),
                    cached.data(),
                    "mask blit differs from direct fill_path for background {bg:?} and color {rgba:?}"
                );
            }
        }
    }

    #[test]
    fn encoded_png_returns_compact_buffer() {
        let opts = RenderOptions {
            font_dir: font_dir(),
            font_size: 300.0,
            ..RenderOptions::default()
        };
        let ast = ratex_parser::parser::parse("x").expect("parse");
        let layout = ratex_layout::layout(&ast, &ratex_layout::LayoutOptions::default());
        let dl = ratex_layout::to_display_list(&layout);
        let png = render_to_png(&dl, &opts).expect("render");
        assert!(
            png.capacity() <= png.len() * 2,
            "returned PNG buffer should not retain its large preallocation (len {}, capacity {})",
            png.len(),
            png.capacity()
        );
    }

    #[test]
    fn glyph_mask_color_key_uses_paint_quantization() {
        // These two values both round to byte 1, but the paint path truncates
        // them to 0 and 1 respectively. The cache key must use the paint
        // quantization, otherwise the second glyph could reuse the first
        // glyph's mask and render the wrong red channel.
        let truncates_to_zero = Color::new(0.002, 0.0, 0.0, 1.0);
        let truncates_to_one = Color::new(0.0058431374, 0.0, 0.0, 1.0);
        assert_ne!(
            pack_color_u32(&truncates_to_zero),
            pack_color_u32(&truncates_to_one)
        );
        assert_eq!(color_to_rgba8(&truncates_to_zero)[0], 0);
        assert_eq!(color_to_rgba8(&truncates_to_one)[0], 1);
    }

    #[test]
    fn emoji_requests_resolving_to_one_strike_share_one_cache_entry() {
        let Some(font_bytes) = ratex_unicode_font::load_emoji_font_data() else {
            return;
        };
        let face_index = ratex_unicode_font::emoji_font_face_index().unwrap_or(0);
        let Ok(face) = ttf_parser::Face::parse(font_bytes.as_slice(), face_index) else {
            return;
        };
        let ch = '😊';
        let Some(gid) = face.glyph_index(ch) else {
            return;
        };

        let mut first_request_by_ppem = HashMap::new();
        let mut duplicate_requests = None;
        for requested in 8_u16..=256 {
            let image = face
                .glyph_raster_image(gid, requested)
                .or_else(|| face.glyph_raster_image(gid, u16::MAX));
            let Some(image) = image else {
                continue;
            };
            if let Some(previous) = first_request_by_ppem.insert(image.pixels_per_em, requested) {
                duplicate_requests = Some((previous, requested));
                break;
            }
        }
        let Some((first_request, second_request)) = duplicate_requests else {
            return;
        };

        EMOJI_RASTER_CACHE.write().unwrap().clear();
        let mut pixmap = Pixmap::new(512, 512).unwrap();
        for requested in [first_request, second_request] {
            assert!(try_blit_raster_glyph(
                &mut pixmap,
                RasterGlyphParams {
                    px: 128.0,
                    py: 256.0,
                    em: f32::from(requested),
                    ch,
                    opacity: 1.0,
                },
                font_bytes.as_slice(),
                face_index,
            ));
        }

        let cache = EMOJI_RASTER_CACHE.read().unwrap();
        let matching_entries = cache
            .entries
            .keys()
            .filter(|key| {
                key.font_ptr == font_bytes.as_ptr() as usize
                    && key.font_len == font_bytes.len()
                    && key.face_index == face_index
                    && key.glyph_id == gid.0
            })
            .count();
        assert_eq!(
            matching_entries, 1,
            "requests {first_request} and {second_request} resolved to one strike"
        );
    }

    #[test]
    fn emoji_raster_cache_enforces_decoded_byte_budget() {
        fn key(glyph_id: u16) -> EmojiRasterCacheKey {
            EmojiRasterCacheKey {
                font_ptr: 1,
                font_len: 1,
                face_index: 0,
                glyph_id,
                pixels_per_em: 16,
            }
        }

        fn entry(width: u32, height: u32) -> Arc<CachedEmojiRaster> {
            Arc::new(CachedEmojiRaster {
                pixmap: Arc::new(Pixmap::new(width, height).unwrap()),
                x: 0.0,
                y: 0.0,
                width: width as f32,
                height: height as f32,
                pixels_per_em: 16.0,
            })
        }

        let mut cache = EmojiRasterCache::default();
        let first = entry(2, 2);
        let entry_bytes = first.pixmap.data().len();
        cache.insert_with_limits(key(1), first, 4, entry_bytes + 1);
        assert_eq!(cache.bytes, entry_bytes);

        // The second raster would exceed the byte budget, so the old entry is
        // evicted and the cache remains within budget.
        let second = entry(2, 2);
        cache.insert_with_limits(key(2), Arc::clone(&second), 4, entry_bytes + 1);
        assert_eq!(cache.entries.len(), 1);
        assert!(cache.entries.contains_key(&key(2)));
        assert_eq!(cache.bytes, entry_bytes);

        // A duplicate insertion at the entry cap must preserve the hot value.
        cache.insert_with_limits(key(2), entry(2, 2), 1, entry_bytes + 1);
        assert!(Arc::ptr_eq(cache.entries.get(&key(2)).unwrap(), &second));

        // A single oversized raster is not cached and does not flush entries
        // that already fit within the budget.
        cache.insert_with_limits(key(3), entry(3, 3), 4, entry_bytes + 1);
        assert_eq!(cache.entries.len(), 1);
        assert!(cache.entries.contains_key(&key(2)));
        assert_eq!(cache.bytes, entry_bytes);
    }

    fn font_dir() -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fonts")
            .to_string_lossy()
            .to_string()
    }

    fn render(expr: &str) -> Pixmap {
        let opts = RenderOptions {
            font_dir: font_dir(),
            ..RenderOptions::default()
        };
        let ast = ratex_parser::parser::parse(expr).expect("parse");
        let layout = ratex_layout::layout(&ast, &ratex_layout::LayoutOptions::default());
        let dl = ratex_layout::to_display_list(&layout);
        render_to_png(&dl, &opts).expect("render");
        Pixmap::decode_png(&render_to_png(&dl, &opts).expect("render")).expect("decode")
    }

    fn ink_bbox(pixmap: &Pixmap) -> (u32, u32, u32, u32) {
        let mut min_x = u32::MAX;
        let mut min_y = u32::MAX;
        let mut max_x = 0;
        let mut max_y = 0;
        for (i, px) in pixmap.data().chunks_exact(4).enumerate() {
            if px[0] < 200 {
                let x = (i as u32) % pixmap.width();
                let y = (i as u32) / pixmap.width();
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
        (min_x, min_y, max_x, max_y)
    }

    #[test]
    fn repeated_render_is_pixel_identical() {
        let a = render("x^2 + y^2 = z^2");
        let b = render("x^2 + y^2 = z^2");
        assert_eq!(a.data(), b.data());
    }

    #[test]
    fn cached_glyph_at_second_position_is_not_misplaced() {
        // Two identical glyphs at different positions: the second must come
        // from the mask cache and still be drawn at its own position.
        let pixmap = render("a+a");
        let (min_x, _, max_x, _) = ink_bbox(&pixmap);
        // ~20 px per glyph at font_size 40 plus spacing: both `a`s must be
        // present, so the ink span covers both positions.
        assert!(
            (max_x - min_x) > 40,
            "expected both 'a's to be drawn, ink bbox x {min_x}..{max_x}"
        );
    }

    #[test]
    fn interleaved_formulas_share_cache_without_misplacement() {
        // The first formula populates the cache ('+' and '=' glyphs); the
        // second must draw them at its own positions.
        let a = render("a+b=c");
        let _b = render("x^2 + y^2 = z^2");
        let c = render("a+b=c");
        assert_eq!(a.data(), c.data());
    }

    #[test]
    fn clipped_glyph_keeps_direct_fill_antialiasing() {
        // The wide mathclap subscript pushes one glyph across the left canvas
        // edge. Rasterizing it into a full offscreen mask and clipping during
        // the blit changes tiny-skia's coverage at this exact edge pixel.
        let pixmap = render(r"\sum_{\mathclap{1\le i\le n}} x_{i}");
        let offset = (82 * pixmap.width() as usize) * 4;
        assert_eq!(&pixmap.data()[offset..offset + 4], &[48, 48, 48, 255]);
    }
}
