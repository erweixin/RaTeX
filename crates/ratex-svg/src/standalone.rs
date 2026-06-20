//! Glyph outlines as SVG `<path>` via `ab_glyph` (feature `standalone`).

use std::collections::HashMap;

use ab_glyph::{Font, FontRef, OutlineCurve};
use ratex_font::FontId;
use ratex_font_loader::FontSet;

fn sfnt_collection_index(id: FontId) -> u32 {
    match id {
        FontId::EmojiFallback => ratex_unicode_font::emoji_font_face_index().unwrap_or(0),
        FontId::CjkRegular => ratex_unicode_font::unicode_font_face_index().unwrap_or(0),
        FontId::CjkFallback => ratex_unicode_font::fallback_font_face_index().unwrap_or(0),
        _ => 0,
    }
}

/// Build a `FontId → FontRef` map from the cached raw data (held alive by `guard`).
pub(crate) fn build_font_refs<'a>(
    data: &'a FontSet,
) -> Result<HashMap<FontId, FontRef<'a>>, String> {
    let mut font_refs = HashMap::new();
    for (id, bytes) in data.iter() {
        let font = FontRef::try_from_slice_and_index(bytes, sfnt_collection_index(*id))
            .map_err(|e| format!("Failed to parse font {:?}: {}", id, e))?;
        font_refs.insert(*id, font);
    }
    Ok(font_refs)
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
        if let Some(d) = outline_to_d(px, py, glyph_em, FontId::CjkRegular, font, glyph_id) {
            return Some(StandaloneGlyph::Path(d));
        }
        if let Some(g) = try_emoji_raster_then_vector_svg(px, py, glyph_em, ch, font_cache) {
            return Some(g);
        }
        if let Some(fb) = font_cache.get(&FontId::CjkFallback) {
            let fid = fb.glyph_id(ch);
            if fid.0 != 0 {
                return outline_to_d(px, py, glyph_em, FontId::CjkFallback, fb, fid)
                    .map(StandaloneGlyph::Path);
            }
        }
        return None;
    }

    if font_id == FontId::CjkFallback {
        if let Some(d) = outline_to_d(px, py, glyph_em, FontId::CjkFallback, font, glyph_id) {
            return Some(StandaloneGlyph::Path(d));
        }
        return try_emoji_raster_then_vector_svg(px, py, glyph_em, ch, font_cache);
    }

    if let Some(d) = outline_to_d(px, py, glyph_em, font_id, font, glyph_id) {
        return Some(StandaloneGlyph::Path(d));
    }

    let skip_main = font_id == FontId::MainRegular;
    try_system_unicode_fallback_svg(px, py, glyph_em, ch, font_cache, skip_main)
}

fn try_emoji_png_data_url(px: f32, py: f32, em: f32, ch: char) -> Option<StandaloneGlyph> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    #[cfg(target_os = "macos")]
    let request_em = em * 2.0;
    #[cfg(not(target_os = "macos"))]
    let request_em = em;

    let strike = ratex_unicode_font::emoji_png_raster_for_char(ch, request_em)?;
    let ppm = f32::from(strike.pixels_per_em.max(1));
    let mut scale = em / ppm;
    // Scale emoji to fit 1.0em layout width if it's wider (prevents overflow).
    let actual_width_em = f32::from(strike.width) / ppm;
    let assumed_width = 1.0;
    if actual_width_em > 0.01 && actual_width_em > assumed_width * 1.01 {
        scale *= assumed_width / actual_width_em;
    }
    let x = px + f32::from(strike.x) * scale;
    // Match `ratex-render::try_blit_raster_glyph`: `y` is the bitmap bottom in y-up strike space;
    // then nudge so the strike's vertical center aligns with the math axis (mixed `\text` + math).
    let mut y = py - (f32::from(strike.y) + f32::from(strike.height)) * scale;
    let center_strike = (f32::from(strike.y) + f32::from(strike.height) / 2.0) / ppm;
    let axis = ratex_font::get_global_metrics(0).axis_height as f32;
    y += (center_strike - axis) * em;
    let w = f32::from(strike.width) * scale;
    let h = f32::from(strike.height) * scale;
    let href = format!("data:image/png;base64,{}", STANDARD.encode(&strike.data));
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
    outline_to_d(px, py, em, FontId::EmojiFallback, emoji_font, eid).map(StandaloneGlyph::Path)
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
    outline_to_d(px, py, em, FontId::EmojiFallback, font, glyph_id).map(StandaloneGlyph::Path)
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
                if let Some(d) = outline_to_d(px, py, em, FontId::MainRegular, fallback, fid) {
                    return Some(StandaloneGlyph::Path(d));
                }
            }
        }
    }
    if let Some(cjk) = font_cache.get(&FontId::CjkRegular) {
        let cid = cjk.glyph_id(ch);
        if cid.0 != 0 {
            if let Some(d) = outline_to_d(px, py, em, FontId::CjkRegular, cjk, cid) {
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
            return outline_to_d(px, py, em, FontId::CjkFallback, fb, fid)
                .map(StandaloneGlyph::Path);
        }
    }
    None
}

fn outline_to_d(
    px: f32,
    py: f32,
    em: f32,
    font_id: FontId,
    font: &FontRef<'_>,
    glyph_id: ab_glyph::GlyphId,
) -> Option<String> {
    let curves = ratex_font_loader::outline_cache::get_or_compute_outline(font_id, font, glyph_id)?;
    let units_per_em = font.units_per_em().unwrap_or(1000.0);
    let mut scale = em / units_per_em;

    // Emoji outline fallback has no KaTeX metrics; scale it to the 1.0em width that layout
    // allocates for missing emoji so Windows vector fallback does not overflow.
    if font_id == FontId::EmojiFallback {
        let actual_advance = font.h_advance_unscaled(glyph_id);
        let actual_advance_em = actual_advance / units_per_em;
        let assumed_width = 1.0;
        if actual_advance_em > 0.01 && actual_advance_em > assumed_width * 1.01 {
            scale *= assumed_width / actual_advance_em;
        }
    }

    let mut d = String::new();
    let mut last_end: Option<(f32, f32)> = None;

    for curve in curves.iter() {
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
