//! Glyph outlines as SVG `<path>` via `ab_glyph` (feature `standalone`).

use std::collections::HashMap;

use ab_glyph::{Font, FontRef, OutlineCurve};
use ratex_font::FontId;

#[allow(unused_variables)]
pub(crate) fn load_all_fonts(font_dir: &str) -> Result<HashMap<FontId, Vec<u8>>, String> {
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
        if !data.contains_key(&FontId::CjkRegular) {
            data.insert(FontId::CjkRegular, cjk_bytes.to_vec());
        }
    }
    // Secondary system fallback for characters the primary CJK font doesn't cover.
    if let Some(fb_bytes) = ratex_unicode_font::load_fallback_font() {
        if !data.contains_key(&FontId::CjkFallback) {
            data.insert(FontId::CjkFallback, fb_bytes.to_vec());
        }
    }
    if let Some(emoji_bytes) = ratex_unicode_font::load_emoji_font() {
        if !data.contains_key(&FontId::EmojiFallback) {
            data.insert(FontId::EmojiFallback, emoji_bytes.to_vec());
        }
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

pub(crate) fn build_font_cache(
    data: &HashMap<FontId, Vec<u8>>,
) -> Result<HashMap<FontId, FontRef<'_>>, String> {
    let mut cache = HashMap::new();
    for (id, bytes) in data {
        let font = FontRef::try_from_slice_and_index(bytes, sfnt_collection_index(*id))
            .map_err(|e| format!("Failed to parse font {id:?}: {e}"))?;
        cache.insert(*id, font);
    }
    Ok(cache)
}

/// Vector path or color-emoji raster (`sbix` PNG as `data:image/png`), matching `ratex-render::render_glyph`.
#[derive(Debug)]
pub(crate) enum StandaloneGlyph {
    Path(String),
    Image {
        href: String,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
    },
}

/// Same geometry as `ratex-render`: SVG user space, y downward. Emoji uses bitmap **before** outline
/// so COLR/sbix faces do not paint invisible vector masks.
pub(crate) fn standalone_glyph(
    px: f32,
    py: f32,
    glyph_em: f32,
    font_name: &str,
    char_code: u32,
    font_cache: &HashMap<FontId, FontRef<'_>>,
) -> Option<StandaloneGlyph> {
    let font_id = FontId::parse(font_name).unwrap_or(FontId::MainRegular);
    let font = match font_cache.get(&font_id) {
        Some(f) => f,
        None => font_cache.get(&FontId::MainRegular)?,
    };

    let ch = ratex_font::katex_ttf_glyph_char(font_id, char_code);
    let glyph_id = font.glyph_id(ch);

    if glyph_id.0 == 0 {
        return try_system_unicode_fallback_svg(px, py, glyph_em, ch, font_cache, false);
    }

    if font_id == FontId::EmojiFallback {
        return try_emoji_raster_or_vector_svg(px, py, glyph_em, ch, font, glyph_id);
    }

    if font_id == FontId::CjkRegular {
        if let Some(d) = outline_to_d(px, py, glyph_em, font, glyph_id) {
            return Some(StandaloneGlyph::Path(d));
        }
        if let Some(g) = try_emoji_raster_then_vector_svg(px, py, glyph_em, ch, font_cache) {
            return Some(g);
        }
        if let Some(fb) = font_cache.get(&FontId::CjkFallback) {
            let fid = fb.glyph_id(ch);
            if fid.0 != 0 {
                return outline_to_d(px, py, glyph_em, fb, fid).map(StandaloneGlyph::Path);
            }
        }
        return None;
    }

    if font_id == FontId::CjkFallback {
        if let Some(d) = outline_to_d(px, py, glyph_em, font, glyph_id) {
            return Some(StandaloneGlyph::Path(d));
        }
        return try_emoji_raster_then_vector_svg(px, py, glyph_em, ch, font_cache);
    }

    if let Some(d) = outline_to_d(px, py, glyph_em, font, glyph_id) {
        return Some(StandaloneGlyph::Path(d));
    }

    let skip_main = font_id == FontId::MainRegular;
    try_system_unicode_fallback_svg(px, py, glyph_em, ch, font_cache, skip_main)
}

fn try_emoji_png_data_url(px: f32, py: f32, em: f32, ch: char) -> Option<StandaloneGlyph> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    let strike = ratex_unicode_font::emoji_png_raster_for_char(ch, em)?;
    let scale = em / f32::from(strike.pixels_per_em.max(1));
    let x = px + f32::from(strike.x) * scale;
    // Match `ratex-render::try_blit_raster_glyph`: `y` is the bitmap bottom in y-up strike space;
    // then nudge so the strike's vertical center aligns with the math axis (mixed `\text` + math).
    let mut y = py - (f32::from(strike.y) + f32::from(strike.height)) * scale;
    let ppem = f32::from(strike.pixels_per_em.max(1));
    let center_strike = (f32::from(strike.y) + f32::from(strike.height) / 2.0) / ppem;
    let axis = ratex_font::get_global_metrics(0).axis_height as f32;
    y += (center_strike - axis) * em;
    let w = f32::from(strike.width) * scale;
    let h = f32::from(strike.height) * scale;
    let href = format!(
        "data:image/png;base64,{}",
        STANDARD.encode(&strike.data)
    );
    Some(StandaloneGlyph::Image { href, x, y, w, h })
}

fn try_emoji_raster_then_vector_svg(
    px: f32,
    py: f32,
    em: f32,
    ch: char,
    font_cache: &HashMap<FontId, FontRef<'_>>,
) -> Option<StandaloneGlyph> {
    if let Some(img) = try_emoji_png_data_url(px, py, em, ch) {
        return Some(img);
    }
    let emoji_font = font_cache.get(&FontId::EmojiFallback)?;
    let eid = emoji_font.glyph_id(ch);
    if eid.0 == 0 {
        return None;
    }
    outline_to_d(px, py, em, emoji_font, eid).map(StandaloneGlyph::Path)
}

fn try_emoji_raster_or_vector_svg(
    px: f32,
    py: f32,
    em: f32,
    ch: char,
    font: &FontRef<'_>,
    glyph_id: ab_glyph::GlyphId,
) -> Option<StandaloneGlyph> {
    if let Some(img) = try_emoji_png_data_url(px, py, em, ch) {
        return Some(img);
    }
    outline_to_d(px, py, em, font, glyph_id).map(StandaloneGlyph::Path)
}

fn try_system_unicode_fallback_svg(
    px: f32,
    py: f32,
    em: f32,
    ch: char,
    font_cache: &HashMap<FontId, FontRef<'_>>,
    skip_main_regular: bool,
) -> Option<StandaloneGlyph> {
    if !skip_main_regular {
        if let Some(fallback) = font_cache.get(&FontId::MainRegular) {
            let fid = fallback.glyph_id(ch);
            if fid.0 != 0 {
                if let Some(d) = outline_to_d(px, py, em, fallback, fid) {
                    return Some(StandaloneGlyph::Path(d));
                }
            }
        }
    }
    if let Some(cjk) = font_cache.get(&FontId::CjkRegular) {
        let cid = cjk.glyph_id(ch);
        if cid.0 != 0 {
            if let Some(d) = outline_to_d(px, py, em, cjk, cid) {
                return Some(StandaloneGlyph::Path(d));
            }
        }
    }
    if let Some(g) = try_emoji_raster_then_vector_svg(px, py, em, ch, font_cache) {
        return Some(g);
    }
    if let Some(fb) = font_cache.get(&FontId::CjkFallback) {
        let fid = fb.glyph_id(ch);
        if fid.0 != 0 {
            return outline_to_d(px, py, em, fb, fid).map(StandaloneGlyph::Path);
        }
    }
    None
}

fn outline_to_d(
    px: f32,
    py: f32,
    em: f32,
    font: &FontRef<'_>,
    glyph_id: ab_glyph::GlyphId,
) -> Option<String> {
    let outline = font.outline(glyph_id)?;
    let units_per_em = font.units_per_em().unwrap_or(1000.0);
    let scale = em / units_per_em;

    let mut d = String::new();
    let mut last_end: Option<(f32, f32)> = None;

    for curve in &outline.curves {
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

        let need_move = match last_end {
            None => true,
            Some((lx, ly)) => (lx - start.0).abs() > 0.01 || (ly - start.1).abs() > 0.01,
        };

        if need_move {
            if last_end.is_some() {
                d.push('Z');
                d.push(' ');
            }
            use std::fmt::Write;
            let _ = write!(
                &mut d,
                "M{} {}",
                super::fmt_num(start.0 as f64),
                super::fmt_num(start.1 as f64)
            );
            d.push(' ');
        }

        match curve {
            OutlineCurve::Line(_, p1) => {
                use std::fmt::Write;
                let _ = write!(
                    &mut d,
                    "L{} {}",
                    super::fmt_num((px + p1.x * scale) as f64),
                    super::fmt_num((py - p1.y * scale) as f64)
                );
                d.push(' ');
            }
            OutlineCurve::Quad(_, p1, p2) => {
                use std::fmt::Write;
                let _ = write!(
                    &mut d,
                    "Q{} {} {} {}",
                    super::fmt_num((px + p1.x * scale) as f64),
                    super::fmt_num((py - p1.y * scale) as f64),
                    super::fmt_num((px + p2.x * scale) as f64),
                    super::fmt_num((py - p2.y * scale) as f64)
                );
                d.push(' ');
            }
            OutlineCurve::Cubic(_, p1, p2, p3) => {
                use std::fmt::Write;
                let _ = write!(
                    &mut d,
                    "C{} {} {} {} {} {}",
                    super::fmt_num((px + p1.x * scale) as f64),
                    super::fmt_num((py - p1.y * scale) as f64),
                    super::fmt_num((px + p2.x * scale) as f64),
                    super::fmt_num((py - p2.y * scale) as f64),
                    super::fmt_num((px + p3.x * scale) as f64),
                    super::fmt_num((py - p3.y * scale) as f64)
                );
                d.push(' ');
            }
        }

        last_end = Some(end);
    }

    if last_end.is_some() {
        d.push('Z');
    }

    let d = d.trim().to_string();
    if d.is_empty() {
        None
    } else {
        Some(d)
    }
}
