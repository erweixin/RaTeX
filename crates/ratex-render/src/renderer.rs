use std::collections::HashMap;
use std::path::Path;

use ab_glyph::{Font, FontRef};
use ratex_font::FontId;
use ratex_types::color::Color;
use ratex_types::display_item::{DisplayItem, DisplayList};
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Stroke, Transform};

pub struct RenderOptions {
    pub font_size: f32,
    pub padding: f32,
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
            } => {
                render_line(
                    &mut pixmap,
                    *x as f32 * em_px + pad_px,
                    *y as f32 * em_px + pad_px,
                    *width as f32 * em_px,
                    *thickness as f32 * em_px,
                    color,
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
        return Err(format!("No fonts found in {}", font_dir));
    }

    Ok(data)
}

fn build_font_cache(data: &HashMap<FontId, Vec<u8>>) -> Result<HashMap<FontId, FontRef<'_>>, String> {
    let mut cache = HashMap::new();
    for (id, bytes) in data {
        let font = FontRef::try_from_slice(bytes)
            .map_err(|e| format!("Failed to parse font {:?}: {}", id, e))?;
        cache.insert(*id, font);
    }
    Ok(cache)
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

    let ch = char::from_u32(char_code).unwrap_or('?');
    let glyph_id = font.glyph_id(ch);

    if glyph_id.0 == 0 {
        if let Some(fallback) = font_cache.get(&FontId::MainRegular) {
            let fid = fallback.glyph_id(ch);
            if fid.0 != 0 {
                return render_glyph_with_font(pixmap, px, py, fallback, fid, color, em);
            }
        }
        return;
    }

    render_glyph_with_font(pixmap, px, py, font, glyph_id, color, em);
}

fn render_glyph_with_font(
    pixmap: &mut Pixmap,
    px: f32,
    py: f32,
    font: &FontRef<'_>,
    glyph_id: ab_glyph::GlyphId,
    color: &Color,
    em: f32,
) {
    let outline = match font.outline(glyph_id) {
        Some(o) => o,
        None => return,
    };

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
            tiny_skia::FillRule::EvenOdd,
            Transform::identity(),
            None,
        );
    }
}

fn render_line(pixmap: &mut Pixmap, x: f32, y: f32, width: f32, thickness: f32, color: &Color) {
    let t = thickness.max(1.0);
    let rect = tiny_skia::Rect::from_xywh(x, y - t / 2.0, width, t);
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

fn render_rect(pixmap: &mut Pixmap, x: f32, y: f32, width: f32, height: f32, color: &Color) {
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
            pixmap.fill_path(
                &path,
                &paint,
                FillRule::Winding,
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
