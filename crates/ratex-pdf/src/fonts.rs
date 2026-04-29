//! Font loading, subsetting, and CIDFont embedding for pdf-writer.

use std::collections::{BTreeMap, HashMap, HashSet};

use ab_glyph::Font as _;
use pdf_writer::{types::*, Filter, Finish, Name, Pdf, Ref, Str};
use ratex_font::FontId;
use skrifa::instance::{LocationRef, Size};
use skrifa::outline::{DrawSettings, OutlinePen};
use skrifa::raw::FontRef as SfFontRef;
use skrifa::raw::TableProvider;
use skrifa::{GlyphId, MetadataProvider};
use subsetter::GlyphRemapper;

/// Raw TTF bytes keyed by FontId.
pub(crate) type RawFontData = HashMap<FontId, Vec<u8>>;

/// The 19 KaTeX font faces.
pub(crate) const FONT_MAP: &[(FontId, &str)] = &[
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

/// Load all KaTeX TTF fonts.
#[allow(unused_variables)]
pub(crate) fn load_all_fonts(font_dir: &str) -> Result<RawFontData, String> {
    let mut data = HashMap::new();

    #[cfg(not(feature = "embed-fonts"))]
    {
        let dir = std::path::Path::new(font_dir);
        for (id, filename) in FONT_MAP {
            let path = dir.join(filename);
            if path.exists() {
                let bytes = std::fs::read(&path)
                    .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
                data.insert(*id, bytes);
            }
        }
        if data.is_empty() {
            return Err(format!("No fonts found in {font_dir}"));
        }
    }

    #[cfg(feature = "embed-fonts")]
    {
        for (id, filename) in FONT_MAP {
            let cow = ratex_katex_fonts::ttf_bytes(filename)
                .ok_or_else(|| format!("Missing embedded font {filename}"))?;
            data.insert(*id, cow.to_vec());
        }
    }

    // Load system Unicode font for CJK/fallback glyphs.
    if let Some(cjk_bytes) = ratex_unicode_font::load_unicode_font() {
        data.entry(FontId::CjkRegular)
            .or_insert_with(|| cjk_bytes.to_vec());
    }
    // Secondary system fallback for characters the primary CJK font doesn't cover
    // (e.g. emoji when RATEX_UNICODE_FONT points to a CJK-only font).
    if let Some(fb_bytes) = ratex_unicode_font::load_fallback_font() {
        data.entry(FontId::CjkFallback)
            .or_insert_with(|| fb_bytes.to_vec());
    }
    if let Some(emoji_bytes) = ratex_unicode_font::load_emoji_font() {
        data.entry(FontId::EmojiFallback)
            .or_insert_with(|| emoji_bytes.to_vec());
    }

    Ok(data)
}

/// `ab_glyph` / OpenType cmap (same stack as PNG/SVG).
fn resolve_glyph_id_abglyph(raw_bytes: &[u8], font_id: FontId, char_code: u32) -> Option<u16> {
    let ch = ratex_font::katex_ttf_glyph_char(font_id, char_code);
    let idx = skrifa_collection_index(font_id);
    let font = ab_glyph::FontRef::try_from_slice_and_index(raw_bytes, idx).ok()?;
    let gid = font.glyph_id(ch);
    if gid.0 == 0 {
        None
    } else {
        Some(gid.0)
    }
}

#[inline]
fn skrifa_collection_index(face_id: FontId) -> u32 {
    match face_id {
        FontId::EmojiFallback => ratex_unicode_font::emoji_font_face_index().unwrap_or(0),
        FontId::CjkRegular => ratex_unicode_font::unicode_font_face_index().unwrap_or(0),
        FontId::CjkFallback => ratex_unicode_font::fallback_font_face_index().unwrap_or(0),
        _ => 0,
    }
}

/// Always use `ab_glyph` / OpenType cmap (same as PNG/SVG). Skrifa's default cmap can map
/// supplementary-plane emoji in `KaTeX_Main-Regular` to a non-zero GID while `ab_glyph` (and our
/// layout) treat them as absent — that would make [`resolve_pdf_glyph`] stop at `MainRegular` and
/// never reach [`FontId::EmojiFallback`], so color emoji rasters were never collected or drawn.
#[inline]
fn resolve_glyph_id_for_face(raw_bytes: &[u8], font_id: FontId, char_code: u32) -> Option<u16> {
    resolve_glyph_id_abglyph(raw_bytes, font_id, char_code)
}

/// True if the glyph has drawable outline segments (not just a lone `move_to` / empty COLR mask).
pub(crate) fn glyph_has_nonempty_outline(raw_bytes: &[u8], face_id: FontId, gid: u16) -> bool {
    let font = match SfFontRef::from_index(raw_bytes, skrifa_collection_index(face_id)) {
        Ok(f) => f,
        Err(_) => return false,
    };
    let outlines = font.outline_glyphs();
    let Some(glyph) = outlines.get(GlyphId::new(gid as u32)) else {
        return false;
    };
    #[derive(Default)]
    struct PenStats {
        /// `line_to` / `quad_to` / `curve_to` only — excludes `move_to` and `close`.
        draws: usize,
    }
    impl OutlinePen for PenStats {
        fn move_to(&mut self, _: f32, _: f32) {}
        fn line_to(&mut self, _: f32, _: f32) {
            self.draws += 1;
        }
        fn quad_to(&mut self, _: f32, _: f32, _: f32, _: f32) {
            self.draws += 1;
        }
        fn curve_to(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: f32) {
            self.draws += 1;
        }
        fn close(&mut self) {}
    }
    let mut pen = PenStats::default();
    let settings = DrawSettings::unhinted(Size::new(16.0), LocationRef::default());
    glyph.draw(settings, &mut pen).is_ok() && pen.draws > 0
}

/// Codepoints that must use sbix PNG rasters in PDF (not vector subset), when a color emoji
/// font is available.
///
/// **Only** the supplementary emoji blocks are included. Do **not** add whole
/// Miscellaneous Symbols (`U+2600`–`U+26FF`) or Dingbats: color fonts often have no PNG strike
/// there (e.g. ★♠♣), and [`embed_emoji_rasters`] would fail the entire PDF. Those glyphs should
/// embed from `CjkRegular` / fallback outlines like ←↑→↓.
#[inline]
pub(crate) fn prefer_color_emoji_raster(char_code: u32) -> bool {
    matches!(char_code, 0x1F000..=0x1FAFF)
}

/// Single source of truth for which font face and GID to subset and show (aligned with PNG/SVG).
///
/// Order: requested → `MainRegular` → `CjkRegular` (only when the display item was not already
/// `CjkRegular`) → `EmojiFallback` → `CjkFallback`. `CjkRegular` cmap hits are ignored unless
/// [`glyph_has_nonempty_outline`] is true so narrow fonts (e.g. AppleGothic missing SC hanzi)
/// fall through. `EmojiFallback` is tried before the broad text face so color emoji are not stuck
/// behind fonts that lack emoji coverage. `CjkFallback` does not require outlines.
///
/// Emoji blocks never settle on `MainRegular` / `CjkRegular` placeholder glyphs so
/// [`collect_glyph_usage`] can emit `EmojiFallback` rasters.
pub(crate) fn resolve_pdf_glyph(
    font_data: &RawFontData,
    font_name: &str,
    char_code: u32,
) -> Option<(FontId, u16)> {
    let font_id = FontId::parse(font_name).unwrap_or(FontId::MainRegular);

    // 1. Requested font
    if let Some(bytes) = font_data.get(&font_id) {
        if let Some(gid) = resolve_glyph_id_for_face(bytes, font_id, char_code) {
            if prefer_color_emoji_raster(char_code) {
                if font_id == FontId::EmojiFallback {
                    return Some((font_id, gid));
                }
            } else if font_id != FontId::CjkRegular || glyph_has_nonempty_outline(bytes, font_id, gid)
            {
                return Some((font_id, gid));
            }
        }
    }
    // 2. MainRegular
    if let Some(bytes) = font_data.get(&FontId::MainRegular) {
        if let Some(gid) = resolve_glyph_id_for_face(bytes, FontId::MainRegular, char_code) {
            if !prefer_color_emoji_raster(char_code) {
                return Some((FontId::MainRegular, gid));
            }
        }
    }
    // 3. CjkRegular — skip when the item already used that face (step 1 tried it).
    if font_id != FontId::CjkRegular {
        if let Some(bytes) = font_data.get(&FontId::CjkRegular) {
            if let Some(gid) = resolve_glyph_id_for_face(bytes, FontId::CjkRegular, char_code) {
                if glyph_has_nonempty_outline(bytes, FontId::CjkRegular, gid)
                    && !prefer_color_emoji_raster(char_code)
                {
                    return Some((FontId::CjkRegular, gid));
                }
            }
        }
    }
    // 4. EmojiFallback (color emoji — before broad text fallback, aligned with PNG)
    if let Some(bytes) = font_data.get(&FontId::EmojiFallback) {
        if let Some(gid) = resolve_glyph_id_for_face(bytes, FontId::EmojiFallback, char_code) {
            return Some((FontId::EmojiFallback, gid));
        }
    }
    // 5. CjkFallback
    if let Some(bytes) = font_data.get(&FontId::CjkFallback) {
        if let Some(gid) = resolve_glyph_id_for_face(bytes, FontId::CjkFallback, char_code) {
            return Some((FontId::CjkFallback, gid));
        }
    }
    None
}

/// Info about a glyph we want to embed.
#[derive(Clone, Debug)]
pub(crate) struct GlyphInfo {
    /// Unicode codepoint for ToUnicode CMap.
    pub unicode: u32,
}

/// Collected usage for one font.
pub(crate) struct FontUsage {
    pub font_id: FontId,
    /// gid → GlyphInfo
    pub glyphs: BTreeMap<u16, GlyphInfo>,
}

/// One emoji codepoint that resolves to `EmojiFallback`, with the largest `glyph_em` seen (px).
pub(crate) struct EmojiRasterUsage {
    pub char_code: u32,
    pub max_glyph_em_px: f32,
}

pub(crate) struct CollectedGlyphs {
    pub font_usages: Vec<FontUsage>,
    pub emoji_rasters: Vec<EmojiRasterUsage>,
}

/// Embedded color-emoji bitmap (PDF image XObject), aligned with PNG/SVG sbix strikes.
pub(crate) struct EmbeddedEmojiImage {
    pub char_code: u32,
    pub res_name: String,
    pub image_ref: Ref,
    pub strike_x: i16,
    pub strike_y: i16,
    pub width_px: u16,
    pub height_px: u16,
    pub pixels_per_em: u16,
}

/// Collect font subset usage and emoji raster usage (`EmojiFallback` is drawn as images, not Type0).
pub(crate) fn collect_glyph_usage(
    items: &[ratex_types::display_item::DisplayItem],
    font_data: &RawFontData,
    body_em: f64,
) -> CollectedGlyphs {
    let mut usage_map: HashMap<FontId, HashSet<(u16, u32)>> = HashMap::new();
    let mut emoji_max: HashMap<u32, f32> = HashMap::new();

    for item in items {
        if let ratex_types::display_item::DisplayItem::GlyphPath {
            font,
            char_code,
            scale,
            ..
        } = item
        {
            let glyph_em = (*scale * body_em) as f32;
            // Always collect sbix rasters for emoji / dingbat blocks when a color font is loaded,
            // independent of [`resolve_pdf_glyph`] (avoids edge cases where CJK/Main still "claim" a CP).
            if prefer_color_emoji_raster(*char_code)
                && ratex_unicode_font::load_emoji_font().is_some()
            {
                emoji_max
                    .entry(*char_code)
                    .and_modify(|m| *m = (*m).max(glyph_em))
                    .or_insert(glyph_em);
                continue;
            }
            if let Some((face, gid)) = resolve_pdf_glyph(font_data, font, *char_code) {
                if face == FontId::EmojiFallback {
                    emoji_max
                        .entry(*char_code)
                        .and_modify(|m| *m = (*m).max(glyph_em))
                        .or_insert(glyph_em);
                    continue;
                }
                usage_map
                    .entry(face)
                    .or_default()
                    .insert((gid, *char_code));
            }
        }
    }

    let mut font_usages: Vec<FontUsage> = usage_map
        .into_iter()
        .map(|(font_id, set)| {
            let mut glyphs = BTreeMap::new();
            for (gid, unicode) in set {
                glyphs.insert(gid, GlyphInfo { unicode });
            }
            FontUsage { font_id, glyphs }
        })
        .collect();
    font_usages.sort_by_key(|u| u.font_id.as_str().to_string());

    let mut emoji_rasters: Vec<EmojiRasterUsage> = emoji_max
        .into_iter()
        .map(|(char_code, max_glyph_em_px)| EmojiRasterUsage {
            char_code,
            max_glyph_em_px,
        })
        .collect();
    emoji_rasters.sort_by_key(|e| e.char_code);

    CollectedGlyphs {
        font_usages,
        emoji_rasters,
    }
}

/// Write PNG sbix strikes as opaque DeviceRGB image XObjects (alpha composited on white).
pub(crate) fn embed_emoji_rasters(
    pdf: &mut Pdf,
    alloc: &mut Ref,
    usages: &[EmojiRasterUsage],
) -> Result<Vec<EmbeddedEmojiImage>, String> {
    let mut out = Vec::with_capacity(usages.len());
    for (i, u) in usages.iter().enumerate() {
        let ch = char::from_u32(u.char_code)
            .ok_or_else(|| format!("invalid char code {}", u.char_code))?;
        let strike = ratex_unicode_font::emoji_png_raster_for_char(ch, u.max_glyph_em_px)
            .ok_or_else(|| format!("emoji PNG strike missing for U+{:04X}", u.char_code))?;

        let (w, h, rgba) = decode_png_rgba8(&strike.data)?;
        if w != u32::from(strike.width) || h != u32::from(strike.height) {
            return Err(format!(
                "PNG size mismatch for U+{:04X}: got {}x{}, expected {}x{}",
                u.char_code,
                w,
                h,
                strike.width,
                strike.height
            ));
        }

        // Composite onto white and emit a single opaque DeviceRGB stream. Many viewers mishandle
        // `/SMask` on small Flate RGB images; this matches a white page and fixes "invisible" emoji.
        let mut rgb = Vec::with_capacity((w * h * 3) as usize);
        for p in rgba.chunks_exact(4) {
            let a = p[3] as f32 / 255.0;
            let r = (p[0] as f32 * a + 255.0 * (1.0 - a)).round() as u8;
            let g = (p[1] as f32 * a + 255.0 * (1.0 - a)).round() as u8;
            let b = (p[2] as f32 * a + 255.0 * (1.0 - a)).round() as u8;
            rgb.extend_from_slice(&[r, g, b]);
        }
        let encoded_rgb = miniz_oxide::deflate::compress_to_vec_zlib(&rgb, 6);

        let image_ref = alloc.bump();

        let mut image = pdf.image_xobject(image_ref, &encoded_rgb);
        image.filter(Filter::FlateDecode);
        image.width(w as i32);
        image.height(h as i32);
        image.color_space().device_rgb();
        image.bits_per_component(8);
        image.finish();

        out.push(EmbeddedEmojiImage {
            char_code: u.char_code,
            res_name: format!("E{i}"),
            image_ref,
            strike_x: strike.x,
            strike_y: strike.y,
            width_px: strike.width,
            height_px: strike.height,
            pixels_per_em: strike.pixels_per_em,
        });
    }
    Ok(out)
}

fn decode_png_rgba8(data: &[u8]) -> Result<(u32, u32, Vec<u8>), String> {
    let dec = png::Decoder::new(std::io::Cursor::new(data));
    let mut reader = dec.read_info().map_err(|e| format!("png: {e}"))?;
    let ct = reader.info().color_type;
    let bd = reader.info().bit_depth;
    if bd != png::BitDepth::Eight {
        return Err(format!("unsupported png bit depth: {bd:?}"));
    }
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("png frame: {e}"))?;
    let (w, h) = (info.width, info.height);
    match ct {
        png::ColorType::Rgba => Ok((w, h, buf)),
        png::ColorType::Rgb => {
            let mut rgba = Vec::with_capacity(buf.len() / 3 * 4);
            for p in buf.chunks_exact(3) {
                rgba.extend_from_slice(&[p[0], p[1], p[2], 255]);
            }
            Ok((w, h, rgba))
        }
        _ => Err(format!("unsupported png color type: {ct:?}")),
    }
}

/// Result of embedding one font into the PDF.
pub(crate) struct EmbeddedFont {
    pub font_id: FontId,
    /// PDF resource name, e.g. "F0", "F1"
    pub res_name: String,
    /// The Type0 font reference for the page Resources dict.
    pub type0_ref: Ref,
    /// Old GID → new CID mapping.
    pub remapper: GlyphRemapper,
}

/// Embed all used fonts into the PDF and return mapping info.
pub(crate) fn embed_fonts(
    pdf: &mut Pdf,
    alloc: &mut Ref,
    usages: &[FontUsage],
    font_data: &RawFontData,
) -> Result<Vec<EmbeddedFont>, String> {
    let mut embedded = Vec::new();

    for (idx, usage) in usages.iter().enumerate() {
        let raw = font_data
            .get(&usage.font_id)
            .ok_or_else(|| format!("Missing font data for {:?}", usage.font_id))?;

        // Build GlyphRemapper with all used glyph IDs.
        let mut remapper = GlyphRemapper::new();
        for &gid in usage.glyphs.keys() {
            remapper.remap(gid);
        }

        // Subset the font.
        let subsetted = subsetter::subset(raw, skrifa_collection_index(usage.font_id), &remapper)
            .map_err(|e| format!("Subset error for {:?}: {e}", usage.font_id))?;

        // Compress the subset.
        let compressed =
            miniz_oxide::deflate::compress_to_vec_zlib(&subsetted, 6);

        // Read font metrics via skrifa.
        let sf = SfFontRef::from_index(raw, skrifa_collection_index(usage.font_id))
            .map_err(|e| format!("skrifa error: {e}"))?;
        let upem = sf.head().map_err(|_| "no head table")?.units_per_em() as f32;
        let scale = 1000.0 / upem; // PDF uses 1000 units per em for metrics

        let (ascent, descent, cap_height) = if let Ok(os2) = sf.os2() {
            let asc = os2.s_typo_ascender() as f32 * scale;
            let desc = os2.s_typo_descender() as f32 * scale;
            let cap = os2.s_cap_height().map_or(asc, |v| v as f32 * scale);
            (asc, desc, cap)
        } else {
            (800.0, -200.0, 800.0)
        };

        let bbox = {
            let head = sf.head().map_err(|_| "no head table")?;
            [
                head.x_min() as f32 * scale,
                head.y_min() as f32 * scale,
                head.x_max() as f32 * scale,
                head.y_max() as f32 * scale,
            ]
        };

        // Glyph widths (in 1000-unit space).
        let hmtx = sf.hmtx().map_err(|_| "no hmtx table")?;
        let mut widths: Vec<(u16, f32)> = Vec::new();
        for &old_gid in usage.glyphs.keys() {
            let new_cid = remapper.get(old_gid).unwrap_or(0);
            let gid = skrifa::raw::types::GlyphId::new(old_gid as u32);
            let advance = hmtx.advance(gid).unwrap_or(0) as f32 * scale;
            widths.push((new_cid, advance));
        }
        widths.sort_by_key(|(cid, _)| *cid);

        // Allocate PDF object refs.
        let type0_ref = alloc.bump();
        let cid_ref = alloc.bump();
        let descriptor_ref = alloc.bump();
        let tounicode_ref = alloc.bump();
        let stream_ref = alloc.bump();

        let base_name = format!("KaTeX_{}", usage.font_id.as_str().replace('-', "_"));
        let res_name = format!("F{idx}");

        // FontDescriptor
        pdf.font_descriptor(descriptor_ref)
            .name(Name(base_name.as_bytes()))
            .flags(FontFlags::SYMBOLIC)
            .bbox(pdf_writer::Rect::new(bbox[0], bbox[1], bbox[2], bbox[3]))
            .italic_angle(0.0)
            .ascent(ascent)
            .descent(descent)
            .cap_height(cap_height)
            .stem_v(80.0)
            .font_file2(stream_ref);

        // CIDFont (Type2)
        let mut cid_font = pdf.cid_font(cid_ref);
        cid_font
            .subtype(CidFontType::Type2)
            .base_font(Name(base_name.as_bytes()))
            .default_width(0.0)
            .font_descriptor(descriptor_ref);
        cid_font
            .system_info(pdf_writer::types::SystemInfo {
                registry: Str(b"Adobe"),
                ordering: Str(b"Identity"),
                supplement: 0,
            });

        // W array (widths per CID).
        if !widths.is_empty() {
            let mut w = cid_font.widths();
            for &(cid, adv) in &widths {
                w.consecutive(cid, [adv]);
            }
            w.finish();
        }
        cid_font.finish();

        // Type0 (composite) font
        pdf.type0_font(type0_ref)
            .base_font(Name(base_name.as_bytes()))
            .encoding_predefined(Name(b"Identity-H"))
            .descendant_font(cid_ref)
            .to_unicode(tounicode_ref);

        // ToUnicode CMap
        let cmap = build_tounicode_cmap(&usage.glyphs, &remapper);
        pdf.stream(tounicode_ref, cmap.as_bytes())
            .pair(Name(b"Type"), Name(b"CMap"));

        // FontFile2 stream (compressed)
        let mut font_stream = pdf.stream(stream_ref, &compressed);
        font_stream.filter(Filter::FlateDecode);
        font_stream.pair(Name(b"Length1"), subsetted.len() as i32);
        font_stream.finish();

        embedded.push(EmbeddedFont {
            font_id: usage.font_id,
            res_name,
            type0_ref,
            remapper,
        });
    }

    Ok(embedded)
}

/// Build a ToUnicode CMap for PDF text extraction.
fn build_tounicode_cmap(glyphs: &BTreeMap<u16, GlyphInfo>, remapper: &GlyphRemapper) -> String {
    let mut entries = Vec::new();
    for (old_gid, info) in glyphs {
        if let Some(new_cid) = remapper.get(*old_gid) {
            entries.push((new_cid, info.unicode));
        }
    }
    entries.sort_by_key(|(cid, _)| *cid);

    let mut cmap = String::new();
    cmap.push_str("/CIDInit /ProcSet findresource begin\n");
    cmap.push_str("12 dict begin\n");
    cmap.push_str("begincmap\n");
    cmap.push_str("/CIDSystemInfo\n");
    cmap.push_str("<< /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n");
    cmap.push_str("/CMapName /Adobe-Identity-UCS def\n");
    cmap.push_str("/CMapType 2 def\n");
    cmap.push_str("1 begincodespacerange\n");
    cmap.push_str("<0000> <FFFF>\n");
    cmap.push_str("endcodespacerange\n");

    // Write in chunks of 100 (PDF spec limit per block).
    for chunk in entries.chunks(100) {
        cmap.push_str(&format!("{} beginbfchar\n", chunk.len()));
        for &(cid, unicode) in chunk {
            if unicode <= 0xFFFF {
                cmap.push_str(&format!("<{:04X}> <{:04X}>\n", cid, unicode));
            } else {
                // Supplementary plane → UTF-16 surrogate pair.
                let hi = ((unicode - 0x10000) >> 10) + 0xD800;
                let lo = ((unicode - 0x10000) & 0x3FF) + 0xDC00;
                cmap.push_str(&format!("<{:04X}> <{:04X}{:04X}>\n", cid, hi, lo));
            }
        }
        cmap.push_str("endbfchar\n");
    }

    cmap.push_str("endcmap\n");
    cmap.push_str("CMapName currentdict /CMap defineresource pop\n");
    cmap.push_str("end\n");
    cmap.push_str("end\n");
    cmap
}

#[cfg(all(test, target_os = "macos"))]
mod macos_cjk_pdf_tests {
    use super::*;
    use ratex_font::FontId;
    use std::collections::HashMap;
    use std::path::Path;

    const APPLE_GOTHIC: &str = "/System/Library/Fonts/Supplemental/AppleGothic.ttf";
    const ARIAL_UNICODE: &str = "/System/Library/Fonts/Supplemental/Arial Unicode.ttf";

    #[test]
    fn applegothic_missing_sc_hanzi_abglyph_sees_unmapped() {
        let bytes = std::fs::read(APPLE_GOTHIC).expect("AppleGothic");
        for cp in [0x6C27u32, 0x78B3u32] {
            assert!(
                resolve_glyph_id_abglyph(&bytes, FontId::CjkRegular, cp).is_none(),
                "U+{cp:04X} must be unmapped in AppleGothic for PNG/PDF parity"
            );
        }
    }

    #[test]
    fn resolve_pdf_glyph_falls_back_for_missing_sc_in_applegothic() {
        let ag = std::fs::read(APPLE_GOTHIC).expect("AppleGothic");
        let au = std::fs::read(ARIAL_UNICODE).expect("Arial Unicode");
        let main_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fonts/KaTeX_Main-Regular.ttf");
        let main = std::fs::read(main_path).expect("KaTeX_Main-Regular");
        let mut data = HashMap::new();
        data.insert(FontId::MainRegular, main);
        data.insert(FontId::CjkRegular, ag);
        data.insert(FontId::CjkFallback, au);
        for cp in [0x6C27u32, 0x78B3u32] {
            let r = resolve_pdf_glyph(&data, "CJK-Regular", cp);
            assert!(
                matches!(r, Some((FontId::CjkFallback, _))),
                "U+{cp:04X}: expected CjkFallback, got {r:?}"
            );
        }
    }

    /// Regression: skrifa Main cmap could map emoji to a GID and block `EmojiFallback`; `ab_glyph` must not.
    #[test]
    fn resolve_pdf_glyph_uses_emoji_face_for_grinning() {
        let ag = std::fs::read(APPLE_GOTHIC).expect("AppleGothic");
        let main_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fonts/KaTeX_Main-Regular.ttf");
        let main = std::fs::read(main_path).expect("KaTeX_Main-Regular");
        let emoji = ratex_unicode_font::load_emoji_font().expect("system emoji font");
        let mut data = HashMap::new();
        data.insert(FontId::MainRegular, main);
        data.insert(FontId::CjkRegular, ag);
        data.insert(FontId::EmojiFallback, emoji.to_vec());
        let r = resolve_pdf_glyph(&data, "CJK-Regular", 0x1F600);
        assert!(
            matches!(r, Some((FontId::EmojiFallback, _))),
            "expected EmojiFallback for U+1F600, got {r:?}"
        );
    }
}
