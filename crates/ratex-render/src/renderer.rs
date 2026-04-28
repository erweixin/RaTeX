use std::collections::HashMap;

use ab_glyph::{Font, FontRef};
use ratex_font::FontId;
use ratex_types::color::Color;
use ratex_types::display_item::{DisplayItem, DisplayList};
use tiny_skia::{
    FilterQuality, FillRule, Paint, PathBuilder, Pixmap, PixmapPaint, Stroke, Transform,
};

pub struct RenderOptions {
    pub font_size: f32,
    pub padding: f32,
    /// Directory containing KaTeX `*.ttf` files (see `load_all_fonts`). Each file that exists is
    /// loaded; missing files (e.g. no `KaTeX_Fraktur-Bold.ttf`) are skipped and that face falls back.
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

    pixmap.fill(tiny_skia::Color::WHITE);

    let font_data = load_all_fonts(&options.font_dir)?;
    let font_cache = build_font_cache(&font_data)?;

    for item in &display_list.items {
        match item {
            DisplayItem::GlyphPath {
                x,
                y,
                scale,
                font,
                char_code,
                commands: _,
                color,
            } => {
                let glyph_em = em_px * *scale as f32;
                render_glyph(
                    &mut pixmap,
                    *x as f32 * em_px + pad_px,
                    *y as f32 * em_px + pad_px,
                    font,
                    *char_code,
                    color,
                    &font_cache,
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
                    &mut pixmap,
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
                    &mut pixmap,
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
                    &mut pixmap,
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

    encode_png(&pixmap)
}

/// Load KaTeX TTFs from disk. Only existing paths are inserted; callers should point [RenderOptions::font_dir]
/// at a folder that includes every face the layout may emit (e.g. repo root `fonts/`).
#[allow(unused_variables)]
fn load_all_fonts(font_dir: &str) -> Result<HashMap<FontId, Vec<u8>>, String> {
    let mut data = HashMap::new();
    let font_map = [
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

    #[cfg(not(feature = "embed-fonts"))]
    {
        let dir = std::path::Path::new(font_dir);
        for (id, filename) in &font_map {
            let path = dir.join(filename);
            if path.exists() {
                let bytes = std::fs::read(&path)
                    .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;
                data.insert(*id, bytes);
            }
        }

        if data.is_empty() {
            return Err(format!("No fonts found in {font_dir}"));
        }
    }

    #[cfg(feature = "embed-fonts")]
    {
        for (id, filename) in &font_map {
            let font = ratex_katex_fonts::ttf_bytes(filename)
                .ok_or_else(|| format!("Failed to get embedded font {filename}"))?;
            data.insert(*id, font.to_vec());
        }
    }

    // Load system Unicode font for CJK/fallback glyphs.
    if let Some(cjk_bytes) = ratex_unicode_font::load_unicode_font() {
        data.entry(FontId::CjkRegular)
            .or_insert_with(|| cjk_bytes.to_vec());
    }
    // Secondary system fallback for characters the primary CJK font doesn't cover.
    if let Some(fb_bytes) = ratex_unicode_font::load_fallback_font() {
        data.entry(FontId::CjkFallback)
            .or_insert_with(|| fb_bytes.to_vec());
    }
    if let Some(emoji_bytes) = ratex_unicode_font::load_emoji_font() {
        data.entry(FontId::EmojiFallback)
            .or_insert_with(|| emoji_bytes.to_vec());
    }

    Ok(data)
}

fn sfnt_collection_index(id: FontId) -> u32 {
    match id {
        FontId::EmojiFallback => ratex_unicode_font::emoji_font_face_index().unwrap_or(0),
        FontId::CjkRegular => ratex_unicode_font::unicode_font_face_index().unwrap_or(0),
        FontId::CjkFallback => ratex_unicode_font::fallback_font_face_index().unwrap_or(0),
        _ => 0,
    }
}

fn build_font_cache(data: &HashMap<FontId, Vec<u8>>) -> Result<HashMap<FontId, FontRef<'_>>, String> {
    let mut cache = HashMap::new();
    for (id, bytes) in data {
        let font = FontRef::try_from_slice_and_index(bytes, sfnt_collection_index(*id))
            .map_err(|e| format!("Failed to parse font {:?}: {}", id, e))?;
        cache.insert(*id, font);
    }
    Ok(cache)
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
    font_cache: &HashMap<FontId, FontRef<'_>>,
    skip_main_regular: bool,
) -> bool {
    if !skip_main_regular {
        if let Some(fallback) = font_cache.get(&FontId::MainRegular) {
            let fid = fallback.glyph_id(ch);
            if fid.0 != 0 && render_glyph_with_font(pixmap, px, py, fallback, fid, color, em) {
                return true;
            }
        }
    }
    if let Some(cjk_font) = font_cache.get(&FontId::CjkRegular) {
        let fid = cjk_font.glyph_id(ch);
        if fid.0 != 0 && render_glyph_with_font(pixmap, px, py, cjk_font, fid, color, em) {
            return true;
        }
    }
    if try_emoji_vector_then_bitmap(pixmap, px, py, ch, color, em, font_cache) {
        return true;
    }
    if let Some(fb_font) = font_cache.get(&FontId::CjkFallback) {
        let fid = fb_font.glyph_id(ch);
        if fid.0 != 0 && render_glyph_with_font(pixmap, px, py, fb_font, fid, color, em) {
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
    font_cache: &HashMap<FontId, FontRef<'_>>,
) -> bool {
    if try_blit_emoji_raster_fallback(pixmap, px, py, em, ch) {
        return true;
    }
    if let Some(emoji_font) = font_cache.get(&FontId::EmojiFallback) {
        let eid = emoji_font.glyph_id(ch);
        if eid.0 != 0 && render_glyph_with_font(pixmap, px, py, emoji_font, eid, color, em) {
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
    font_name: &str,
    char_code: u32,
    color: &Color,
    font_cache: &HashMap<FontId, FontRef<'_>>,
    em: f32,
) {
    let font_id = FontId::parse(font_name).unwrap_or(FontId::MainRegular);
    let font = match font_cache.get(&font_id) {
        Some(f) => f,
        None => match font_cache.get(&FontId::MainRegular) {
            Some(f) => f,
            None => return,
        },
    };

    let ch = ratex_font::katex_ttf_glyph_char(font_id, char_code);
    let glyph_id = font.glyph_id(ch);

    if glyph_id.0 == 0 {
        let _ = try_system_unicode_fallback(pixmap, px, py, ch, color, em, font_cache, false);
        return;
    }

    if font_id == FontId::EmojiFallback {
        if try_blit_emoji_raster_fallback(pixmap, px, py, em, ch) {
            return;
        }
        let _ = render_glyph_with_font(pixmap, px, py, font, glyph_id, color, em);
        return;
    }

    // `RATEX_UNICODE_FONT` may map a codepoint to a non-.notdef glyph with no outlines; try system fallback.
    if font_id == FontId::CjkRegular {
        if render_glyph_with_font(pixmap, px, py, font, glyph_id, color, em) {
            return;
        }
        if try_emoji_vector_then_bitmap(pixmap, px, py, ch, color, em, font_cache) {
            return;
        }
        if let Some(fb_font) = font_cache.get(&FontId::CjkFallback) {
            let fid = fb_font.glyph_id(ch);
            if fid.0 != 0 && render_glyph_with_font(pixmap, px, py, fb_font, fid, color, em) {
                return;
            }
        }
        return;
    }

    if font_id == FontId::CjkFallback {
        if render_glyph_with_font(pixmap, px, py, font, glyph_id, color, em) {
            return;
        }
        let _ = try_emoji_vector_then_bitmap(pixmap, px, py, ch, color, em, font_cache);
        return;
    }

    if render_glyph_with_font(pixmap, px, py, font, glyph_id, color, em) {
        return;
    }
    // cmap had a non-zero GID but no `glyf` outline (e.g. blank text-font slot for emoji).
    let skip_main = font_id == FontId::MainRegular;
    let _ = try_system_unicode_fallback(pixmap, px, py, ch, color, em, font_cache, skip_main);
}

fn render_glyph_with_font(
    pixmap: &mut Pixmap,
    px: f32,
    py: f32,
    font: &FontRef<'_>,
    glyph_id: ab_glyph::GlyphId,
    color: &Color,
    em: f32,
) -> bool {
    let outline = match font.outline(glyph_id) {
        Some(o) => o,
        None => return false,
    };
    if outline.curves.is_empty() {
        return false;
    }

    let units_per_em = font.units_per_em().unwrap_or(1000.0);
    let scale = em / units_per_em;

    let mut builder = PathBuilder::new();
    let mut last_end: Option<(f32, f32)> = None;

    for curve in &outline.curves {
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

    if let Some(path) = builder.finish() {
        let mut paint = Paint::default();
        paint.set_color_rgba8(
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            255,
        );
        paint.anti_alias = true;
        pixmap.fill_path(
            &path,
            &paint,
            tiny_skia::FillRule::Winding,
            Transform::identity(),
            None,
        );
        true
    } else {
        false
    }
}

/// Color emoji (sbix / CBDT / etc.) often have no `glyf` outlines; `ttf-parser` embedded strikes + PNG.
fn try_blit_emoji_raster_fallback(pixmap: &mut Pixmap, px: f32, py: f32, em: f32, ch: char) -> bool {
    let Some((bytes, idx)) = ratex_unicode_font::load_emoji_font_with_index() else {
        return false;
    };
    try_blit_raster_glyph(pixmap, px, py, em, ch, bytes, idx)
}

fn try_blit_raster_glyph(
    pixmap: &mut Pixmap,
    px: f32,
    py: f32,
    em: f32,
    ch: char,
    font_bytes: &[u8],
    face_index: u32,
) -> bool {
    let face = match ttf_parser::Face::parse(font_bytes, face_index) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let gid = match face.glyph_index(ch) {
        Some(g) => g,
        None => return false,
    };
    let strike = em.round().clamp(8.0, 256.0) as u16;
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
    let scale = em / f32::from(img.pixels_per_em.max(1));
    let top_x = px + f32::from(img.x) * scale;
    // `ttf-parser` / OpenType: `RasterGlyphImage::{x,y}` are in strike pixels; `y` is the
    // **bottom** edge of the bitmap in y-up coordinates (sbix yOffset to bottom; CBDT normalized
    // the same way). Top edge = y + height — using `y` alone shifts the glyph down by ~full height.
    let mut top_y = py - (f32::from(img.y) + f32::from(img.height)) * scale;
    // sbix places the bitmap bottom on the math baseline, but tall (~1em) color strikes put the
    // ink centroid near 0.5em above baseline. Binary/relation glyphs (+, =) are centered on the
    // math axis (~0.25em). Nudge the bitmap so its vertical center matches the axis — matches
    // mixed `\text{emoji} … formula` rows without changing layout baselines.
    let ppem = f32::from(img.pixels_per_em.max(1));
    let center_strike = (f32::from(img.y) + f32::from(img.height) / 2.0) / ppem;
    let axis = ratex_font::get_global_metrics(0).axis_height as f32;
    top_y += (center_strike - axis) * em;
    let paint = PixmapPaint {
        quality: FilterQuality::Bilinear,
        ..Default::default()
    };
    let transform = Transform::from_row(scale, 0.0, 0.0, scale, top_x, top_y);
    pixmap.draw_pixmap(0, 0, glyph_pm.as_ref(), &paint, transform, None);
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

fn render_line(pixmap: &mut Pixmap, x: f32, y: f32, width: f32, thickness: f32, color: &Color, dashed: bool) {
    let t = thickness.max(1.0);
    let mut paint = Paint::default();
    paint.set_color_rgba8(
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        255,
    );

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
    } else {
        if let Some(rect) = tiny_skia::Rect::from_xywh(x, y - t / 2.0, width, t) {
            pixmap.fill_rect(rect, &paint, Transform::identity(), None);
        }
    }
}

fn render_rect(pixmap: &mut Pixmap, x: f32, y: f32, width: f32, height: f32, color: &Color) {
    // Clamp to at least 2px: with width=1px at a fractional pixel position, fill_dot8's
    // dot-8 fixed-point arithmetic can produce inner_width=0 and trigger a debug_assert.
    // 2px guarantees at least 1 full interior pixel regardless of sub-pixel alignment.
    let width = width.max(2.0);
    let height = height.max(2.0);
    let rect = tiny_skia::Rect::from_xywh(x, y, width, height);
    if let Some(rect) = rect {
        let mut paint = Paint::default();
        paint.set_color_rgba8(
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            255,
        );
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
                if matches!(commands[i], ratex_types::path_command::PathCommand::MoveTo { .. }) {
                    render_path_segment(pixmap, x, y, &commands[start..i], fill, color, em, stroke_width_px);
                    start = i;
                }
            }
            render_path_segment(pixmap, x, y, &commands[start..], fill, color, em, stroke_width_px);
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
            ratex_types::path_command::PathCommand::QuadTo { x1, y1, x: cx, y: cy } => {
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
        let mut paint = Paint::default();
        paint.set_color_rgba8(
            (color.r * 255.0) as u8,
            (color.g * 255.0) as u8,
            (color.b * 255.0) as u8,
            255,
        );
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

fn encode_png(pixmap: &Pixmap) -> Result<Vec<u8>, String> {
    let mut buf = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut buf, pixmap.width(), pixmap.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder
            .write_header()
            .map_err(|e| format!("PNG header error: {}", e))?;
        writer
            .write_image_data(pixmap.data())
            .map_err(|e| format!("PNG write error: {}", e))?;
    }
    Ok(buf)
}
