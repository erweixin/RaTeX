//! Extract color-emoji bitmap strikes (`sbix`, `CBDT`, …) for PNG/SVG/PDF parity with `ratex-render`.

use ttf_parser::{Face, RasterImageFormat};

/// PNG (or raw bitmap) payload and placement from a color font, aligned with `glyph_raster_image`.
#[derive(Debug, Clone)]
pub struct EmojiRasterStrike {
    /// `RasterImageFormat::PNG`: deflate-ready PNG bytes. Other formats: raw samples as in the font.
    pub format: RasterImageFormat,
    pub data: Vec<u8>,
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
    pub pixels_per_em: u16,
}

/// Look up a bitmap strike for `ch` in the discovered emoji face (e.g. Apple Color Emoji).
///
/// `glyph_em_px` is the requested em size in pixels (same convention as `ratex-render` pixmap).
pub fn emoji_raster_for_char(ch: char, glyph_em_px: f32) -> Option<EmojiRasterStrike> {
    let (bytes, idx) = super::load_emoji_font_with_index()?;
    let face = Face::parse(bytes, idx).ok()?;
    let gid = face.glyph_index(ch)?;
    let strike = glyph_em_px.round().clamp(8.0, 256.0) as u16;
    let img = face
        .glyph_raster_image(gid, strike)
        .or_else(|| face.glyph_raster_image(gid, u16::MAX))?;
    Some(EmojiRasterStrike {
        format: img.format,
        data: img.data.to_vec(),
        x: img.x,
        y: img.y,
        width: img.width,
        height: img.height,
        pixels_per_em: img.pixels_per_em.max(1),
    })
}

/// Like [`emoji_raster_for_char`], but only returns strikes we can embed as a PNG (`data:image/png`).
pub fn emoji_png_raster_for_char(ch: char, glyph_em_px: f32) -> Option<EmojiRasterStrike> {
    let s = emoji_raster_for_char(ch, glyph_em_px)?;
    matches!(s.format, RasterImageFormat::PNG).then_some(s)
}
