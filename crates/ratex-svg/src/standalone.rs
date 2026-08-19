//! Glyph outlines as SVG `<path>` via `ab_glyph` (feature `standalone`).

use std::cell::OnceCell;
use std::collections::HashMap;

use ab_glyph::{Font, FontRef, FontVec, OutlineCurve};
use ratex_font::FontId;
use ratex_font_loader::{OutlineSourceId, ParsedFontSet, SystemFontResolver};

pub(crate) enum SvgFont<'a> {
    Parsed(&'a FontVec),
    Raw(Box<LazyRawFont<'a>>),
    System(&'a FontRef<'static>),
}

pub(crate) struct LazyRawFont<'a> {
    bytes: &'a [u8],
    face_index: u32,
    parsed: OnceCell<Result<FontRef<'a>, ab_glyph::InvalidFont>>,
}

impl<'a> LazyRawFont<'a> {
    fn get(&self) -> Option<&FontRef<'a>> {
        self.parsed
            .get_or_init(|| FontRef::try_from_slice_and_index(self.bytes, self.face_index))
            .as_ref()
            .ok()
    }
}

impl SvgFont<'_> {
    fn glyph_id(&self, ch: char) -> ab_glyph::GlyphId {
        match self {
            Self::Parsed(font) => font.glyph_id(ch),
            Self::Raw(raw) => raw
                .get()
                .map_or(ab_glyph::GlyphId(0), |font| font.glyph_id(ch)),
            Self::System(font) => font.glyph_id(ch),
        }
    }

    fn units_per_em(&self) -> Option<f32> {
        match self {
            Self::Parsed(font) => font.units_per_em(),
            Self::Raw(raw) => raw.get().and_then(Font::units_per_em),
            Self::System(font) => font.units_per_em(),
        }
    }

    fn h_advance_unscaled(&self, glyph_id: ab_glyph::GlyphId) -> f32 {
        match self {
            Self::Parsed(font) => font.h_advance_unscaled(glyph_id),
            Self::Raw(raw) => raw
                .get()
                .map_or(0.0, |font| font.h_advance_unscaled(glyph_id)),
            Self::System(font) => font.h_advance_unscaled(glyph_id),
        }
    }

    fn cached_outline(
        &self,
        font_id: FontId,
        source_id: OutlineSourceId,
        glyph_id: ab_glyph::GlyphId,
    ) -> Option<std::sync::Arc<[OutlineCurve]>> {
        match self {
            Self::Parsed(font) => ratex_font_loader::outline_cache::get_or_compute_outline_fontvec(
                font_id, font, source_id, glyph_id,
            ),
            Self::Raw(raw) => raw.get().and_then(|font| {
                ratex_font_loader::outline_cache::get_or_compute_outline_with_source_id(
                    font_id, font, source_id, glyph_id,
                )
            }),
            Self::System(font) => {
                ratex_font_loader::outline_cache::get_or_compute_outline_with_source_id(
                    font_id, font, source_id, glyph_id,
                )
            }
        }
    }
}

pub(crate) struct SvgFontRef<'a> {
    font: SvgFont<'a>,
    source_id: OutlineSourceId,
}

/// Build a map combining cached small `FontVec`s with borrowed raw large fonts.
pub(crate) fn build_font_refs(data: &ParsedFontSet) -> HashMap<FontId, SvgFontRef<'_>> {
    let mut refs = HashMap::new();
    for (id, font, source_id) in data.iter_with_source() {
        refs.insert(
            *id,
            SvgFontRef {
                font: SvgFont::Parsed(font),
                source_id,
            },
        );
    }
    for (id, bytes, source_id) in data.iter_raw_with_source() {
        refs.insert(
            *id,
            SvgFontRef {
                font: SvgFont::Raw(Box::new(LazyRawFont {
                    bytes,
                    face_index: ratex_font_loader::font_face_index(*id),
                    parsed: OnceCell::new(),
                })),
                source_id,
            },
        );
    }
    refs
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
    font_cache: &HashMap<FontId, SvgFontRef<'_>>,
    system_fonts: &SystemFontResolver,
) -> Option<StandaloneGlyph> {
    let font_id = FontId::parse(font_name).unwrap_or(FontId::MainRegular);
    let font_entry = match font_cache.get(&font_id) {
        Some(entry) => entry,
        None => font_cache.get(&FontId::MainRegular)?,
    };
    let font = &font_entry.font;

    let ch = ratex_font::katex_ttf_glyph_char(font_id, char_code);
    let glyph_id = font.glyph_id(ch);

    if glyph_id.0 == 0 {
        return try_system_unicode_fallback_svg(
            px,
            py,
            glyph_em,
            ch,
            font_cache,
            system_fonts,
            false,
        );
    }

    if font_id == FontId::EmojiFallback {
        return try_emoji_raster_or_vector_svg(
            px,
            py,
            glyph_em,
            ch,
            font_entry.source_id,
            font,
            glyph_id,
        );
    }

    if font_id == FontId::CjkRegular {
        if let Some(d) = outline_to_d(
            px,
            py,
            glyph_em,
            FontId::CjkRegular,
            font_entry.source_id,
            font,
            glyph_id,
        ) {
            return Some(StandaloneGlyph::Path(d));
        }
        if let Some(g) =
            try_emoji_raster_then_vector_svg(px, py, glyph_em, ch, font_cache, system_fonts)
        {
            return Some(g);
        }
        return outline_char_with_system_fallback(
            px,
            py,
            glyph_em,
            ch,
            FontId::CjkFallback,
            font_cache,
            system_fonts,
        );
    }

    if font_id == FontId::CjkFallback {
        if let Some(d) = outline_to_d(
            px,
            py,
            glyph_em,
            FontId::CjkFallback,
            font_entry.source_id,
            font,
            glyph_id,
        ) {
            return Some(StandaloneGlyph::Path(d));
        }
        return try_emoji_raster_then_vector_svg(px, py, glyph_em, ch, font_cache, system_fonts);
    }

    if let Some(d) = outline_to_d(
        px,
        py,
        glyph_em,
        font_id,
        font_entry.source_id,
        font,
        glyph_id,
    ) {
        return Some(StandaloneGlyph::Path(d));
    }

    let skip_main = font_id == FontId::MainRegular;
    try_system_unicode_fallback_svg(px, py, glyph_em, ch, font_cache, system_fonts, skip_main)
}

fn outline_char_with_entry(
    px: f32,
    py: f32,
    em: f32,
    ch: char,
    font_id: FontId,
    entry: &SvgFontRef<'_>,
) -> Option<StandaloneGlyph> {
    let glyph_id = entry.font.glyph_id(ch);
    if glyph_id.0 == 0 {
        return None;
    }
    outline_to_d(px, py, em, font_id, entry.source_id, &entry.font, glyph_id)
        .map(StandaloneGlyph::Path)
}

fn outline_char_with_system_fallback(
    px: f32,
    py: f32,
    em: f32,
    ch: char,
    font_id: FontId,
    font_cache: &HashMap<FontId, SvgFontRef<'_>>,
    system_fonts: &SystemFontResolver,
) -> Option<StandaloneGlyph> {
    if let Some(entry) = font_cache.get(&font_id) {
        return outline_char_with_entry(px, py, em, ch, font_id, entry);
    }

    let resolved = system_fonts.get(font_id).ok().flatten()?;
    let entry = SvgFontRef {
        font: SvgFont::System(resolved.font()),
        source_id: resolved.source_id(),
    };
    outline_char_with_entry(px, py, em, ch, font_id, &entry)
}

fn try_emoji_png_data_url(px: f32, py: f32, em: f32, ch: char) -> Option<StandaloneGlyph> {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    if !ratex_unicode_font::is_emoji_candidate(ch) {
        return None;
    }

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
    font_cache: &HashMap<FontId, SvgFontRef<'_>>,
    system_fonts: &SystemFontResolver,
) -> Option<StandaloneGlyph> {
    if !ratex_unicode_font::is_emoji_candidate(ch) {
        return None;
    }
    if let Some(img) = try_emoji_png_data_url(px, py, em, ch) {
        return Some(img);
    }
    outline_char_with_system_fallback(
        px,
        py,
        em,
        ch,
        FontId::EmojiFallback,
        font_cache,
        system_fonts,
    )
}
fn try_emoji_raster_or_vector_svg(
    px: f32,
    py: f32,
    em: f32,
    ch: char,
    source_id: OutlineSourceId,
    font: &SvgFont<'_>,
    glyph_id: ab_glyph::GlyphId,
) -> Option<StandaloneGlyph> {
    if let Some(img) = try_emoji_png_data_url(px, py, em, ch) {
        return Some(img);
    }
    outline_to_d(px, py, em, FontId::EmojiFallback, source_id, font, glyph_id)
        .map(StandaloneGlyph::Path)
}
fn try_system_unicode_fallback_svg(
    px: f32,
    py: f32,
    em: f32,
    ch: char,
    font_cache: &HashMap<FontId, SvgFontRef<'_>>,
    system_fonts: &SystemFontResolver,
    skip_main_regular: bool,
) -> Option<StandaloneGlyph> {
    if !skip_main_regular {
        if let Some(fallback) = font_cache.get(&FontId::MainRegular) {
            let fid = fallback.font.glyph_id(ch);
            if fid.0 != 0 {
                if let Some(d) = outline_to_d(
                    px,
                    py,
                    em,
                    FontId::MainRegular,
                    fallback.source_id,
                    &fallback.font,
                    fid,
                ) {
                    return Some(StandaloneGlyph::Path(d));
                }
            }
        }
    }
    if let Some(glyph) = outline_char_with_system_fallback(
        px,
        py,
        em,
        ch,
        FontId::CjkRegular,
        font_cache,
        system_fonts,
    ) {
        return Some(glyph);
    }
    if let Some(g) = try_emoji_raster_then_vector_svg(px, py, em, ch, font_cache, system_fonts) {
        return Some(g);
    }
    outline_char_with_system_fallback(
        px,
        py,
        em,
        ch,
        FontId::CjkFallback,
        font_cache,
        system_fonts,
    )
}
fn outline_to_d(
    px: f32,
    py: f32,
    em: f32,
    font_id: FontId,
    source_id: OutlineSourceId,
    font: &SvgFont<'_>,
    glyph_id: ab_glyph::GlyphId,
) -> Option<String> {
    let mut d = String::with_capacity(256);
    if outline_to_d_into(&mut d, px, py, em, font_id, source_id, font, glyph_id) {
        Some(d)
    } else {
        None
    }
}

/// Append the glyph outline as SVG path data to `out`, returning whether any
/// command was written. Equivalent to the legacy `outline_to_d` (which trimmed
/// the result and returned `None` for empty output).
#[allow(clippy::too_many_arguments)]
fn outline_to_d_into(
    out: &mut String,
    px: f32,
    py: f32,
    em: f32,
    font_id: FontId,
    source_id: OutlineSourceId,
    font: &SvgFont<'_>,
    glyph_id: ab_glyph::GlyphId,
) -> bool {
    let Some(curves) = font.cached_outline(font_id, source_id, glyph_id) else {
        return false;
    };
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

    let start_len = out.len();
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
                out.push('Z');
                out.push(' ');
            }
            out.push('M');
            crate::fmt_num_to(out, start.0 as f64);
            out.push(' ');
            crate::fmt_num_to(out, start.1 as f64);
            out.push(' ');
        }

        match curve {
            OutlineCurve::Line(_, p1) => {
                out.push('L');
                crate::fmt_num_to(out, (px + p1.x * scale) as f64);
                out.push(' ');
                crate::fmt_num_to(out, (py - p1.y * scale) as f64);
                out.push(' ');
            }
            OutlineCurve::Quad(_, p1, p2) => {
                out.push('Q');
                crate::fmt_num_to(out, (px + p1.x * scale) as f64);
                out.push(' ');
                crate::fmt_num_to(out, (py - p1.y * scale) as f64);
                out.push(' ');
                crate::fmt_num_to(out, (px + p2.x * scale) as f64);
                out.push(' ');
                crate::fmt_num_to(out, (py - p2.y * scale) as f64);
                out.push(' ');
            }
            OutlineCurve::Cubic(_, p1, p2, p3) => {
                out.push('C');
                crate::fmt_num_to(out, (px + p1.x * scale) as f64);
                out.push(' ');
                crate::fmt_num_to(out, (py - p1.y * scale) as f64);
                out.push(' ');
                crate::fmt_num_to(out, (px + p2.x * scale) as f64);
                out.push(' ');
                crate::fmt_num_to(out, (py - p2.y * scale) as f64);
                out.push(' ');
                crate::fmt_num_to(out, (px + p3.x * scale) as f64);
                out.push(' ');
                crate::fmt_num_to(out, (py - p3.y * scale) as f64);
                out.push(' ');
            }
        }

        last_end = Some(end);
    }

    if last_end.is_some() {
        out.push('Z');
    }

    // `d.trim()`: no leading whitespace is ever written, so this is trim-end.
    let bytes = out.as_bytes();
    let mut end = out.len();
    while end > start_len && bytes[end - 1].is_ascii_whitespace() {
        end -= 1;
    }
    out.truncate(end);
    end > start_len
}
