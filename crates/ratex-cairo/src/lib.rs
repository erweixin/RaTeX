use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use ab_glyph::{Font, FontRef, OutlineCurve};
use ratex_font::FontId;
use ratex_font_loader::FontSet;
use ratex_types::{Color, DisplayItem, DisplayList, PathCommand};
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct CairoOptions {
    pub font_size: f64,
    pub padding: f64,
    pub font_dir: Option<PathBuf>,
}

impl Default for CairoOptions {
    fn default() -> Self {
        Self {
            font_size: 24.0,
            padding: 4.0,
            font_dir: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderMetrics {
    pub width: f64,
    pub total_height: f64,
    pub baseline: f64,
}

#[derive(Debug, Error)]
pub enum CairoError {
    #[error("font error: {0}")]
    Font(String),
    #[error("cairo error: {0}")]
    Cairo(String),
}

pub fn measure_display_list(display_list: &DisplayList, options: &CairoOptions) -> RenderMetrics {
    let width = display_list.width * options.font_size + 2.0 * options.padding;
    let baseline = display_list.height * options.font_size + options.padding;
    let total_height = display_list.total_height() * options.font_size + 2.0 * options.padding;
    RenderMetrics {
        width: width.max(1.0),
        total_height: total_height.max(1.0),
        baseline: baseline.max(0.0),
    }
}

pub fn render_to_cairo(
    cr: &cairo::Context,
    display_list: &DisplayList,
    options: &CairoOptions,
) -> Result<(), CairoError> {
    cr.set_antialias(cairo::Antialias::Best);

    let font_dir = options
        .font_dir
        .as_deref()
        .and_then(Path::to_str)
        .unwrap_or("");
    let fonts = ratex_font_loader::load_fonts_for_items(font_dir, &display_list.items)
        .map_err(CairoError::Font)?;
    let font_refs = build_font_refs(&fonts).map_err(CairoError::Font)?;

    let em = options.font_size as f32;
    let pad = options.padding as f32;
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
                let font_id = *font_id_cache
                    .entry(font.as_str())
                    .or_insert_with(|| FontId::parse(font).unwrap_or(FontId::MainRegular));
                render_glyph(
                    cr,
                    *x as f32 * em + pad,
                    *y as f32 * em + pad,
                    font_id,
                    *char_code,
                    *color,
                    &font_refs,
                    *scale as f32 * em,
                )?;
            }
            DisplayItem::Line {
                x,
                y,
                width,
                thickness,
                color,
                dashed,
            } => render_line(
                cr,
                *x as f32 * em + pad,
                *y as f32 * em + pad,
                *width as f32 * em,
                *thickness as f32 * em,
                *color,
                *dashed,
            )?,
            DisplayItem::Rect {
                x,
                y,
                width,
                height,
                color,
            } => render_rect(
                cr,
                *x as f32 * em + pad,
                *y as f32 * em + pad,
                *width as f32 * em,
                *height as f32 * em,
                *color,
            )?,
            DisplayItem::Path {
                x,
                y,
                commands,
                fill,
                color,
            } => render_path(
                cr,
                *x as f32 * em + pad,
                *y as f32 * em + pad,
                commands,
                *fill,
                *color,
                em,
            )?,
        }
    }

    Ok(())
}

fn build_font_refs(data: &FontSet) -> Result<HashMap<FontId, FontRef<'_>>, String> {
    let mut font_refs = HashMap::new();
    for (id, bytes) in data.iter() {
        let font = FontRef::try_from_slice_and_index(bytes, sfnt_collection_index(*id))
            .map_err(|e| format!("Failed to parse font {:?}: {}", id, e))?;
        font_refs.insert(*id, font);
    }

    if !font_refs.contains_key(&FontId::MainRegular) {
        return Err("Main-Regular font not found".to_string());
    }

    Ok(font_refs)
}

fn sfnt_collection_index(id: FontId) -> u32 {
    match id {
        FontId::EmojiFallback => ratex_unicode_font::emoji_font_face_index().unwrap_or(0),
        FontId::CjkRegular => ratex_unicode_font::unicode_font_face_index().unwrap_or(0),
        FontId::CjkFallback => ratex_unicode_font::fallback_font_face_index().unwrap_or(0),
        _ => 0,
    }
}

fn render_glyph(
    cr: &cairo::Context,
    px: f32,
    py: f32,
    font_id: FontId,
    char_code: u32,
    color: Color,
    font_cache: &HashMap<FontId, FontRef<'_>>,
    em: f32,
) -> Result<(), CairoError> {
    let font = match font_cache.get(&font_id) {
        Some(font) => font,
        None => font_cache
            .get(&FontId::MainRegular)
            .ok_or_else(|| CairoError::Font("Main-Regular font not found".to_string()))?,
    };

    let ch = ratex_font::katex_ttf_glyph_char(font_id, char_code);
    let glyph_id = font.glyph_id(ch);

    if glyph_id.0 == 0 {
        let _ = try_system_unicode_fallback(cr, px, py, ch, color, em, font_cache, false)?;
        return Ok(());
    }

    if font_id == FontId::EmojiFallback {
        if try_draw_emoji_png(cr, px, py, em, ch)? {
            return Ok(());
        }
        if render_glyph_with_font(
            cr,
            px,
            py,
            FontGlyph {
                font_id,
                font,
                glyph_id,
            },
            color,
            em,
        )? {
            return Ok(());
        }
    }

    if font_id == FontId::CjkRegular {
        if render_glyph_with_font(
            cr,
            px,
            py,
            FontGlyph {
                font_id,
                font,
                glyph_id,
            },
            color,
            em,
        )? {
            return Ok(());
        }
        if try_draw_emoji_or_outline(cr, px, py, ch, color, em, font_cache)? {
            return Ok(());
        }
        if let Some(fallback_font) = font_cache.get(&FontId::CjkFallback) {
            let fallback_id = fallback_font.glyph_id(ch);
            if fallback_id.0 != 0 {
                let _ = render_glyph_with_font(
                    cr,
                    px,
                    py,
                    FontGlyph {
                        font_id: FontId::CjkFallback,
                        font: fallback_font,
                        glyph_id: fallback_id,
                    },
                    color,
                    em,
                )?;
            }
        }
        return Ok(());
    }

    if font_id == FontId::CjkFallback {
        if render_glyph_with_font(
            cr,
            px,
            py,
            FontGlyph {
                font_id,
                font,
                glyph_id,
            },
            color,
            em,
        )? {
            return Ok(());
        }
        let _ = try_draw_emoji_or_outline(cr, px, py, ch, color, em, font_cache)?;
        return Ok(());
    }

    if render_glyph_with_font(
        cr,
        px,
        py,
        FontGlyph {
            font_id,
            font,
            glyph_id,
        },
        color,
        em,
    )? {
        return Ok(());
    }

    let skip_main = font_id == FontId::MainRegular;
    let _ = try_system_unicode_fallback(cr, px, py, ch, color, em, font_cache, skip_main)?;
    Ok(())
}

fn try_system_unicode_fallback(
    cr: &cairo::Context,
    px: f32,
    py: f32,
    ch: char,
    color: Color,
    em: f32,
    font_cache: &HashMap<FontId, FontRef<'_>>,
    skip_main_regular: bool,
) -> Result<bool, CairoError> {
    if !skip_main_regular {
        if let Some(font) = font_cache.get(&FontId::MainRegular) {
            let glyph_id = font.glyph_id(ch);
            if glyph_id.0 != 0
                && render_glyph_with_font(
                    cr,
                    px,
                    py,
                    FontGlyph {
                        font_id: FontId::MainRegular,
                        font,
                        glyph_id,
                    },
                    color,
                    em,
                )?
            {
                return Ok(true);
            }
        }
    }

    if let Some(font) = font_cache.get(&FontId::CjkRegular) {
        let glyph_id = font.glyph_id(ch);
        if glyph_id.0 != 0
            && render_glyph_with_font(
                cr,
                px,
                py,
                FontGlyph {
                    font_id: FontId::CjkRegular,
                    font,
                    glyph_id,
                },
                color,
                em,
            )?
        {
            return Ok(true);
        }
    }

    if try_draw_emoji_or_outline(cr, px, py, ch, color, em, font_cache)? {
        return Ok(true);
    }

    if let Some(font) = font_cache.get(&FontId::CjkFallback) {
        let glyph_id = font.glyph_id(ch);
        if glyph_id.0 != 0
            && render_glyph_with_font(
                cr,
                px,
                py,
                FontGlyph {
                    font_id: FontId::CjkFallback,
                    font,
                    glyph_id,
                },
                color,
                em,
            )?
        {
            return Ok(true);
        }
    }

    Ok(false)
}

fn try_draw_emoji_or_outline(
    cr: &cairo::Context,
    px: f32,
    py: f32,
    ch: char,
    color: Color,
    em: f32,
    font_cache: &HashMap<FontId, FontRef<'_>>,
) -> Result<bool, CairoError> {
    if try_draw_emoji_png(cr, px, py, em, ch)? {
        return Ok(true);
    }

    if let Some(font) = font_cache.get(&FontId::EmojiFallback) {
        let glyph_id = font.glyph_id(ch);
        if glyph_id.0 != 0 {
            return render_glyph_with_font(
                cr,
                px,
                py,
                FontGlyph {
                    font_id: FontId::EmojiFallback,
                    font,
                    glyph_id,
                },
                color,
                em,
            );
        }
    }

    Ok(false)
}

struct FontGlyph<'a> {
    font_id: FontId,
    font: &'a FontRef<'a>,
    glyph_id: ab_glyph::GlyphId,
}

fn render_glyph_with_font(
    cr: &cairo::Context,
    px: f32,
    py: f32,
    glyph: FontGlyph<'_>,
    color: Color,
    em: f32,
) -> Result<bool, CairoError> {
    let curves = match ratex_font_loader::outline_cache::get_or_compute_outline(
        glyph.font_id,
        glyph.font,
        glyph.glyph_id,
    ) {
        Some(curves) => curves,
        None => return Ok(false),
    };
    if curves.is_empty() {
        return Ok(false);
    }

    let units_per_em = glyph.font.units_per_em().unwrap_or(1000.0);
    let mut scale = em / units_per_em;
    if glyph.font_id == FontId::EmojiFallback {
        let actual_advance = glyph.font.h_advance_unscaled(glyph.glyph_id);
        let actual_advance_em = actual_advance / units_per_em;
        if actual_advance_em > 0.01 && actual_advance_em > 1.01 {
            scale *= 1.0 / actual_advance_em;
        }
    }

    append_outline_path(cr, &curves, px, py, scale);
    set_source_color(cr, color);
    cr.fill()
        .map_err(|err| CairoError::Cairo(err.to_string()))?;
    Ok(true)
}

fn append_outline_path(cr: &cairo::Context, curves: &[OutlineCurve], px: f32, py: f32, scale: f32) {
    let mut last_end: Option<(f32, f32)> = None;

    for curve in curves {
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
                cr.close_path();
            }
            cr.move_to(start.0 as f64, start.1 as f64);
        }

        match curve {
            OutlineCurve::Line(_, p1) => {
                cr.line_to((px + p1.x * scale) as f64, (py - p1.y * scale) as f64);
            }
            OutlineCurve::Quad(p0, p1, p2) => {
                let p0x = px + p0.x * scale;
                let p0y = py - p0.y * scale;
                let p1x = px + p1.x * scale;
                let p1y = py - p1.y * scale;
                let ex = px + p2.x * scale;
                let ey = py - p2.y * scale;
                let c1x = p0x + (2.0 / 3.0) * (p1x - p0x);
                let c1y = p0y + (2.0 / 3.0) * (p1y - p0y);
                let c2x = ex + (2.0 / 3.0) * (p1x - ex);
                let c2y = ey + (2.0 / 3.0) * (p1y - ey);
                cr.curve_to(
                    c1x as f64, c1y as f64, c2x as f64, c2y as f64, ex as f64, ey as f64,
                );
            }
            OutlineCurve::Cubic(_, p1, p2, p3) => {
                cr.curve_to(
                    (px + p1.x * scale) as f64,
                    (py - p1.y * scale) as f64,
                    (px + p2.x * scale) as f64,
                    (py - p2.y * scale) as f64,
                    (px + p3.x * scale) as f64,
                    (py - p3.y * scale) as f64,
                );
            }
        }

        last_end = Some(end);
    }

    if last_end.is_some() {
        cr.close_path();
    }
}

fn try_draw_emoji_png(
    cr: &cairo::Context,
    px: f32,
    py: f32,
    em: f32,
    ch: char,
) -> Result<bool, CairoError> {
    let strike = match ratex_unicode_font::emoji_png_raster_for_char(ch, em) {
        Some(strike) => strike,
        None => return Ok(false),
    };

    let ppm = f32::from(strike.pixels_per_em.max(1));
    let mut scale = em / ppm;
    let actual_width_em = f32::from(strike.width) / ppm;
    if actual_width_em > 0.01 && actual_width_em > 1.01 {
        scale *= 1.0 / actual_width_em;
    }

    let top_x = px + f32::from(strike.x) * scale;
    let mut top_y = py - (f32::from(strike.y) + f32::from(strike.height)) * scale;
    let center_strike = (f32::from(strike.y) + f32::from(strike.height) / 2.0) / ppm;
    let axis = ratex_font::get_global_metrics(0).axis_height as f32;
    top_y += (center_strike - axis) * em;

    let mut cursor = Cursor::new(strike.data);
    let surface = cairo::ImageSurface::create_from_png(&mut cursor)
        .map_err(|err| CairoError::Cairo(err.to_string()))?;

    cr.save()
        .map_err(|err| CairoError::Cairo(err.to_string()))?;
    cr.translate(top_x as f64, top_y as f64);
    cr.scale(scale as f64, scale as f64);
    cr.set_source_surface(&surface, 0.0, 0.0)
        .map_err(|err| CairoError::Cairo(err.to_string()))?;
    cr.paint()
        .map_err(|err| CairoError::Cairo(err.to_string()))?;
    cr.restore()
        .map_err(|err| CairoError::Cairo(err.to_string()))?;
    Ok(true)
}

fn render_line(
    cr: &cairo::Context,
    x: f32,
    y: f32,
    width: f32,
    thickness: f32,
    color: Color,
    dashed: bool,
) -> Result<(), CairoError> {
    let t = thickness.max(0.5);
    set_source_color(cr, color);
    if dashed {
        cr.save()
            .map_err(|err| CairoError::Cairo(err.to_string()))?;
        cr.set_line_width(t as f64);
        cr.set_dash(&[(t * 3.0) as f64, (t * 3.0) as f64], 0.0);
        cr.move_to(x as f64, y as f64);
        cr.line_to((x + width) as f64, y as f64);
        cr.stroke()
            .map_err(|err| CairoError::Cairo(err.to_string()))?;
        cr.restore()
            .map_err(|err| CairoError::Cairo(err.to_string()))?;
    } else {
        cr.rectangle(x as f64, (y - t / 2.0) as f64, width as f64, t as f64);
        cr.fill()
            .map_err(|err| CairoError::Cairo(err.to_string()))?;
    }
    Ok(())
}

fn render_rect(
    cr: &cairo::Context,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
) -> Result<(), CairoError> {
    set_source_color(cr, color);
    cr.rectangle(
        x as f64,
        y as f64,
        width.max(1.0) as f64,
        height.max(1.0) as f64,
    );
    cr.fill()
        .map_err(|err| CairoError::Cairo(err.to_string()))?;
    Ok(())
}

fn render_path(
    cr: &cairo::Context,
    x: f32,
    y: f32,
    commands: &[PathCommand],
    fill: bool,
    color: Color,
    em: f32,
) -> Result<(), CairoError> {
    set_source_color(cr, color);
    if fill {
        let mut start = 0;
        for i in 1..commands.len() {
            if matches!(commands[i], PathCommand::MoveTo { .. }) {
                render_path_segment(cr, x, y, &commands[start..i], true, color, em)?;
                start = i;
            }
        }
        render_path_segment(cr, x, y, &commands[start..], true, color, em)?;
    } else {
        render_path_segment(cr, x, y, commands, false, color, em)?;
    }
    Ok(())
}

fn render_path_segment(
    cr: &cairo::Context,
    x: f32,
    y: f32,
    commands: &[PathCommand],
    fill: bool,
    color: Color,
    em: f32,
) -> Result<(), CairoError> {
    cr.new_path();
    for command in commands {
        match command {
            PathCommand::MoveTo { x: cx, y: cy } => {
                cr.move_to((x + *cx as f32 * em) as f64, (y + *cy as f32 * em) as f64);
            }
            PathCommand::LineTo { x: cx, y: cy } => {
                cr.line_to((x + *cx as f32 * em) as f64, (y + *cy as f32 * em) as f64);
            }
            PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x: cx,
                y: cy,
            } => {
                cr.curve_to(
                    (x + *x1 as f32 * em) as f64,
                    (y + *y1 as f32 * em) as f64,
                    (x + *x2 as f32 * em) as f64,
                    (y + *y2 as f32 * em) as f64,
                    (x + *cx as f32 * em) as f64,
                    (y + *cy as f32 * em) as f64,
                );
            }
            PathCommand::QuadTo {
                x1,
                y1,
                x: cx,
                y: cy,
            } => {
                let start = cr.current_point().unwrap_or((x as f64, y as f64));
                let start_x = start.0 as f32;
                let start_y = start.1 as f32;
                let c1x = start_x + (2.0 / 3.0) * ((x + *x1 as f32 * em) - start_x);
                let c1y = start_y + (2.0 / 3.0) * ((y + *y1 as f32 * em) - start_y);
                let end_x = x + *cx as f32 * em;
                let end_y = y + *cy as f32 * em;
                let c2x = end_x + (2.0 / 3.0) * ((x + *x1 as f32 * em) - end_x);
                let c2y = end_y + (2.0 / 3.0) * ((y + *y1 as f32 * em) - end_y);
                cr.curve_to(
                    c1x as f64,
                    c1y as f64,
                    c2x as f64,
                    c2y as f64,
                    end_x as f64,
                    end_y as f64,
                );
            }
            PathCommand::Close => cr.close_path(),
        }
    }

    set_source_color(cr, color);
    if fill {
        cr.set_fill_rule(cairo::FillRule::EvenOdd);
        cr.fill()
            .map_err(|err| CairoError::Cairo(err.to_string()))?;
    } else {
        cr.save()
            .map_err(|err| CairoError::Cairo(err.to_string()))?;
        cr.set_line_width(1.5);
        cr.stroke()
            .map_err(|err| CairoError::Cairo(err.to_string()))?;
        cr.restore()
            .map_err(|err| CairoError::Cairo(err.to_string()))?;
    }
    Ok(())
}

fn set_source_color(cr: &cairo::Context, color: Color) {
    cr.set_source_rgba(
        color.r.clamp(0.0, 1.0) as f64,
        color.g.clamp(0.0, 1.0) as f64,
        color.b.clamp(0.0, 1.0) as f64,
        color.a.clamp(0.0, 1.0) as f64,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratex_layout::{layout, to_display_list, LayoutOptions};
    use ratex_parser::parse;

    #[test]
    fn render_metrics_include_padding() {
        let display_list = DisplayList {
            items: Vec::new(),
            width: 2.0,
            height: 1.0,
            depth: 0.5,
        };
        let metrics = measure_display_list(
            &display_list,
            &CairoOptions {
                font_size: 20.0,
                padding: 3.0,
                font_dir: None,
            },
        );

        assert_eq!(metrics.width, 46.0);
        assert_eq!(metrics.total_height, 36.0);
        assert_eq!(metrics.baseline, 23.0);
    }

    #[test]
    fn render_simple_formula_to_image_surface() {
        let ast = parse(r"\frac{1}{2}").expect("formula should parse");
        let layout = layout(&ast, &LayoutOptions::default());
        let display_list = to_display_list(&layout);
        let options = CairoOptions::default();
        let metrics = measure_display_list(&display_list, &options);

        let surface = cairo::ImageSurface::create(
            cairo::Format::ARgb32,
            metrics.width.ceil() as i32,
            metrics.total_height.ceil() as i32,
        )
        .expect("image surface should be created");
        let cr = cairo::Context::new(&surface).expect("context should be created");

        render_to_cairo(&cr, &display_list, &options).expect("render should succeed");
    }
}
