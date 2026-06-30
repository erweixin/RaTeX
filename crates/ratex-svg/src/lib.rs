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
//! **Self-contained SVG:** enable Cargo feature `standalone`, then set
//! [`SvgOptions::embed_glyphs`] to output glyphs as `<path>` or `<image>` instead of `<text>`.
//! Without `embed-fonts`, this needs [`SvgOptions::font_dir`] pointing to KaTeX `.ttf` files.
//! With `embed-fonts`, `font_dir` is ignored and glyph bytes come from the embedded
//! `ratex-katex-fonts` crate. Color emoji prefer embedded PNG strikes and fall back to outline
//! paths only when no raster strike is available.

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
    /// When the `standalone` feature is enabled and this is `true`, glyphs are emitted as
    /// outlines/images instead of KaTeX `<text>` elements.
    pub embed_glyphs: bool,
    /// Directory containing KaTeX `.ttf` files. Used only when `embed-fonts` is disabled.
    pub font_dir: String,
}

/// SVG paint color syntax.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum SvgColorSyntax {
    /// Emit paint values as `rgba(r,g,b,a)`.
    #[default]
    Rgba,
    /// Emit paint values as `rgb(r,g,b)` and preserve alpha with opacity attributes.
    Rgb,
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

/// Render a display list to a standalone SVG document string using `rgba(...)` paint values.
pub fn render_to_svg(list: &DisplayList, opts: &SvgOptions) -> String {
    render_to_svg_with_color_syntax(list, opts, SvgColorSyntax::Rgba)
}

/// Render a display list to a standalone SVG document string with the requested paint syntax.
///
/// Use [`SvgColorSyntax::Rgb`] to emit `rgb(...)` paint values and preserve alpha with
/// SVG opacity attributes. [`render_to_svg`] retains the original `rgba(...)` behavior.
pub fn render_to_svg_with_color_syntax(
    list: &DisplayList,
    opts: &SvgOptions,
    color_syntax: SvgColorSyntax,
) -> String {
    let context = SvgRenderContext { opts, color_syntax };

    #[cfg(feature = "standalone")]
    #[cfg(not(feature = "embed-fonts"))]
    let load_fonts = opts.embed_glyphs && !opts.font_dir.is_empty();
    #[cfg(feature = "embed-fonts")]
    let load_fonts = opts.embed_glyphs;

    // Pre-render standalone glyphs while holding the font lock, then drop it.
    // This avoids self-referential struct issues with FontRef borrowing from the lock guard.
    #[cfg(feature = "standalone")]
    let prerendered_glyphs: Option<Vec<Option<standalone::StandaloneGlyph>>> = {
        if load_fonts {
            if let Ok(fonts) = ratex_font_loader::load_fonts_for_items(&opts.font_dir, &list.items)
            {
                if let Ok(font_refs) = standalone::build_font_refs(&fonts) {
                    let em = opts.em_px();
                    let pad = opts.padding;
                    let mut out = Vec::with_capacity(list.items.len());
                    for item in &list.items {
                        let glyph = if let DisplayItem::GlyphPath {
                            x,
                            y,
                            scale,
                            font,
                            char_code,
                            ..
                        } = item
                        {
                            let px = (*x * em + pad) as f32;
                            let py = (*y * em + pad) as f32;
                            let glyph_em = (*scale * em) as f32;
                            standalone::standalone_glyph(
                                px, py, glyph_em, font, *char_code, &font_refs,
                            )
                        } else {
                            None
                        };
                        out.push(glyph);
                    }
                    Some(out)
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    };

    let em = opts.em_px();
    let pad = opts.padding;
    let total_h = list.height + list.depth;
    let vb_w = list.width * em + 2.0 * pad;
    let vb_h = total_h * em + 2.0 * pad;

    let mut body = String::new();
    for (item_idx, item) in list.items.iter().enumerate() {
        #[cfg(not(feature = "standalone"))]
        let _ = item_idx;
        match item {
            DisplayItem::GlyphPath {
                x,
                y,
                scale,
                font,
                char_code,
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
                {
                    let prerendered = prerendered_glyphs
                        .as_ref()
                        .and_then(|v| v.get(item_idx).and_then(|g| g.as_ref()));
                    emit_glyph_standalone(&mut body, g, context, prerendered);
                }
                #[cfg(not(feature = "standalone"))]
                emit_glyph_text(&mut body, g, context);
            }
            DisplayItem::Line {
                x,
                y,
                width,
                thickness,
                color,
                dashed,
            } => emit_line(
                &mut body, *x, *y, *width, *thickness, color, *dashed, context,
            ),
            DisplayItem::Rect {
                x,
                y,
                width,
                height,
                color,
            } => emit_rect(&mut body, *x, *y, *width, *height, color, context),
            DisplayItem::Path {
                x,
                y,
                commands,
                fill,
                color,
            } => emit_path_item(&mut body, *x, *y, commands, *fill, color, context),
        }
    }

    wrap_svg(vb_w, vb_h, &body)
}

fn wrap_svg(vb_w: f64, vb_h: f64, body: &str) -> String {
    let w = fmt_num(vb_w);
    let h = fmt_num(vb_h);
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}pt" height="{h}pt">{body}</svg>"#
    )
}

fn tx(x_em: f64, opts: &SvgOptions) -> f64 {
    x_em * opts.em_px() + opts.padding
}

fn ty(y_em: f64, opts: &SvgOptions) -> f64 {
    y_em * opts.em_px() + opts.padding
}

fn color_to_svg(c: &Color, syntax: SvgColorSyntax) -> String {
    let r = (c.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (c.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (c.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    match syntax {
        SvgColorSyntax::Rgba => {
            let a = normalized_alpha(c.a);
            format!("rgba({r},{g},{b},{a})")
        }
        SvgColorSyntax::Rgb => format!("rgb({r},{g},{b})"),
    }
}

fn color_opacity_attr(c: &Color, attr: &str, syntax: SvgColorSyntax) -> String {
    if syntax != SvgColorSyntax::Rgb {
        return String::new();
    }

    let alpha = normalized_alpha(c.a);
    if alpha >= 1.0 {
        String::new()
    } else {
        let opacity = fmt_num(alpha as f64);
        format!(r#" {attr}="{opacity}""#)
    }
}

fn normalized_alpha(alpha: f32) -> f32 {
    if alpha.is_finite() {
        alpha.clamp(0.0, 1.0)
    } else {
        1.0
    }
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

#[derive(Clone, Copy)]
struct SvgRenderContext<'a> {
    opts: &'a SvgOptions,
    color_syntax: SvgColorSyntax,
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
        "Fraktur-Bold" => ("KaTeX_Fraktur", "bold", "normal"),
        "SansSerif-Regular" => ("KaTeX_SansSerif", "normal", "normal"),
        "SansSerif-Bold" => ("KaTeX_SansSerif", "bold", "normal"),
        "SansSerif-Italic" => ("KaTeX_SansSerif", "normal", "italic"),
        "Script-Regular" => ("KaTeX_Script", "normal", "normal"),
        "Typewriter-Regular" => ("KaTeX_Typewriter", "normal", "normal"),
        "Size1-Regular" => ("KaTeX_Size1", "normal", "normal"),
        "Size2-Regular" => ("KaTeX_Size2", "normal", "normal"),
        "Size3-Regular" => ("KaTeX_Size3", "normal", "normal"),
        "Size4-Regular" => ("KaTeX_Size4", "normal", "normal"),
        "CJK-Regular" => ("sans-serif", "normal", "normal"),
        "CJK-Fallback" => ("sans-serif", "normal", "normal"),
        // Stack so SVG `<text>` fallback works across macOS / Windows / Linux.
        "Emoji-Fallback" => (
            r#"Apple Color Emoji, "Segoe UI Emoji", "Noto Color Emoji", sans-serif"#,
            "normal",
            "normal",
        ),
        _ => ("KaTeX_Main", "normal", "normal"),
    }
}

#[cfg(feature = "standalone")]
fn emit_glyph_standalone(
    out: &mut String,
    g: GlyphEmit<'_>,
    context: SvgRenderContext<'_>,
    prerendered: Option<&standalone::StandaloneGlyph>,
) {
    let opts = context.opts;
    let color_syntax = context.color_syntax;
    if opts.embed_glyphs {
        if let Some(glyph) = prerendered {
            match glyph {
                standalone::StandaloneGlyph::Path(d) => {
                    let fill = color_to_svg(g.color, color_syntax);
                    let opacity = color_opacity_attr(g.color, "fill-opacity", color_syntax);
                    use std::fmt::Write;
                    let _ = write!(
                        out,
                        r#"<path d="{d}" fill="{fill}"{opacity} fill-rule="nonzero" stroke="none"/>"#
                    );
                    return;
                }
                standalone::StandaloneGlyph::Image { href, x, y, w, h } => {
                    use std::fmt::Write;
                    let x_s = fmt_num(*x as f64);
                    let y_s = fmt_num(*y as f64);
                    let w_s = fmt_num(*w as f64);
                    let h_s = fmt_num(*h as f64);
                    let opacity = normalized_alpha(g.color.a);
                    if opacity < 1.0 {
                        let opacity_s = fmt_num(opacity as f64);
                        let _ = write!(
                            out,
                            r#"<image href="{href}" x="{x_s}" y="{y_s}" width="{w_s}" height="{h_s}" opacity="{opacity_s}" preserveAspectRatio="none"/>"#
                        );
                    } else {
                        let _ = write!(
                            out,
                            r#"<image href="{href}" x="{x_s}" y="{y_s}" width="{w_s}" height="{h_s}" preserveAspectRatio="none"/>"#
                        );
                    }
                    return;
                }
            }
        }
    }
    emit_glyph_text(out, g, context);
}

fn emit_glyph_text(out: &mut String, g: GlyphEmit<'_>, context: SvgRenderContext<'_>) {
    let opts = context.opts;
    let color_syntax = context.color_syntax;
    let ch = char::from_u32(g.char_code).unwrap_or('\u{fffd}');
    let text = xml_escape_text(&ch.to_string());
    let (family, weight, style) = katex_face(g.font);
    let fs = g.scale * opts.em_px();
    let fill = color_to_svg(g.color, color_syntax);
    let opacity = color_opacity_attr(g.color, "fill-opacity", color_syntax);
    let x_s = fmt_num(tx(g.x, opts));
    let y_s = fmt_num(ty(g.y, opts));
    let fs_s = fmt_num(fs);
    use std::fmt::Write;
    let _ = write!(
        out,
        r#"<text x="{x_s}" y="{y_s}" font-family="{family}" font-size="{fs_s}" font-weight="{weight}" font-style="{style}" fill="{fill}"{opacity} dominant-baseline="alphabetic">{text}</text>"#
    );
}

#[allow(clippy::too_many_arguments)]
fn emit_line(
    out: &mut String,
    x: f64,
    y: f64,
    width: f64,
    thickness: f64,
    color: &Color,
    dashed: bool,
    context: SvgRenderContext<'_>,
) {
    let opts = context.opts;
    let color_syntax = context.color_syntax;
    let em = opts.em_px();
    let x0 = tx(x, opts);
    let yc = ty(y, opts);
    let t = (thickness * em).max(1e-6);
    let w = width * em;
    let stroke = color_to_svg(color, color_syntax);
    use std::fmt::Write;
    if dashed {
        let opacity = color_opacity_attr(color, "stroke-opacity", color_syntax);
        let x0s = fmt_num(x0);
        let ycs = fmt_num(yc);
        let x1s = fmt_num(x0 + w);
        let ts = fmt_num(t);
        let dash = fmt_num(t * 3.0);
        let _ = write!(
            out,
            r#"<line x1="{x0s}" y1="{ycs}" x2="{x1s}" y2="{ycs}" stroke="{stroke}"{opacity} stroke-width="{ts}" stroke-dasharray="{dash} {dash}"/>"#
        );
    } else {
        let opacity = color_opacity_attr(color, "fill-opacity", color_syntax);
        let y0 = yc - t / 2.0;
        let x0s = fmt_num(x0);
        let y0s = fmt_num(y0);
        let ws = fmt_num(w);
        let hs = fmt_num(t);
        let _ = write!(
            out,
            r#"<rect x="{x0s}" y="{y0s}" width="{ws}" height="{hs}" fill="{stroke}"{opacity}/>"#
        );
    }
}

fn emit_rect(
    out: &mut String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    color: &Color,
    context: SvgRenderContext<'_>,
) {
    let opts = context.opts;
    let color_syntax = context.color_syntax;
    let em = opts.em_px();
    let x0 = tx(x, opts);
    let y0 = ty(y, opts);
    let w = width * em;
    let h = height * em;
    let fill = color_to_svg(color, color_syntax);
    let opacity = color_opacity_attr(color, "fill-opacity", color_syntax);
    let x0s = fmt_num(x0);
    let y0s = fmt_num(y0);
    let ws = fmt_num(w);
    let hs = fmt_num(h);
    use std::fmt::Write;
    let _ = write!(
        out,
        r#"<rect x="{x0s}" y="{y0s}" width="{ws}" height="{hs}" fill="{fill}"{opacity}/>"#
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
    context: SvgRenderContext<'_>,
) {
    let opts = context.opts;
    let color_syntax = context.color_syntax;
    let em = opts.em_px();
    let ox = tx(x, opts);
    let oy = ty(y, opts);
    let paint = color_to_svg(color, color_syntax);

    if fill {
        let opacity = color_opacity_attr(color, "fill-opacity", color_syntax);
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
                    r#"<path d="{d}" fill="{paint}"{opacity} fill-rule="nonzero" stroke="none"/>"#
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
                    r#"<path d="{d}" fill="{paint}"{opacity} fill-rule="nonzero" stroke="none"/>"#
                );
            }
        }
    } else {
        let d = path_commands_to_d(ox, oy, em, commands);
        if d.is_empty() {
            return;
        }
        let sw = fmt_num(opts.stroke_width);
        let opacity = color_opacity_attr(color, "stroke-opacity", color_syntax);
        use std::fmt::Write;
        let _ = write!(
            out,
            r#"<path d="{d}" fill="none" stroke="{paint}"{opacity} stroke-width="{sw}" stroke-linecap="round" stroke-linejoin="round"/>"#
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

    #[test]
    fn rgb_color_syntax_uses_rgb_and_opacity_attrs() {
        let list = DisplayList {
            items: vec![
                DisplayItem::Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 0.5,
                    height: 0.2,
                    color: Color::new(1.0, 0.0, 0.0, 0.5),
                },
                DisplayItem::Path {
                    x: 0.0,
                    y: 0.0,
                    commands: vec![
                        PathCommand::MoveTo { x: 0.0, y: 0.0 },
                        PathCommand::LineTo { x: 1.0, y: 0.0 },
                    ],
                    fill: false,
                    color: Color::new(0.0, 0.5, 0.0, 0.25),
                },
                DisplayItem::GlyphPath {
                    x: 0.1,
                    y: 0.8,
                    scale: 1.0,
                    font: "Math-Italic".to_string(),
                    char_code: b'x' as u32,
                    color: Color::new(0.0, 0.0, 1.0, 0.75),
                },
            ],
            width: 2.0,
            height: 1.0,
            depth: 0.0,
        };
        let svg = render_to_svg_with_color_syntax(
            &list,
            &SvgOptions {
                font_size: 10.0,
                padding: 0.0,
                stroke_width: 1.0,
                embed_glyphs: false,
                font_dir: String::new(),
            },
            SvgColorSyntax::Rgb,
        );

        assert!(!svg.contains("rgba("), "{svg}");
        assert!(svg.contains(r#"fill="rgb(255,0,0)" fill-opacity="0.5""#));
        assert!(svg.contains(r#"stroke="rgb(0,128,0)" stroke-opacity="0.25""#));
        assert!(svg.contains(r#"fill="rgb(0,0,255)" fill-opacity="0.75""#));
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
        assert!(svg.contains("fill-rule=\"nonzero\""));
        assert!(!svg.contains("<text"));
    }

    #[cfg(feature = "standalone")]
    #[test]
    fn embedded_emoji_image_uses_color_alpha_as_opacity() {
        use std::path::PathBuf;

        let ch = '😀';
        if ratex_unicode_font::emoji_png_raster_for_char(ch, 10.0).is_none() {
            return;
        }

        let font_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tools/lexer_compare/node_modules/katex/dist/fonts");
        if !font_dir.join("KaTeX_Main-Regular.ttf").exists() {
            return;
        }

        let list = DisplayList {
            items: vec![DisplayItem::GlyphPath {
                x: 0.0,
                y: 1.0,
                scale: 1.0,
                font: "Emoji-Fallback".to_string(),
                char_code: ch as u32,
                color: Color::new(1.0, 0.0, 0.0, 0.5),
            }],
            width: 1.2,
            height: 2.0,
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

        assert!(svg.contains("<image"));
        assert!(svg.contains("opacity=\"0.5\""), "{svg}");
    }
}
