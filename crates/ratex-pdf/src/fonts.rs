//! Font loading, subsetting, and CIDFont embedding for pdf-writer.

use std::collections::{BTreeMap, HashMap, HashSet};

use pdf_writer::{types::*, Filter, Finish, Name, Pdf, Ref, Str};
use ratex_font::FontId;
use skrifa::raw::FontRef as SfFontRef;
use skrifa::raw::TableProvider;
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

    Ok(data)
}

/// Resolve (FontId, char_code) → skrifa GlyphId (u16).
pub(crate) fn resolve_glyph_id(raw_bytes: &[u8], font_id: FontId, char_code: u32) -> Option<u16> {
    let mapped_char = ratex_font::katex_ttf_glyph_char(font_id, char_code);
    let sf = SfFontRef::new(raw_bytes).ok()?;
    let charmap = skrifa::charmap::Charmap::new(&sf);
    let gid = charmap.map(mapped_char)?;
    let v = gid.to_u32();
    if v == 0 { None } else { Some(v as u16) }
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

/// Collect which fonts & glyphs are used in the display list.
pub(crate) fn collect_glyph_usage(
    items: &[ratex_types::display_item::DisplayItem],
    font_data: &RawFontData,
) -> Vec<FontUsage> {
    let mut usage_map: HashMap<FontId, HashSet<(u16, u32)>> = HashMap::new();

    for item in items {
        if let ratex_types::display_item::DisplayItem::GlyphPath {
            font, char_code, ..
        } = item
        {
            let font_id = FontId::parse(font).unwrap_or(FontId::MainRegular);
            let bytes = match font_data.get(&font_id) {
                Some(b) => b,
                None => match font_data.get(&FontId::MainRegular) {
                    Some(b) => b,
                    None => continue,
                },
            };
            let actual_fid = if font_data.contains_key(&font_id) {
                font_id
            } else {
                FontId::MainRegular
            };
            if let Some(gid) = resolve_glyph_id(bytes, font_id, *char_code) {
                usage_map
                    .entry(actual_fid)
                    .or_default()
                    .insert((gid, *char_code));
            }
        }
    }

    let mut usages: Vec<FontUsage> = usage_map
        .into_iter()
        .map(|(font_id, set)| {
            let mut glyphs = BTreeMap::new();
            for (gid, unicode) in set {
                glyphs.insert(gid, GlyphInfo { unicode });
            }
            FontUsage { font_id, glyphs }
        })
        .collect();
    // Sort by font name string for deterministic ordering across runs.
    // HashMap iteration order varies per process due to random hash seeds.
    usages.sort_by_key(|u| u.font_id.as_str().to_string());
    usages
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
        let subsetted = subsetter::subset(raw, 0, &remapper)
            .map_err(|e| format!("Subset error for {:?}: {e}", usage.font_id))?;

        // Compress the subset.
        let compressed =
            miniz_oxide::deflate::compress_to_vec_zlib(&subsetted, 6);

        // Read font metrics via skrifa.
        let sf = SfFontRef::new(raw).map_err(|e| format!("skrifa error: {e}"))?;
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
