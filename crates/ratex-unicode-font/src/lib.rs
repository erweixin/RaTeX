//! Discover a system Unicode font for fallback rendering of glyphs not present in KaTeX fonts.
//!
//! Checks (in order):
//! 1. `RATEX_UNICODE_FONT` environment variable
//! 2. Hard-coded system paths (Linux, macOS, Windows)
//! 3. `fontdb` system font database (SansSerif query, then brute-force)
//!
//! The result is cached in a `OnceLock` so it is computed at most once per process.

use std::sync::OnceLock;

static UNICODE_FONT: OnceLock<Option<Vec<u8>>> = OnceLock::new();

/// Raw TTF/OTF bytes of a discovered Unicode font, or `None` if no suitable font was found.
///
/// The result is cached after the first call.
pub fn load_unicode_font() -> Option<&'static [u8]> {
    UNICODE_FONT
        .get_or_init(load_unicode_fallback_font)
        .as_ref()
        .map(|v| v.as_slice())
}

fn is_valid_font(bytes: &[u8]) -> bool {
    // Validate SFNT font magic numbers:
    //   0x00010000 / 0x00000100 — TrueType / OpenType with TrueType outlines (.ttf)
    //   0x4F54544F ("OTTO")     — OpenType with CFF outlines (.otf)
    //   0x74727565 ("true")     — Apple TrueType (old Mac format)
    bytes.len() >= 4
        && (bytes[..4] == [0x00, 0x01, 0x00, 0x00]
            || bytes[..4] == [0x4F, 0x54, 0x54, 0x4F]
            || bytes[..4] == [0x74, 0x72, 0x75, 0x65])
}

fn load_unicode_fallback_font() -> Option<Vec<u8>> {
    // 1. User-specified font via RATEX_UNICODE_FONT
    if let Ok(p) = std::env::var("RATEX_UNICODE_FONT") {
        if let Ok(bytes) = std::fs::read(std::path::Path::new(&p)) {
            if is_valid_font(&bytes) {
                return Some(bytes);
            }
        }
    }

    // 2. Typical system paths (TTF and OTF)
    #[rustfmt::skip]
    let candidates: &[&str] = &[
        // Linux TTF
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSans-Regular.ttf",
        "/usr/share/fonts/opentype/noto/NotoSans-Regular.otf",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.otf",
        // macOS TTF / OTF
        "/Library/Fonts/Arial.ttf",
        "/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        // Windows TTF
        "C:\\Windows\\Fonts\\arial.ttf",
        "C:\\Windows\\Fonts\\segoeui.ttf",
    ];

    for path in candidates {
        if let Ok(bytes) = std::fs::read(std::path::Path::new(path)) {
            if is_valid_font(&bytes) {
                return Some(bytes);
            }
        }
    }

    // 3. fontdb SansSerif query
    let mut db = fontdb::Database::new();
    db.load_system_fonts();

    let query = fontdb::Query {
        families: &[fontdb::Family::SansSerif],
        weight: fontdb::Weight::NORMAL,
        stretch: fontdb::Stretch::Normal,
        style: fontdb::Style::Normal,
    };

    if let Some(id) = db.query(&query) {
        if let Some(bytes) = db.with_face_data(id, |data, _| data.to_vec()) {
            if is_valid_font(&bytes) {
                return Some(bytes);
            }
        }
    }

    // 4. Brute-force fontdb scan
    for face in db.faces() {
        if let Some(bytes) = db.with_face_data(face.id, |data, _| data.to_vec()) {
            if is_valid_font(&bytes) {
                return Some(bytes);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::is_valid_font;

    #[test]
    fn valid_truetype_magic() {
        assert!(is_valid_font(&[0x00, 0x01, 0x00, 0x00]));
    }

    #[test]
    fn valid_otto_magic() {
        assert!(is_valid_font(&[0x4F, 0x54, 0x54, 0x4F]));
    }

    #[test]
    fn valid_apple_truetype_magic() {
        assert!(is_valid_font(&[0x74, 0x72, 0x75, 0x65]));
    }

    #[test]
    fn invalid_empty_slice() {
        assert!(!is_valid_font(&[]));
    }

    #[test]
    fn invalid_wrong_magic() {
        assert!(!is_valid_font(b"ABCD"));
    }

    #[test]
    fn invalid_woff_magic() {
        assert!(!is_valid_font(b"wOFF"));
    }

    #[test]
    fn invalid_too_short() {
        assert!(!is_valid_font(&[0x00, 0x01, 0x00]));
    }
}
