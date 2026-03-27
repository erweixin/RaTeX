//! Serialize a [`DisplayList`](ratex_types::display_item::DisplayList) to SVG.
//!
//! Coordinates match [`ratex_layout::to_display_list`](https://docs.rs/ratex-layout): **em** units
//! with **y downward** and the baseline at `y = height` in layout space. They are scaled by
//! [`SvgOptions::font_size`] plus [`SvgOptions::padding`], same convention as `ratex-render`.
//!
//! **Glyphs (default):** each [`DisplayItem::GlyphPath`](ratex_types::display_item::DisplayItem::GlyphPath)
//! becomes a `<text>` element using KaTeX CSS `font-family` names (`KaTeX_Main`, `KaTeX_Math`, …).
//! Load [KaTeX](https://katex.org/) stylesheets in the host page for correct shapes.
//!
//! **Self-contained SVG:** enable Cargo feature `standalone` and set [`SvgOptions::embed_glyphs`]
//! with a KaTeX `font_dir` to emit glyph outlines as `<path>` (no webfonts), matching
//! `ratex-render`'s `ab_glyph` geometry.

use ratex_types::color::Color;
use ratex_types::display_item::{DisplayItem, DisplayList};
use ratex_types::path_command::PathCommand;

#[cfg(feature = "standalone")]
mod standalone;

/// Options controlling SVG size and stroke appearance.
#[derive(Debug, Clone)]
pub struct SvgOptions {
    /// User units per em (matches `ratex_render::RenderOptions::font_size` at DPR 1).
    pub font_size: f64,
    /// Padding on all sides, in the same user units as a pixel at DPR 1 when `font_size` is 40.
    pub padding: f64,
    /// Stroke width for unfilled [`DisplayItem::Path`](DisplayItem::Path), in user units.
    pub stroke_width: f64,
    /// When the `standalone` feature is enabled and this is `true`, glyphs are drawn as filled
    /// `<path>` outlines using fonts from [`Self::font_dir`]. Otherwise a `<text>` element with
    /// KaTeX CSS `font-family` names is emitted.
    pub embed_glyphs: bool,
    /// Directory containing KaTeX `.ttf` files (same as `ratex_render::RenderOptions::font_dir`).
    pub font_dir: String,
}

impl Default for SvgOptions {
    fn default() -> Self {
        Self {
            font_size: 40.0,
            padding: 10.0,
            stroke_width: 1.5,
            embed_glyphs: false,
            font_dir: String::new(),
        }
    }
}

impl SvgOptions {
    fn em_px(&self) -> f64 {
        self.font_size
    }
}

/// Render a display list to a standalone SVG document string.
pub fn render_to_svg(list: &DisplayList, opts: &SvgOptions) -> String {
    #[cfg(feature = "standalone")]
    let font_data = if opts.embed_glyphs && !opts.font_dir.is_empty() {
        standalone::load_all_fonts(&opts.font_dir).ok()
    } else {
        None
    };
    #[cfg(feature = "standalone")]
    let font_cache = font_data
        .as_ref()
        .and_then(|d| standalone::build_font_cache(d).ok());

    let em = opts.em_px();
    let pad = opts.padding;
    let total_h = list.height + list.depth;
    let vb_w = list.width * em + 2.0 * pad;
    let vb_h = total_h * em + 2.0 * pad;

    let mut body = String::new();
    for item in &list.items {
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
                let g = GlyphEmit {
                    x: *x,
                    y: *y,
                    scale: *scale,
                    font: font.as_str(),
                    char_code: *char_code,
                    color,
                };
                #[cfg(feature = "standalone")]
                emit_glyph(&mut body, g, opts, font_cache.as_ref());
                #[cfg(not(feature = "standalone"))]
                emit_glyph_text(&mut body, g, opts);
            }
            DisplayItem::Line {
                x,
                y,
                width,
                thickness,
                color,
                ..
            } => emit_line(&mut body, *x, *y, *width, *thickness, color, opts),
            DisplayItem::Rect {
                x,
                y,
                width,
                height,
                color,
            } => emit_rect(&mut body, *x, *y, *width, *height, color, opts),
            DisplayItem::Path {
                x,
                y,
                commands,
                fill,
                color,
            } => emit_path_item(&mut body, *x, *y, commands, *fill, color, opts),
        }
    }

    wrap_svg(vb_w, vb_h, &body)
}

fn wrap_svg(vb_w: f64, vb_h: f64, body: &str) -> String {
    let w = fmt_num(vb_w);
    let h = fmt_num(vb_h);
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}">{body}</svg>"#,
        w = w,
        h = h,
        body = body,
    )
}

fn tx(x_em: f64, opts: &SvgOptions) -> f64 {
    x_em * opts.em_px() + opts.padding
}

fn ty(y_em: f64, opts: &SvgOptions) -> f64 {
    y_em * opts.em_px() + opts.padding
}

fn color_to_svg(c: &Color) -> String {
    let r = (c.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (c.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (c.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    let a = c.a.clamp(0.0, 1.0);
    format!("rgba({r},{g},{b},{a})")
}

pub(crate) fn fmt_num(n: f64) -> String {
    let s = format!("{n:.6}");
    let s = s.trim_end_matches('0');
    let s = s.trim_end_matches('.');
    if s.is_empty() || s == "-" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

fn xml_escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

/// Map internal font id string (e.g. `Main-Regular`) to KaTeX CSS `font-family` and face attributes.
struct GlyphEmit<'a> {
    x: f64,
    y: f64,
    scale: f64,
    font: &'a str,
    char_code: u32,
    color: &'a Color,
}

fn katex_face(font: &str) -> (&'static str, &'static str, &'static str) {
    match font {
        "Main-Regular" => ("KaTeX_Main", "normal", "normal"),
        "Main-Bold" => ("KaTeX_Main", "bold", "normal"),
        "Main-Italic" => ("KaTeX_Main", "normal", "italic"),
        "Main-BoldItalic" => ("KaTeX_Main", "bold", "italic"),
        "Math-Italic" => ("KaTeX_Math", "normal", "italic"),
        "Math-BoldItalic" => ("KaTeX_Math", "bold", "italic"),
        "AMS-Regular" => ("KaTeX_AMS", "normal", "normal"),
        "Caligraphic-Regular" => ("KaTeX_Caligraphic", "normal", "normal"),
        "Fraktur-Regular" => ("KaTeX_Fraktur", "normal", "normal"),
        "SansSerif-Regular" => ("KaTeX_SansSerif", "normal", "normal"),
        "SansSerif-Bold" => ("KaTeX_SansSerif", "bold", "normal"),
        "SansSerif-Italic" => ("KaTeX_SansSerif", "normal", "italic"),
        "Script-Regular" => ("KaTeX_Script", "normal", "normal"),
        "Typewriter-Regular" => ("KaTeX_Typewriter", "normal", "normal"),
        "Size1-Regular" => ("KaTeX_Size1", "normal", "normal"),
        "Size2-Regular" => ("KaTeX_Size2", "normal", "normal"),
        "Size3-Regular" => ("KaTeX_Size3", "normal", "normal"),
        "Size4-Regular" => ("KaTeX_Size4", "normal", "normal"),
        _ => ("KaTeX_Main", "normal", "normal"),
    }
}

#[cfg(feature = "standalone")]
fn emit_glyph(
    out: &mut String,
    g: GlyphEmit<'_>,
    opts: &SvgOptions,
    font_cache: Option<&std::collections::HashMap<ratex_font::FontId, ab_glyph::FontRef<'_>>>,
) {
    if opts.embed_glyphs {
        if let Some(cache) = font_cache {
            let px = tx(g.x, opts) as f32;
            let py = ty(g.y, opts) as f32;
            let glyph_em = (g.scale * opts.em_px()) as f32;
            if let Some(d) =
                standalone::glyph_svg_path(px, py, glyph_em, g.font, g.char_code, cache)
            {
                let fill = color_to_svg(g.color);
                use std::fmt::Write;
                let _ = write!(
                    out,
                    r#"<path d="{d}" fill="{fill}" fill-rule="evenodd" stroke="none"/>"#
                );
                return;
            }
        }
    }
    emit_glyph_text(out, g, opts);
}

fn emit_glyph_text(out: &mut String, g: GlyphEmit<'_>, opts: &SvgOptions) {
    let ch = char::from_u32(g.char_code).unwrap_or('\u{fffd}');
    let text = xml_escape_text(&ch.to_string());
    let (family, weight, style) = katex_face(g.font);
    let fs = g.scale * opts.em_px();
    let fill = color_to_svg(g.color);
    let x_s = fmt_num(tx(g.x, opts));
    let y_s = fmt_num(ty(g.y, opts));
    let fs_s = fmt_num(fs);
    use std::fmt::Write;
    let _ = write!(
        out,
        r#"<text x="{x_s}" y="{y_s}" font-family="{family}" font-size="{fs_s}" font-weight="{weight}" font-style="{style}" fill="{fill}" dominant-baseline="alphabetic">{text}</text>"#
    );
}

fn emit_line(
    out: &mut String,
    x: f64,
    y: f64,
    width: f64,
    thickness: f64,
    color: &Color,
    opts: &SvgOptions,
) {
    let em = opts.em_px();
    let x0 = tx(x, opts);
    let yc = ty(y, opts);
    let t = (thickness * em).max(1e-6);
    let y0 = yc - t / 2.0;
    let w = width * em;
    let fill = color_to_svg(color);
    let x0s = fmt_num(x0);
    let y0s = fmt_num(y0);
    let ws = fmt_num(w);
    let hs = fmt_num(t);
    use std::fmt::Write;
    let _ = write!(
        out,
        r#"<rect x="{x0s}" y="{y0s}" width="{ws}" height="{hs}" fill="{fill}"/>"#
    );
}

fn emit_rect(
    out: &mut String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    color: &Color,
    opts: &SvgOptions,
) {
    let em = opts.em_px();
    let x0 = tx(x, opts);
    let y0 = ty(y, opts);
    let w = width * em;
    let h = height * em;
    let fill = color_to_svg(color);
    let x0s = fmt_num(x0);
    let y0s = fmt_num(y0);
    let ws = fmt_num(w);
    let hs = fmt_num(h);
    use std::fmt::Write;
    let _ = write!(
        out,
        r#"<rect x="{x0s}" y="{y0s}" width="{ws}" height="{hs}" fill="{fill}"/>"#
    );
}

fn path_commands_to_d(origin_x: f64, origin_y: f64, em: f64, commands: &[PathCommand]) -> String {
    let mut d = String::new();
    for cmd in commands {
        match cmd {
            PathCommand::MoveTo { x, y } => {
                d.push('M');
                d.push_str(&fmt_num(origin_x + x * em));
                d.push(' ');
                d.push_str(&fmt_num(origin_y + y * em));
            }
            PathCommand::LineTo { x, y } => {
                d.push('L');
                d.push_str(&fmt_num(origin_x + x * em));
                d.push(' ');
                d.push_str(&fmt_num(origin_y + y * em));
            }
            PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                d.push('C');
                d.push_str(&fmt_num(origin_x + x1 * em));
                d.push(' ');
                d.push_str(&fmt_num(origin_y + y1 * em));
                d.push(' ');
                d.push_str(&fmt_num(origin_x + x2 * em));
                d.push(' ');
                d.push_str(&fmt_num(origin_y + y2 * em));
                d.push(' ');
                d.push_str(&fmt_num(origin_x + x * em));
                d.push(' ');
                d.push_str(&fmt_num(origin_y + y * em));
            }
            PathCommand::QuadTo { x1, y1, x, y } => {
                d.push('Q');
                d.push_str(&fmt_num(origin_x + x1 * em));
                d.push(' ');
                d.push_str(&fmt_num(origin_y + y1 * em));
                d.push(' ');
                d.push_str(&fmt_num(origin_x + x * em));
                d.push(' ');
                d.push_str(&fmt_num(origin_y + y * em));
            }
            PathCommand::Close => d.push('Z'),
        }
        d.push(' ');
    }
    d.trim_end().to_string()
}

fn emit_path_item(
    out: &mut String,
    x: f64,
    y: f64,
    commands: &[PathCommand],
    fill: bool,
    color: &Color,
    opts: &SvgOptions,
) {
    let em = opts.em_px();
    let ox = tx(x, opts);
    let oy = ty(y, opts);
    let paint = color_to_svg(color);

    if fill {
        let mut start = 0usize;
        for i in 1..commands.len() {
            if matches!(commands[i], PathCommand::MoveTo { .. }) {
                let seg = &commands[start..i];
                start = i;
                if seg.is_empty() {
                    continue;
                }
                let d = path_commands_to_d(ox, oy, em, seg);
                if d.is_empty() {
                    continue;
                }
                use std::fmt::Write;
                let _ = write!(
                    out,
                    r#"<path d="{d}" fill="{paint}" fill-rule="nonzero" stroke="none"/>"#
                );
            }
        }
        let seg = &commands[start..];
        if !seg.is_empty() {
            let d = path_commands_to_d(ox, oy, em, seg);
            if !d.is_empty() {
                use std::fmt::Write;
                let _ = write!(
                    out,
                    r#"<path d="{d}" fill="{paint}" fill-rule="nonzero" stroke="none"/>"#
                );
            }
        }
    } else {
        let d = path_commands_to_d(ox, oy, em, commands);
        if d.is_empty() {
            return;
        }
        let sw = fmt_num(opts.stroke_width);
        use std::fmt::Write;
        let _ = write!(
            out,
            r#"<path d="{d}" fill="none" stroke="{paint}" stroke-width="{sw}" stroke-linecap="round" stroke-linejoin="round"/>"#
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratex_types::path_command::PathCommand;

    #[test]
    fn empty_list_produces_svg() {
        let list = DisplayList {
            items: vec![],
            width: 2.0,
            height: 1.0,
            depth: 0.5,
        };
        let svg = render_to_svg(&list, &SvgOptions::default());
        assert!(svg.starts_with("<svg "));
        assert!(svg.contains("viewBox=\"0 0 100 80\""));
        assert!(svg.ends_with("</svg>"));
    }

    #[test]
    fn line_rect_path_glyph_roundtrip_structure() {
        let list = DisplayList {
            items: vec![
                DisplayItem::Line {
                    x: 0.0,
                    y: 0.5,
                    width: 1.0,
                    thickness: 0.04,
                    color: Color::BLACK,
                    dashed: false,
                },
                DisplayItem::Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 0.5,
                    height: 0.2,
                    color: Color::rgb(1.0, 0.0, 0.0),
                },
                DisplayItem::Path {
                    x: 0.0,
                    y: 0.0,
                    commands: vec![
                        PathCommand::MoveTo { x: 0.0, y: 0.0 },
                        PathCommand::LineTo { x: 1.0, y: 0.0 },
                    ],
                    fill: false,
                    color: Color::BLACK,
                },
                DisplayItem::GlyphPath {
                    x: 0.1,
                    y: 0.8,
                    scale: 1.0,
                    font: "Math-Italic".to_string(),
                    char_code: b'x' as u32,
                    commands: vec![],
                    color: Color::BLACK,
                },
            ],
            width: 2.0,
            height: 1.0,
            depth: 0.0,
        };
        let svg = render_to_svg(
            &list,
            &SvgOptions {
                font_size: 10.0,
                padding: 0.0,
                stroke_width: 1.0,
                embed_glyphs: false,
                font_dir: String::new(),
            },
        );
        assert!(svg.contains("<rect"));
        assert!(svg.contains("<path"));
        assert!(svg.contains("<text"));
        assert!(svg.contains("KaTeX_Math"));
        assert!(svg.contains("fill=\"rgba(255,0,0,1)\"") || svg.contains("fill=\"rgba(255,0,0,1"));
    }

    #[cfg(feature = "standalone")]
    #[test]
    fn embed_glyphs_use_path_when_katex_fonts_present() {
        use std::path::PathBuf;

        let font_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/lexer_compare/node_modules/katex/dist/fonts");
        if !font_dir.join("KaTeX_Math-Italic.ttf").exists() {
            return;
        }

        let list = DisplayList {
            items: vec![DisplayItem::GlyphPath {
                x: 0.1,
                y: 0.8,
                scale: 1.0,
                font: "Math-Italic".to_string(),
                char_code: b'x' as u32,
                commands: vec![],
                color: Color::BLACK,
            }],
            width: 1.0,
            height: 1.0,
            depth: 0.0,
        };
        let svg = render_to_svg(
            &list,
            &SvgOptions {
                font_size: 10.0,
                padding: 0.0,
                stroke_width: 1.0,
                embed_glyphs: true,
                font_dir: font_dir.to_string_lossy().into(),
            },
        );
        assert!(svg.contains("<path"));
        assert!(svg.contains("fill-rule=\"evenodd\""));
        assert!(!svg.contains("<text"));
    }
}
