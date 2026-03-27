//! Glyph outlines as SVG `<path>` via `ab_glyph` (feature `standalone`).

use std::collections::HashMap;
use std::path::Path;

use ab_glyph::{Font, FontRef, OutlineCurve};
use ratex_font::FontId;

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

    let dir = Path::new(font_dir);
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

    Ok(data)
}

pub(crate) fn build_font_cache(
    data: &HashMap<FontId, Vec<u8>>,
) -> Result<HashMap<FontId, FontRef<'_>>, String> {
    let mut cache = HashMap::new();
    for (id, bytes) in data {
        let font = FontRef::try_from_slice(bytes)
            .map_err(|e| format!("Failed to parse font {id:?}: {e}"))?;
        cache.insert(*id, font);
    }
    Ok(cache)
}

/// Same geometry as `ratex-render` glyph rasterization: SVG user space, y downward.
pub(crate) fn glyph_svg_path(
    px: f32,
    py: f32,
    glyph_em: f32,
    font_name: &str,
    char_code: u32,
    font_cache: &HashMap<FontId, FontRef<'_>>,
) -> Option<String> {
    let font_id = FontId::parse(font_name).unwrap_or(FontId::MainRegular);
    let font = match font_cache.get(&font_id) {
        Some(f) => f,
        None => font_cache.get(&FontId::MainRegular)?,
    };

    let ch = char::from_u32(char_code).unwrap_or('?');
    let glyph_id = font.glyph_id(ch);

    if glyph_id.0 == 0 {
        let fallback = font_cache.get(&FontId::MainRegular)?;
        let fid = fallback.glyph_id(ch);
        if fid.0 == 0 {
            return None;
        }
        return outline_to_d(px, py, glyph_em, fallback, fid);
    }

    outline_to_d(px, py, glyph_em, font, glyph_id)
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
