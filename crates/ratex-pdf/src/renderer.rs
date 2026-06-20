//! Core rendering: convert a [`DisplayList`] into PDF bytes via pdf-writer.
//!
//! Two-pass architecture:
//! 1. Collect all glyphs used across the display list.
//! 2. Subset & embed fonts, then write the content stream.

use std::collections::{BTreeMap, HashMap};

use pdf_writer::{types::ProcSet, Content, Filter, Finish, Name, Pdf, Rect, Ref, Str};
use ratex_font::FontId;
use ratex_types::color::Color;
use ratex_types::display_item::{DisplayItem, DisplayList};
use ratex_types::path_command::PathCommand;

use crate::fonts::{self, EmbeddedFont};

// Keep a tiny vertical guard band for rasterizers/viewers that antialias
// glyphs just outside the TeX layout box when callers render with little/no padding.
const MIN_VERTICAL_ANTIALIAS_GUARD: f64 = 1.0;

/// Options controlling PDF output.
#[derive(Debug, Clone)]
pub struct PdfOptions {
    /// User units per em. Default: 40.
    pub font_size: f64,
    /// Padding on all sides, in user units. Default: 10.
    pub padding: f64,
    /// Stroke width for unfilled paths, in user units. Default: 1.5.
    pub stroke_width: f64,
    /// Directory containing KaTeX `.ttf` files. Used only when `embed-fonts` is disabled.
    pub font_dir: String,
}

impl Default for PdfOptions {
    fn default() -> Self {
        Self {
            font_size: 40.0,
            padding: 10.0,
            stroke_width: 1.5,
            font_dir: String::new(),
        }
    }
}

/// Errors that can occur during PDF rendering.
#[derive(Debug)]
pub enum PdfError {
    Font(String),
    Render(String),
}

impl std::fmt::Display for PdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PdfError::Font(s) => write!(f, "Font error: {s}"),
            PdfError::Render(s) => write!(f, "Render error: {s}"),
        }
    }
}

impl std::error::Error for PdfError {}

/// Render a [`DisplayList`] to a PDF byte buffer.
pub fn render_to_pdf(
    display_list: &DisplayList,
    options: &PdfOptions,
) -> Result<Vec<u8>, PdfError> {
    let em = options.font_size;
    let pad = options.padding;
    let sw = options.stroke_width;

    let total_h = display_list.height + display_list.depth;
    let vertical_guard = if display_list.items.is_empty() {
        0.0
    } else {
        (MIN_VERTICAL_ANTIALIAS_GUARD - pad).max(0.0)
    };
    let y_origin = pad + vertical_guard;
    let page_w = display_list.width * em + 2.0 * pad;
    let page_h = total_h * em + 2.0 * pad + 2.0 * vertical_guard;

    // Load raw font data (lazy: only fonts referenced by this display list).
    let font_data = ratex_font_loader::load_fonts_for_items(&options.font_dir, &display_list.items)
        .map_err(PdfError::Font)?;

    // Pass 1: collect glyph usage (emoji → raster XObjects; other faces → subset fonts).
    let collected = fonts::collect_glyph_usage(&display_list.items, &font_data, em);

    // Build the PDF.
    let mut pdf = Pdf::new();
    let mut alloc = Ref::new(1);

    let catalog_ref = alloc.bump();
    let pages_ref = alloc.bump();
    let page_ref = alloc.bump();
    let content_ref = alloc.bump();

    // Pass 2: embed fonts (no Type0 for color emoji — those use images below).
    let embedded = fonts::embed_fonts(&mut pdf, &mut alloc, &collected.font_usages, &font_data)
        .map_err(PdfError::Font)?;

    let emoji_embedded = fonts::embed_emoji_rasters(&mut pdf, &mut alloc, &collected.emoji_rasters)
        .map_err(PdfError::Font)?;

    // Build lookup: FontId → EmbeddedFont index.
    let font_index: HashMap<FontId, usize> = embedded
        .iter()
        .enumerate()
        .map(|(i, ef)| (ef.font_id, i))
        .collect();

    let emoji_ix: HashMap<u32, usize> = emoji_embedded
        .iter()
        .enumerate()
        .map(|(i, e)| (e.char_code, i))
        .collect();

    let alpha_states = collect_alpha_graphics_states(&display_list.items, &mut alloc);

    // Generate content stream.
    let content_bytes = build_content_stream(
        &display_list.items,
        &embedded,
        &font_index,
        &font_data,
        &emoji_embedded,
        &emoji_ix,
        &alpha_states,
        em,
        pad,
        y_origin,
        page_h,
        sw,
    );

    // Compress content stream.
    let compressed = miniz_oxide::deflate::compress_to_vec_zlib(&content_bytes, 6);

    // Write content stream object.
    let mut stream = pdf.stream(content_ref, &compressed);
    stream.filter(Filter::FlateDecode);
    stream.pair(Name(b"Length1"), content_bytes.len() as i32);
    stream.finish();

    write_alpha_graphics_states(&mut pdf, &alpha_states);

    // Page object.
    let mut page = pdf.page(page_ref);
    page.parent(pages_ref);
    page.media_box(Rect::new(0.0, 0.0, page_w as f32, page_h as f32));
    page.contents(content_ref);

    // Page Resources: font dictionary.
    let mut resources = page.resources();
    if !emoji_embedded.is_empty() {
        // Color images via `Do` — include ImageC for older print/PDF pipelines that omit it.
        resources.proc_sets([
            ProcSet::Pdf,
            ProcSet::Text,
            ProcSet::ImageGrayscale,
            ProcSet::ImageColor,
            ProcSet::ImageIndexed,
        ]);
    }
    let mut font_dict = resources.fonts();
    for ef in &embedded {
        font_dict.pair(Name(ef.res_name.as_bytes()), ef.type0_ref);
    }
    font_dict.finish();
    if !emoji_embedded.is_empty() {
        let mut xobjects = resources.x_objects();
        for e in &emoji_embedded {
            xobjects.pair(Name(e.res_name.as_bytes()), e.image_ref);
        }
        xobjects.finish();
    }
    if !alpha_states.is_empty() {
        let mut ext_g_states = resources.ext_g_states();
        for state in alpha_states.values() {
            ext_g_states.pair(Name(state.res_name.as_bytes()), state.state_ref);
        }
        ext_g_states.finish();
    }
    resources.finish();
    page.finish();

    // Pages node.
    let mut pages = pdf.pages(pages_ref);
    pages.count(1);
    pages.kids([page_ref]);
    pages.finish();

    // Catalog.
    pdf.catalog(catalog_ref).pages(pages_ref);

    Ok(pdf.finish())
}

// ---------------------------------------------------------------------------
// Content stream generation
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn build_content_stream(
    items: &[DisplayItem],
    embedded: &[EmbeddedFont],
    font_index: &HashMap<FontId, usize>,
    font_data: &fonts::RawFontData,
    emoji_assets: &[fonts::EmbeddedEmojiImage],
    emoji_ix: &HashMap<u32, usize>,
    alpha_states: &AlphaGraphicsStates,
    em: f64,
    x_origin: f64,
    y_origin: f64,
    page_h: f64,
    stroke_width: f64,
) -> Vec<u8> {
    let mut content = Content::new();

    for item in items {
        match item {
            DisplayItem::GlyphPath {
                x,
                y,
                scale,
                font,
                char_code,
                color,
                ..
            } => {
                emit_glyph(
                    &mut content,
                    *x * em + x_origin,
                    *y * em + y_origin,
                    font,
                    *char_code,
                    *scale,
                    color,
                    em,
                    page_h,
                    embedded,
                    font_index,
                    font_data,
                    emoji_assets,
                    emoji_ix,
                    alpha_states,
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
                emit_line(
                    &mut content,
                    &LineParams {
                        x: *x * em + x_origin,
                        y: *y * em + y_origin,
                        width: *width * em,
                        thickness: *thickness * em,
                        color: *color,
                        dashed: *dashed,
                        page_h,
                    },
                    alpha_states,
                );
            }
            DisplayItem::Rect {
                x,
                y,
                width,
                height,
                color,
            } => {
                emit_rect(
                    &mut content,
                    *x * em + x_origin,
                    *y * em + y_origin,
                    *width * em,
                    *height * em,
                    color,
                    page_h,
                    alpha_states,
                );
            }
            DisplayItem::Path {
                x,
                y,
                commands,
                fill,
                color,
            } => {
                emit_path(
                    &mut content,
                    *x * em + x_origin,
                    *y * em + y_origin,
                    commands,
                    *fill,
                    color,
                    em,
                    stroke_width,
                    page_h,
                    alpha_states,
                );
            }
        }
    }

    content.finish().into_vec()
}

/// Flip Y: PDF origin is bottom-left, DisplayList origin is top-left.
#[inline]
fn flip_y(y: f64, page_h: f64) -> f32 {
    (page_h - y) as f32
}

// ---------------------------------------------------------------------------
// Glyph
// ---------------------------------------------------------------------------

/// Color emoji via sbix PNG and image XObject (placement matches `ratex-render::try_blit_raster_glyph`).
fn emit_emoji_raster(
    content: &mut Content,
    px: f64,
    py: f64,
    glyph_em: f64,
    page_h: f64,
    asset: &fonts::EmbeddedEmojiImage,
) {
    let ppm = f64::from(asset.pixels_per_em.max(1));
    let mut s = glyph_em / ppm;

    // Scale emoji to fit 1.0em layout width if it's wider (prevents overflow).
    let actual_width_em = f64::from(asset.width_px) / ppm;
    let assumed_width = 1.0;
    if actual_width_em > 0.01 && actual_width_em > assumed_width * 1.01 {
        s *= assumed_width / actual_width_em;
    }

    let disp_w = f64::from(asset.width_px) * s;
    let disp_h = f64::from(asset.height_px) * s;
    let top_x = px + f64::from(asset.strike_x) * s;
    let mut top_y = py - (f64::from(asset.strike_y) + f64::from(asset.height_px)) * s;
    let center_strike = (f64::from(asset.strike_y) + f64::from(asset.height_px) / 2.0) / ppm;
    let axis = ratex_font::get_global_metrics(0).axis_height;
    top_y += (center_strike - axis) * glyph_em;
    let mut pdf_y_bl = page_h - top_y - disp_h;
    let pdf_y_top = pdf_y_bl + disp_h;
    // Many viewers clip XObjects strictly to MediaBox. If sbix placement + axis nudge pushes the
    // bitmap fully above y=page_h or fully below y=0, nothing paints ("invisible" emoji).
    if pdf_y_top > page_h {
        pdf_y_bl = page_h - disp_h;
    }
    if pdf_y_bl < 0.0 {
        pdf_y_bl = 0.0;
    }
    content.save_state();
    content.transform([
        disp_w as f32,
        0.0,
        0.0,
        disp_h as f32,
        top_x as f32,
        pdf_y_bl as f32,
    ]);
    content.x_object(Name(asset.res_name.as_bytes()));
    content.restore_state();
}

#[allow(clippy::too_many_arguments)]
fn emit_glyph(
    content: &mut Content,
    px: f64,
    py: f64,
    font_name: &str,
    char_code: u32,
    scale: f64,
    color: &Color,
    em: f64,
    page_h: f64,
    embedded: &[EmbeddedFont],
    font_index: &HashMap<FontId, usize>,
    font_data: &fonts::RawFontData,
    emoji_assets: &[fonts::EmbeddedEmojiImage],
    emoji_ix: &HashMap<u32, usize>,
    alpha_states: &AlphaGraphicsStates,
) {
    // Color emoji: collect/embed keyed only by char_code; draw whenever we embedded an XObject,
    // without re-resolving (must match [`fonts::collect_glyph_usage`] prefer-color path).
    if let Some(&ix) = emoji_ix.get(&char_code) {
        let asset = &emoji_assets[ix];
        let alpha_applied = apply_non_stroking_alpha(content, color, alpha_states);
        emit_emoji_raster(content, px, py, scale * em, page_h, asset);
        restore_alpha_if_needed(content, alpha_applied);
        return;
    }

    let (actual_fid, gid) = match fonts::resolve_pdf_glyph(font_data, font_name, char_code) {
        Some(p) => p,
        None => return,
    };

    let ef_idx = match font_index.get(&actual_fid) {
        Some(&i) => i,
        None => return,
    };
    let ef = &embedded[ef_idx];

    let new_cid = match ef.remapper.get(gid) {
        Some(c) => c,
        None => return,
    };

    let glyph_em = (scale * em) as f32;
    let pdf_x = px as f32;
    let pdf_y = flip_y(py, page_h);

    // Emoji outline fallback has no KaTeX metrics; scale it to the 1.0em width that layout
    // allocates for missing emoji so Windows vector fallback does not overflow.
    let mut text_matrix_scale = 1.0;
    if actual_fid == FontId::EmojiFallback {
        if let Some(font_bytes) = font_data.get(&FontId::EmojiFallback) {
            use ab_glyph::Font;
            let idx = ratex_unicode_font::emoji_font_face_index().unwrap_or(0);
            if let Ok(font) = ab_glyph::FontRef::try_from_slice_and_index(font_bytes, idx) {
                let ch = char::from_u32(char_code).unwrap_or('\u{FFFD}');
                let glyph_id = font.glyph_id(ch);
                if glyph_id.0 != 0 {
                    let actual_advance = font.h_advance_unscaled(glyph_id);
                    let units_per_em = font.units_per_em().unwrap_or(1000.0);
                    let actual_advance_em = actual_advance / units_per_em;
                    let assumed_width = 1.0;
                    if actual_advance_em > 0.01 && actual_advance_em > assumed_width * 1.01 {
                        text_matrix_scale = assumed_width / actual_advance_em;
                    }
                }
            }
        }
    }

    // CID as 2-byte big-endian.
    let cid_bytes = [(new_cid >> 8) as u8, (new_cid & 0xFF) as u8];

    let alpha_applied = apply_non_stroking_alpha(content, color, alpha_states);
    set_fill_rgb(content, color);
    content.begin_text();
    content.set_font(Name(ef.res_name.as_bytes()), glyph_em);
    content.set_text_matrix([text_matrix_scale, 0.0, 0.0, text_matrix_scale, pdf_x, pdf_y]);
    content.show(Str(&cid_bytes));
    content.end_text();
    restore_alpha_if_needed(content, alpha_applied);
}

// ---------------------------------------------------------------------------
// Line
// ---------------------------------------------------------------------------

struct LineParams {
    x: f64,
    y: f64,
    width: f64,
    thickness: f64,
    color: Color,
    dashed: bool,
    page_h: f64,
}

fn emit_line(content: &mut Content, line: &LineParams, alpha_states: &AlphaGraphicsStates) {
    let t = line.thickness.max(0.5);

    let alpha_applied = apply_non_stroking_alpha(content, &line.color, alpha_states);
    set_fill_rgb(content, &line.color);

    if line.dashed {
        let dash_len = (4.0 * t).max(1.0);
        let gap_len = (4.0 * t).max(1.0);
        let period = dash_len + gap_len;
        let top = line.y - t / 2.0;
        let mut cur_x = line.x;
        while cur_x < line.x + line.width {
            let seg_w = dash_len.min(line.x + line.width - cur_x).max(0.5);
            let pdf_x = cur_x as f32;
            let pdf_y = flip_y(top + t, line.page_h); // bottom edge in PDF coords
            content.rect(pdf_x, pdf_y, seg_w as f32, t as f32);
            cur_x += period;
        }
        content.fill_nonzero();
    } else {
        let top = line.y - t / 2.0;
        let pdf_x = line.x as f32;
        let pdf_y = flip_y(top + t, line.page_h);
        content.rect(pdf_x, pdf_y, line.width as f32, t as f32);
        content.fill_nonzero();
    }
    restore_alpha_if_needed(content, alpha_applied);
}

// ---------------------------------------------------------------------------
// Rect
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn emit_rect(
    content: &mut Content,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    color: &Color,
    page_h: f64,
    alpha_states: &AlphaGraphicsStates,
) {
    let w = width.max(0.5);
    let h = height.max(0.5);

    let alpha_applied = apply_non_stroking_alpha(content, color, alpha_states);
    set_fill_rgb(content, color);
    let pdf_x = x as f32;
    let pdf_y = flip_y(y + h, page_h); // bottom-left corner in PDF coords
    content.rect(pdf_x, pdf_y, w as f32, h as f32);
    content.fill_nonzero();
    restore_alpha_if_needed(content, alpha_applied);
}

// ---------------------------------------------------------------------------
// Path
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn emit_path(
    content: &mut Content,
    ox: f64,
    oy: f64,
    commands: &[PathCommand],
    fill: bool,
    color: &Color,
    em: f64,
    stroke_width: f64,
    page_h: f64,
    alpha_states: &AlphaGraphicsStates,
) {
    if fill {
        // Split by MoveTo to avoid cross-contour winding issues (same as ratex-render).
        let mut start = 0;
        for i in 1..commands.len() {
            if matches!(commands[i], PathCommand::MoveTo { .. }) {
                emit_path_segment(
                    content,
                    ox,
                    oy,
                    &commands[start..i],
                    true,
                    color,
                    em,
                    stroke_width,
                    page_h,
                    alpha_states,
                );
                start = i;
            }
        }
        emit_path_segment(
            content,
            ox,
            oy,
            &commands[start..],
            true,
            color,
            em,
            stroke_width,
            page_h,
            alpha_states,
        );
    } else {
        emit_path_segment(
            content,
            ox,
            oy,
            commands,
            false,
            color,
            em,
            stroke_width,
            page_h,
            alpha_states,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_path_segment(
    content: &mut Content,
    ox: f64,
    oy: f64,
    commands: &[PathCommand],
    fill: bool,
    color: &Color,
    em: f64,
    stroke_width: f64,
    page_h: f64,
    alpha_states: &AlphaGraphicsStates,
) {
    if commands.is_empty() {
        return;
    }

    let alpha_applied = if fill {
        apply_non_stroking_alpha(content, color, alpha_states)
    } else {
        apply_stroking_alpha(content, color, alpha_states)
    };
    if fill {
        set_fill_rgb(content, color);
    } else {
        set_stroke_rgb(content, color);
        content.set_line_width(stroke_width as f32);
    }

    // Track current point for quad-to-cubic promotion.
    let mut cur = (0.0f32, 0.0f32);

    for cmd in commands {
        match cmd {
            PathCommand::MoveTo { x, y } => {
                let px = (ox + x * em) as f32;
                let py = flip_y(oy + y * em, page_h);
                content.move_to(px, py);
                cur = (px, py);
            }
            PathCommand::LineTo { x, y } => {
                let px = (ox + x * em) as f32;
                let py = flip_y(oy + y * em, page_h);
                content.line_to(px, py);
                cur = (px, py);
            }
            PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let end_x = (ox + x * em) as f32;
                let end_y = flip_y(oy + y * em, page_h);
                content.cubic_to(
                    (ox + x1 * em) as f32,
                    flip_y(oy + y1 * em, page_h),
                    (ox + x2 * em) as f32,
                    flip_y(oy + y2 * em, page_h),
                    end_x,
                    end_y,
                );
                cur = (end_x, end_y);
            }
            PathCommand::QuadTo { x1, y1, x, y } => {
                // PDF has no native quadratic Bezier; promote to cubic.
                // Q(P0,P1,P2) → C(P0, P0+2/3*(P1-P0), P2+2/3*(P1-P2), P2)
                let qx = (ox + x1 * em) as f32;
                let qy = flip_y(oy + y1 * em, page_h);
                let end_x = (ox + x * em) as f32;
                let end_y = flip_y(oy + y * em, page_h);
                let cp1_x = cur.0 + 2.0 / 3.0 * (qx - cur.0);
                let cp1_y = cur.1 + 2.0 / 3.0 * (qy - cur.1);
                let cp2_x = end_x + 2.0 / 3.0 * (qx - end_x);
                let cp2_y = end_y + 2.0 / 3.0 * (qy - end_y);
                content.cubic_to(cp1_x, cp1_y, cp2_x, cp2_y, end_x, end_y);
                cur = (end_x, end_y);
            }
            PathCommand::Close => {
                content.close_path();
            }
        }
    }

    if fill {
        content.fill_even_odd();
    } else {
        content.stroke();
    }
    restore_alpha_if_needed(content, alpha_applied);
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn set_fill_rgb(content: &mut Content, color: &Color) {
    content.set_fill_rgb(color.r, color.g, color.b);
}

fn set_stroke_rgb(content: &mut Content, color: &Color) {
    content.set_stroke_rgb(color.r, color.g, color.b);
}

#[derive(Debug)]
struct AlphaGraphicsState {
    alpha: f32,
    res_name: String,
    state_ref: Ref,
}

type AlphaGraphicsStates = BTreeMap<u32, AlphaGraphicsState>;

fn collect_alpha_graphics_states(items: &[DisplayItem], alloc: &mut Ref) -> AlphaGraphicsStates {
    let mut states = AlphaGraphicsStates::new();
    for item in items {
        let color = match item {
            DisplayItem::GlyphPath { color, .. }
            | DisplayItem::Line { color, .. }
            | DisplayItem::Rect { color, .. }
            | DisplayItem::Path { color, .. } => color,
        };

        if let Some(key) = alpha_key(color.a) {
            states.entry(key).or_insert_with(|| AlphaGraphicsState {
                alpha: normalized_alpha(color.a),
                res_name: format!("GS{key}"),
                state_ref: alloc.bump(),
            });
        }
    }
    states
}

fn write_alpha_graphics_states(pdf: &mut Pdf, states: &AlphaGraphicsStates) {
    for state in states.values() {
        let mut ext_g_state = pdf.ext_graphics(state.state_ref);
        ext_g_state
            .stroking_alpha(state.alpha)
            .non_stroking_alpha(state.alpha);
        ext_g_state.finish();
    }
}

fn apply_non_stroking_alpha(
    content: &mut Content,
    color: &Color,
    states: &AlphaGraphicsStates,
) -> bool {
    apply_alpha(content, color, states)
}

fn apply_stroking_alpha(
    content: &mut Content,
    color: &Color,
    states: &AlphaGraphicsStates,
) -> bool {
    apply_alpha(content, color, states)
}

fn apply_alpha(content: &mut Content, color: &Color, states: &AlphaGraphicsStates) -> bool {
    let Some(key) = alpha_key(color.a) else {
        return false;
    };
    let Some(state) = states.get(&key) else {
        return false;
    };

    content.save_state();
    content.set_parameters(Name(state.res_name.as_bytes()));
    true
}

fn restore_alpha_if_needed(content: &mut Content, applied: bool) {
    if applied {
        content.restore_state();
    }
}

fn alpha_key(alpha: f32) -> Option<u32> {
    let alpha = normalized_alpha(alpha);
    if alpha >= 1.0 {
        return None;
    }
    Some((alpha * 1_000_000.0).round() as u32)
}

fn normalized_alpha(alpha: f32) -> f32 {
    if alpha.is_finite() {
        alpha.clamp(0.0, 1.0)
    } else {
        1.0
    }
}
