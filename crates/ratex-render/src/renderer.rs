use std::collections::HashMap;
use std::sync::{Arc, LazyLock, RwLock};

use ab_glyph::{Font, FontVec};
use ratex_font::FontId;
use ratex_font_loader::{OutlineSourceId, ParsedFontSet};
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
    render_display_list(pixmap, display_list, &font_refs, em_px, pad_px, dpr);
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

#[derive(Clone, Copy)]
struct ParsedFontRef<'a> {
    font: &'a FontVec,
    source_id: OutlineSourceId,
}

/// Build a `FontId → (&FontVec, OutlineSourceId)` map from the parsed-font cache.
///
/// The parsed fonts are already cached globally by `ratex-font-loader`, so
/// this is just a reference map for the current render call.
fn build_font_refs(data: &ParsedFontSet) -> HashMap<FontId, ParsedFontRef<'_>> {
    data.iter_with_source()
        .map(|(id, font, source_id)| (*id, ParsedFontRef { font, source_id }))
        .collect()
}

/// Render all items in the DisplayList using the given font cache.
fn render_display_list(
    pixmap: &mut Pixmap,
    display_list: &DisplayList,
    font_cache: &HashMap<FontId, ParsedFontRef<'_>>,
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
    font_cache: &HashMap<FontId, ParsedFontRef<'_>>,
    skip_main_regular: bool,
) -> bool {
    if !skip_main_regular {
        if let Some(fallback) = font_cache.get(&FontId::MainRegular) {
            let fid = fallback.font.glyph_id(ch);
            if fid.0 != 0
                && render_glyph_with_font(
                    pixmap,
                    px,
                    py,
                    FontGlyph {
                        font_id: FontId::MainRegular,
                        font: fallback.font,
                        source_id: fallback.source_id,
                        glyph_id: fid,
                    },
                    color,
                    em,
                )
            {
                return true;
            }
        }
    }
    if let Some(cjk_font) = font_cache.get(&FontId::CjkRegular) {
        let fid = cjk_font.font.glyph_id(ch);
        if fid.0 != 0
            && render_glyph_with_font(
                pixmap,
                px,
                py,
                FontGlyph {
                    font_id: FontId::CjkRegular,
                    font: cjk_font.font,
                    source_id: cjk_font.source_id,
                    glyph_id: fid,
                },
                color,
                em,
            )
        {
            return true;
        }
    }
    if try_emoji_vector_then_bitmap(pixmap, px, py, ch, color, em, font_cache) {
        return true;
    }
    if let Some(fb_font) = font_cache.get(&FontId::CjkFallback) {
        let fid = fb_font.font.glyph_id(ch);
        if fid.0 != 0
            && render_glyph_with_font(
                pixmap,
                px,
                py,
                FontGlyph {
                    font_id: FontId::CjkFallback,
                    font: fb_font.font,
                    source_id: fb_font.source_id,
                    glyph_id: fid,
                },
                color,
                em,
            )
        {
            return true;
        }
    }
    false
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
    font_cache: &HashMap<FontId, ParsedFontRef<'_>>,
) -> bool {
    if try_blit_emoji_raster_fallback(pixmap, px, py, em, ch, color) {
        return true;
    }
    if let Some(emoji_font) = font_cache.get(&FontId::EmojiFallback) {
        let eid = emoji_font.font.glyph_id(ch);
        if eid.0 != 0
            && render_glyph_with_font(
                pixmap,
                px,
                py,
                FontGlyph {
                    font_id: FontId::EmojiFallback,
                    font: emoji_font.font,
                    source_id: emoji_font.source_id,
                    glyph_id: eid,
                },
                color,
                em,
            )
        {
            return true;
        }
    }
    false
}

#[allow(clippy::too_many_arguments)]
fn render_glyph(
    pixmap: &mut Pixmap,
    px: f32,
    py: f32,
    font_id: FontId,
    char_code: u32,
    color: &Color,
    font_cache: &HashMap<FontId, ParsedFontRef<'_>>,
    em: f32,
) {
    let font_entry = match font_cache.get(&font_id) {
        Some(entry) => *entry,
        None => match font_cache.get(&FontId::MainRegular) {
            Some(entry) => *entry,
            None => return,
        },
    };
    let font = font_entry.font;

    let ch = ratex_font::katex_ttf_glyph_char(font_id, char_code);
    let glyph_id = font.glyph_id(ch);

    if glyph_id.0 == 0 {
        let _ = try_system_unicode_fallback(pixmap, px, py, ch, color, em, font_cache, false);
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
                source_id: font_entry.source_id,
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
                source_id: font_entry.source_id,
                glyph_id,
            },
            color,
            em,
        ) {
            return;
        }
        if try_emoji_vector_then_bitmap(pixmap, px, py, ch, color, em, font_cache) {
            return;
        }
        if let Some(fb_font) = font_cache.get(&FontId::CjkFallback) {
            let fid = fb_font.font.glyph_id(ch);
            if fid.0 != 0
                && render_glyph_with_font(
                    pixmap,
                    px,
                    py,
                    FontGlyph {
                        font_id: FontId::CjkFallback,
                        font: fb_font.font,
                        source_id: fb_font.source_id,
                        glyph_id: fid,
                    },
                    color,
                    em,
                )
            {
                return;
            }
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
                source_id: font_entry.source_id,
                glyph_id,
            },
            color,
            em,
        ) {
            return;
        }
        let _ = try_emoji_vector_then_bitmap(pixmap, px, py, ch, color, em, font_cache);
        return;
    }

    if render_glyph_with_font(
        pixmap,
        px,
        py,
        FontGlyph {
            font_id,
            font,
            source_id: font_entry.source_id,
            glyph_id,
        },
        color,
        em,
    ) {
        return;
    }
    // cmap had a non-zero GID but no `glyf` outline (e.g. blank text-font slot for emoji).
    let skip_main = font_id == FontId::MainRegular;
    let _ = try_system_unicode_fallback(pixmap, px, py, ch, color, em, font_cache, skip_main);
}

struct FontGlyph<'a> {
    font_id: FontId,
    font: &'a FontVec,
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
/// The font bytes are held in an `Arc` inside `ratex-unicode-font`, so the
/// pointer/length pair is stable for the process lifetime and avoids cloning
/// or hashing the font data on every lookup. This cache must only be fed from
/// that process-lifetime `OnceLock<Arc<Vec<u8>>>`; transient buffers could be
/// freed and their address reused by a different font.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct EmojiRasterCacheKey {
    font_ptr: usize,
    font_len: usize,
    face_index: u32,
    ch: char,
    strike: u16,
}

struct CachedEmojiRaster {
    pixmap: Arc<Pixmap>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    pixels_per_em: f32,
}

static EMOJI_RASTER_CACHE: LazyLock<RwLock<HashMap<EmojiRasterCacheKey, Arc<CachedEmojiRaster>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Upper bound on cached decoded emoji strikes. Each strike is usually a few
/// KiB to tens of KiB; clearing wholesale on overflow keeps long-running
/// renderers bounded while preserving the common repeated-formula fast path.
const EMOJI_RASTER_CACHE_CAP: usize = 4096;

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

static GLYPH_MASK_CACHE: LazyLock<RwLock<HashMap<GlyphMaskKey, Arc<Pixmap>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// Upper bound on cached glyph masks. Masks are small (a 40 px glyph is
/// ~4 KiB); the cap keeps long-running processes bounded. On overflow the
/// cache is cleared wholesale — simple, and misses only re-rasterize.
const GLYPH_MASK_CACHE_CAP: usize = 8192;

fn pack_color_u32(color: &Color) -> u32 {
    let [r, g, b, a] = color_to_rgba8(color);
    ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | a as u32
}

/// Draw a cached glyph mask at the given integer anchor (exact pixel copy).
fn blit_glyph_mask(pixmap: &mut Pixmap, mask: &Pixmap, dst_x: i32, dst_y: i32) {
    let paint = PixmapPaint {
        opacity: 1.0,
        blend_mode: tiny_skia::BlendMode::SourceOver,
        quality: FilterQuality::Nearest,
    };
    pixmap.draw_pixmap(
        dst_x,
        dst_y,
        mask.as_ref(),
        &paint,
        Transform::identity(),
        None,
    );
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

fn render_glyph_with_font(
    pixmap: &mut Pixmap,
    px: f32,
    py: f32,
    g: FontGlyph<'_>,
    color: &Color,
    em: f32,
) -> bool {
    let curves = match ratex_font_loader::outline_cache::get_or_compute_outline_fontvec(
        g.font_id,
        g.font,
        g.source_id,
        g.glyph_id,
    ) {
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

    {
        let cache = GLYPH_MASK_CACHE.read().unwrap();
        if let Some(mask) = cache.get(&cache_key) {
            let mask = Arc::clone(mask);
            drop(cache);
            blit_glyph_mask(pixmap, &mask, dst_x, dst_y);
            return true;
        }
    }

    let mut builder = PathBuilder::new();
    let mut last_end: Option<(f32, f32)> = None;

    for curve in curves.iter() {
        use ab_glyph::OutlineCurve;
        let (start, end) = match curve {
            OutlineCurve::Line(p0, p1) => {
                let sx = px + p0.x * scale;
                let sy = py - p0.y * scale;
                let ex = px + p1.x * scale;
                let ey = py - p1.y * scale;
                ((sx, sy), (ex, ey))
            }
            OutlineCurve::Quad(p0, _, p2) => {
                let sx = px + p0.x * scale;
                let sy = py - p0.y * scale;
                let ex = px + p2.x * scale;
                let ey = py - p2.y * scale;
                ((sx, sy), (ex, ey))
            }
            OutlineCurve::Cubic(p0, _, _, p3) => {
                let sx = px + p0.x * scale;
                let sy = py - p0.y * scale;
                let ex = px + p3.x * scale;
                let ey = py - p3.y * scale;
                ((sx, sy), (ex, ey))
            }
        };

        // New contour if start doesn't match previous end
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

    let Some(path) = builder.finish() else {
        return false;
    };

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

    blit_glyph_mask(pixmap, &mask, dst_x, dst_y);

    // Insert without replacing an existing entry: another thread may have
    // rasterized the same glyph while we were working.
    let entry = Arc::new(mask);
    let mut cache = GLYPH_MASK_CACHE.write().unwrap();
    if cache.len() >= GLYPH_MASK_CACHE_CAP {
        cache.clear();
    }
    cache.entry(cache_key).or_insert(entry);
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
    let Some(bytes) = ratex_unicode_font::load_emoji_font_arc() else {
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
    let strike = params.em.round().clamp(8.0, 256.0) as u16;

    let key = EmojiRasterCacheKey {
        font_ptr: font_bytes.as_ptr() as usize,
        font_len: font_bytes.len(),
        face_index,
        ch: params.ch,
        strike,
    };
    {
        let cache = EMOJI_RASTER_CACHE.read().unwrap();
        if let Some(entry) = cache.get(&key) {
            return blit_cached_emoji_raster(pixmap, &params, entry);
        }
    }

    let img = face
        .glyph_raster_image(gid, strike)
        .or_else(|| face.glyph_raster_image(gid, u16::MAX));
    let Some(img) = img else {
        return false;
    };
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
    if cache.len() >= EMOJI_RASTER_CACHE_CAP {
        cache.clear();
    }
    cache.entry(key).or_insert(entry);
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
    // Fast compression path: `Pixmap::encode_png` uses the png crate's default
    // (stronger) zlib settings; math formulas are mostly flat white regions,
    // where `Compression::Fast` + `Sub` filtering decodes to identical pixels
    // in a fraction of the time (larger files by a small margin).
    let data = demultiply_rgba(pixmap.data());
    let mut out = Vec::with_capacity(pixmap.width() as usize * pixmap.height() as usize);
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
    Ok(out)
}

#[cfg(test)]
mod glyph_mask_cache_tests {
    use super::*;

    #[test]
    fn glyph_mask_color_key_uses_paint_quantization() {
        // These two values both round to byte 1, but the paint path truncates
        // them to 0 and 1 respectively. The cache key must use the paint
        // quantization, otherwise the second glyph could reuse the first
        // glyph's mask and render the wrong red channel.
        let truncates_to_zero = Color::new(0.0020000001, 0.0, 0.0, 1.0);
        let truncates_to_one = Color::new(0.0058431374, 0.0, 0.0, 1.0);
        assert_ne!(
            pack_color_u32(&truncates_to_zero),
            pack_color_u32(&truncates_to_one)
        );
        assert_eq!(color_to_rgba8(&truncates_to_zero)[0], 0);
        assert_eq!(color_to_rgba8(&truncates_to_one)[0], 1);
    }

    fn font_dir() -> String {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fonts")
            .to_string_lossy()
            .to_string()
    }

    fn render(expr: &str) -> Pixmap {
        let mut opts = RenderOptions::default();
        opts.font_dir = font_dir();
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
}
