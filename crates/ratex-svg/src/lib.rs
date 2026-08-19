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

    // Pre-render standalone glyphs while the `ParsedFontSet` and its borrowed
    // parsed/raw font references are alive, then drop them. The emitted paths/images
    // are self-contained, so the body loop below does not need the font cache.
    #[cfg(feature = "standalone")]
    let prerendered_glyphs: Option<Vec<Option<standalone::StandaloneGlyph>>> = {
        if load_fonts {
            if let Ok(fonts) =
                ratex_font_loader::load_fonts_for_items_parsed(&opts.font_dir, &list.items)
            {
                let font_refs = standalone::build_font_refs(&fonts);
                let system_fonts = ratex_font_loader::SystemFontResolver::new();
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
                            px,
                            py,
                            glyph_em,
                            font,
                            *char_code,
                            &font_refs,
                            &system_fonts,
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
    let mut out = String::with_capacity(body.len() + 96);
    out.push_str(r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 "#);
    fmt_num_to(&mut out, vb_w);
    out.push(' ');
    fmt_num_to(&mut out, vb_h);
    out.push_str(r#"" width=""#);
    fmt_num_to(&mut out, vb_w);
    out.push_str(r#"pt" height=""#);
    fmt_num_to(&mut out, vb_h);
    out.push_str("pt\">");
    out.push_str(body);
    out.push_str("</svg>");
    out
}

fn tx(x_em: f64, opts: &SvgOptions) -> f64 {
    x_em * opts.em_px() + opts.padding
}

fn ty(y_em: f64, opts: &SvgOptions) -> f64 {
    y_em * opts.em_px() + opts.padding
}

/// Append `n` formatted with the legacy `fmt_num` semantics directly to `out`
/// (6 fractional digits, trailing zeros and dot trimmed; `-0` is preserved),
/// without any intermediate allocation.
fn fmt_num_to(out: &mut String, n: f64) {
    use std::fmt::Write;
    let start = out.len();
    let _ = write!(out, "{n:.6}");
    // Reproduce `s.trim_end_matches('0').trim_end_matches('.')` on the tail.
    let bytes = out.as_bytes();
    let mut end = out.len();
    while end > start + 1 && bytes[end - 1] == b'0' {
        end -= 1;
    }
    if end > start && bytes[end - 1] == b'.' {
        end -= 1;
    }
    out.truncate(end);
    // Legacy safety net: empty or bare `-` becomes `0`.
    if end == start || &out[start..end] == "-" {
        out.truncate(start);
        out.push('0');
    }
}

fn color_to_svg_into(out: &mut String, c: &Color, syntax: SvgColorSyntax) {
    use std::fmt::Write;
    let r = (c.r.clamp(0.0, 1.0) * 255.0).round() as u8;
    let g = (c.g.clamp(0.0, 1.0) * 255.0).round() as u8;
    let b = (c.b.clamp(0.0, 1.0) * 255.0).round() as u8;
    match syntax {
        SvgColorSyntax::Rgba => {
            let a = normalized_alpha(c.a);
            let _ = write!(out, "rgba({r},{g},{b},{a})");
        }
        SvgColorSyntax::Rgb => {
            let _ = write!(out, "rgb({r},{g},{b})");
        }
    }
}

/// Append ` attr="opacity"` (Rgb syntax, alpha < 1) or nothing.
fn color_opacity_attr_into(out: &mut String, c: &Color, attr: &str, syntax: SvgColorSyntax) {
    if syntax != SvgColorSyntax::Rgb {
        return;
    }

    let alpha = normalized_alpha(c.a);
    if alpha < 1.0 {
        out.push(' ');
        out.push_str(attr);
        out.push_str("=\"");
        fmt_num_to(out, alpha as f64);
        out.push('"');
    }
}

fn normalized_alpha(alpha: f32) -> f32 {
    if alpha.is_finite() {
        alpha.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

/// Append `ch` with XML text escaping directly to `out`.
fn push_escaped_char(out: &mut String, ch: char) {
    match ch {
        '&' => out.push_str("&amp;"),
        '<' => out.push_str("&lt;"),
        '>' => out.push_str("&gt;"),
        '"' => out.push_str("&quot;"),
        _ => out.push(ch),
    }
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
                    use std::fmt::Write;
                    let _ = write!(out, r#"<path d="{d}" fill=""#);
                    color_to_svg_into(out, g.color, color_syntax);
                    out.push('"');
                    color_opacity_attr_into(out, g.color, "fill-opacity", color_syntax);
                    let _ = write!(out, r#" fill-rule="nonzero" stroke="none"/>"#);
                    return;
                }
                standalone::StandaloneGlyph::Image { href, x, y, w, h } => {
                    use std::fmt::Write;
                    out.push_str(r#"<image href=""#);
                    out.push_str(href);
                    out.push_str(r#"" x=""#);
                    fmt_num_to(out, *x as f64);
                    out.push_str(r#"" y=""#);
                    fmt_num_to(out, *y as f64);
                    out.push_str(r#"" width=""#);
                    fmt_num_to(out, *w as f64);
                    out.push_str(r#"" height=""#);
                    fmt_num_to(out, *h as f64);
                    let opacity = normalized_alpha(g.color.a);
                    if opacity < 1.0 {
                        out.push_str(r#"" opacity=""#);
                        fmt_num_to(out, opacity as f64);
                    }
                    let _ = write!(out, r#"" preserveAspectRatio="none"/>"#);
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
    let (family, weight, style) = katex_face(g.font);
    let fs = g.scale * opts.em_px();
    use std::fmt::Write;
    let _ = write!(out, r#"<text x=""#);
    fmt_num_to(out, tx(g.x, opts));
    let _ = write!(out, r#"" y=""#);
    fmt_num_to(out, ty(g.y, opts));
    let _ = write!(out, r#"" font-family="{family}" font-size=""#);
    fmt_num_to(out, fs);
    let _ = write!(
        out,
        r#"" font-weight="{weight}" font-style="{style}" fill=""#
    );
    color_to_svg_into(out, g.color, color_syntax);
    out.push('"');
    color_opacity_attr_into(out, g.color, "fill-opacity", color_syntax);
    let _ = write!(out, r#" dominant-baseline="alphabetic">"#);
    push_escaped_char(out, ch);
    let _ = write!(out, "</text>");
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
    use std::fmt::Write;
    if dashed {
        let _ = write!(out, r#"<line x1=""#);
        fmt_num_to(out, x0);
        let _ = write!(out, r#"" y1=""#);
        fmt_num_to(out, yc);
        let _ = write!(out, r#"" x2=""#);
        fmt_num_to(out, x0 + w);
        let _ = write!(out, r#"" y2=""#);
        fmt_num_to(out, yc);
        let _ = write!(out, r#"" stroke=""#);
        color_to_svg_into(out, color, color_syntax);
        out.push('"');
        color_opacity_attr_into(out, color, "stroke-opacity", color_syntax);
        let _ = write!(out, r#" stroke-width=""#);
        fmt_num_to(out, t);
        let _ = write!(out, r#"" stroke-dasharray=""#);
        fmt_num_to(out, t * 3.0);
        let _ = write!(out, r#" "#);
        fmt_num_to(out, t * 3.0);
        let _ = write!(out, r#""/>"#);
    } else {
        let y0 = yc - t / 2.0;
        let _ = write!(out, r#"<rect x=""#);
        fmt_num_to(out, x0);
        let _ = write!(out, r#"" y=""#);
        fmt_num_to(out, y0);
        let _ = write!(out, r#"" width=""#);
        fmt_num_to(out, w);
        let _ = write!(out, r#"" height=""#);
        fmt_num_to(out, t);
        let _ = write!(out, r#"" fill=""#);
        color_to_svg_into(out, color, color_syntax);
        out.push('"');
        color_opacity_attr_into(out, color, "fill-opacity", color_syntax);
        let _ = write!(out, "/>");
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
    use std::fmt::Write;
    let _ = write!(out, r#"<rect x=""#);
    fmt_num_to(out, x0);
    let _ = write!(out, r#"" y=""#);
    fmt_num_to(out, y0);
    let _ = write!(out, r#"" width=""#);
    fmt_num_to(out, w);
    let _ = write!(out, r#"" height=""#);
    fmt_num_to(out, h);
    let _ = write!(out, r#"" fill=""#);
    color_to_svg_into(out, color, color_syntax);
    out.push('"');
    color_opacity_attr_into(out, color, "fill-opacity", color_syntax);
    let _ = write!(out, "/>");
}

/// Append the SVG path data for `commands` to `out`, trimming the trailing
/// separator exactly like the legacy `path_commands_to_d` did.
fn path_commands_to_d_into(
    out: &mut String,
    origin_x: f64,
    origin_y: f64,
    em: f64,
    commands: &[PathCommand],
) {
    let start_len = out.len();
    for cmd in commands {
        match cmd {
            PathCommand::MoveTo { x, y } => {
                out.push('M');
                fmt_num_to(out, origin_x + x * em);
                out.push(' ');
                fmt_num_to(out, origin_y + y * em);
            }
            PathCommand::LineTo { x, y } => {
                out.push('L');
                fmt_num_to(out, origin_x + x * em);
                out.push(' ');
                fmt_num_to(out, origin_y + y * em);
            }
            PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                out.push('C');
                fmt_num_to(out, origin_x + x1 * em);
                out.push(' ');
                fmt_num_to(out, origin_y + y1 * em);
                out.push(' ');
                fmt_num_to(out, origin_x + x2 * em);
                out.push(' ');
                fmt_num_to(out, origin_y + y2 * em);
                out.push(' ');
                fmt_num_to(out, origin_x + x * em);
                out.push(' ');
                fmt_num_to(out, origin_y + y * em);
            }
            PathCommand::QuadTo { x1, y1, x, y } => {
                out.push('Q');
                fmt_num_to(out, origin_x + x1 * em);
                out.push(' ');
                fmt_num_to(out, origin_y + y1 * em);
                out.push(' ');
                fmt_num_to(out, origin_x + x * em);
                out.push(' ');
                fmt_num_to(out, origin_y + y * em);
            }
            PathCommand::Close => out.push('Z'),
        }
        out.push(' ');
    }
    // `d.trim_end()` on the segment just written; never touch content that
    // was already in `out` before this call.
    let bytes = out.as_bytes();
    let mut end = out.len();
    while end > start_len && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    out.truncate(end);
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

    if fill {
        let mut start = 0usize;
        for i in 1..commands.len() {
            if matches!(commands[i], PathCommand::MoveTo { .. }) {
                let seg = &commands[start..i];
                start = i;
                if seg.is_empty() {
                    continue;
                }
                use std::fmt::Write;
                let _ = write!(out, r#"<path d=""#);
                path_commands_to_d_into(out, ox, oy, em, seg);
                let _ = write!(out, r#"" fill=""#);
                color_to_svg_into(out, color, color_syntax);
                out.push('"');
                color_opacity_attr_into(out, color, "fill-opacity", color_syntax);
                let _ = write!(out, r#" fill-rule="nonzero" stroke="none"/>"#);
            }
        }
        let seg = &commands[start..];
        if !seg.is_empty() {
            use std::fmt::Write;
            let _ = write!(out, r#"<path d=""#);
            path_commands_to_d_into(out, ox, oy, em, seg);
            let _ = write!(out, r#"" fill=""#);
            color_to_svg_into(out, color, color_syntax);
            out.push('"');
            color_opacity_attr_into(out, color, "fill-opacity", color_syntax);
            let _ = write!(out, r#" fill-rule="nonzero" stroke="none"/>"#);
        }
    } else {
        if commands.is_empty() {
            return;
        }
        use std::fmt::Write;
        let _ = write!(out, r#"<path d=""#);
        path_commands_to_d_into(out, ox, oy, em, commands);
        let _ = write!(out, r#"" fill="none" stroke=""#);
        color_to_svg_into(out, color, color_syntax);
        out.push('"');
        color_opacity_attr_into(out, color, "stroke-opacity", color_syntax);
        let _ = write!(out, r#" stroke-width=""#);
        fmt_num_to(out, opts.stroke_width);
        let _ = write!(out, r#"" stroke-linecap="round" stroke-linejoin="round"/>"#);
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
