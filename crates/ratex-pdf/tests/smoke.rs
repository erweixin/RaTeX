//! Smoke: LaTeX → layout → [`ratex_pdf::render_to_pdf`], using workspace `fonts/` KaTeX TTFs.

use std::path::Path;

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parser::parse;
use ratex_pdf::{render_to_pdf, PdfOptions};
use ratex_types::color::Color;
use ratex_types::display_item::DisplayItem;
use ratex_types::display_item::DisplayList;
use ratex_types::math_style::MathStyle;
use ratex_types::path_command::PathCommand;

fn katex_font_dir() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fonts")
        .canonicalize()
        .expect("expected ../../fonts from crates/ratex-pdf (repo KaTeX TTFs)")
        .to_string_lossy()
        .into_owned()
}

fn latex_to_display_list(latex: &str) -> DisplayList {
    let nodes = parse(latex).expect("parse LaTeX");
    let lbox = layout(
        &nodes,
        &LayoutOptions::default().with_style(MathStyle::Display),
    );
    to_display_list(&lbox)
}

fn latex_to_pdf(latex: &str) -> Vec<u8> {
    let list = latex_to_display_list(latex);
    let opts = PdfOptions {
        font_dir: katex_font_dir(),
        ..Default::default()
    };
    render_to_pdf(&list, &opts).expect("render_to_pdf")
}

fn extract_media_box(pdf: &[u8]) -> [f64; 4] {
    let s = String::from_utf8_lossy(pdf);
    let marker = "/MediaBox [";
    let start = s.find(marker).expect("expected /MediaBox") + marker.len();
    let end = s[start..].find(']').expect("expected MediaBox close") + start;
    let parts: Vec<f64> = s[start..end]
        .split_whitespace()
        .map(|part| part.parse().expect("parse MediaBox number"))
        .collect();
    assert_eq!(parts.len(), 4, "expected four MediaBox values");
    [parts[0], parts[1], parts[2], parts[3]]
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 0.01,
        "expected {actual} to be close to {expected}"
    );
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn decoded_pdf_streams(pdf: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut pos = 0;
    while let Some(stream_pos) = find_bytes(&pdf[pos..], b"stream") {
        let mut data_start = pos + stream_pos + b"stream".len();
        if pdf.get(data_start) == Some(&b'\r') && pdf.get(data_start + 1) == Some(&b'\n') {
            data_start += 2;
        } else if pdf.get(data_start) == Some(&b'\n') {
            data_start += 1;
        }

        let Some(end_rel) = find_bytes(&pdf[data_start..], b"endstream") else {
            break;
        };
        let data_end = data_start + end_rel;
        if let Ok(decoded) =
            miniz_oxide::inflate::decompress_to_vec_zlib(&pdf[data_start..data_end])
        {
            out.push(String::from_utf8_lossy(&decoded).into_owned());
        }
        pos = data_end + b"endstream".len();
    }
    out
}

#[test]
fn smoke_fraction_renders_valid_pdf() {
    let pdf = latex_to_pdf(r"\frac{1}{2}");
    assert!(
        pdf.starts_with(b"%PDF-"),
        "expected %PDF- header, got {:?}",
        pdf.get(..12)
            .map(|s| std::str::from_utf8(s).unwrap_or("<binary>"))
    );
    assert!(
        pdf.len() > 256,
        "PDF unexpectedly small: {} bytes",
        pdf.len()
    );
}

#[test]
fn pdf_preserves_fill_and_stroke_alpha() {
    let list = DisplayList {
        width: 2.0,
        height: 1.0,
        depth: 0.0,
        items: vec![
            DisplayItem::Rect {
                x: 0.0,
                y: 0.0,
                width: 1.0,
                height: 1.0,
                color: Color::new(1.0, 0.0, 0.0, 0.5),
            },
            DisplayItem::Path {
                x: 0.0,
                y: 0.0,
                commands: vec![
                    PathCommand::MoveTo { x: 0.0, y: 0.0 },
                    PathCommand::LineTo { x: 1.0, y: 1.0 },
                ],
                fill: false,
                color: Color::new(0.0, 0.0, 1.0, 0.25),
            },
        ],
    };
    let opts = PdfOptions {
        font_dir: katex_font_dir(),
        ..Default::default()
    };
    let pdf = render_to_pdf(&list, &opts).expect("render_to_pdf");
    let pdf_text = String::from_utf8_lossy(&pdf);

    assert!(pdf_text.contains("/ExtGState"));
    assert!(pdf_text.contains("/GS500000"));
    assert!(pdf_text.contains("/GS250000"));
    assert!(pdf_text.contains("/ca 0.5"));
    assert!(pdf_text.contains("/CA 0.5"));
    assert!(pdf_text.contains("/ca 0.25"));
    assert!(pdf_text.contains("/CA 0.25"));

    let content = decoded_pdf_streams(&pdf).join("\n");
    assert!(content.contains("/GS500000 gs"), "{content}");
    assert!(content.contains("/GS250000 gs"), "{content}");
}

#[test]
fn pdf_preserves_textcolor_hex_alpha() {
    let pdf = latex_to_pdf(r"\textcolor{#ff000050}{x}");
    let pdf_text = String::from_utf8_lossy(&pdf);
    let alpha_key = ((80.0_f32 / 255.0) * 1_000_000.0).round() as u32;
    let state_name = format!("/GS{alpha_key}");

    assert!(pdf_text.contains("/ExtGState"));
    assert!(pdf_text.contains(&state_name));
    assert!(pdf_text.contains("/ca 0.313725"));
    assert!(pdf_text.contains("/CA 0.313725"));

    let content = decoded_pdf_streams(&pdf).join("\n");
    assert!(content.contains(&format!("{state_name} gs")), "{content}");
}

#[test]
fn pdf_preserves_emoji_raster_alpha() {
    let ch = '😀';
    #[cfg(target_os = "macos")]
    let glyph_em = 80.0;
    #[cfg(not(target_os = "macos"))]
    let glyph_em = 40.0;

    if ratex_unicode_font::emoji_png_raster_for_char(ch, glyph_em).is_none() {
        eprintln!("SKIP pdf alpha: PNG emoji raster missing");
        return;
    }

    let list = DisplayList {
        width: 1.2,
        height: 1.2,
        depth: 0.4,
        items: vec![DisplayItem::GlyphPath {
            x: 0.0,
            y: 0.8,
            scale: 1.0,
            font: "Emoji-Fallback".to_string(),
            char_code: ch as u32,
            color: Color::new(1.0, 0.0, 0.0, 0.5),
        }],
    };
    let opts = PdfOptions {
        font_dir: katex_font_dir(),
        ..Default::default()
    };
    let pdf = render_to_pdf(&list, &opts).expect("render_to_pdf");
    let content = decoded_pdf_streams(&pdf).join("\n");

    assert!(content.contains("/GS500000 gs"), "{content}");
    assert!(content.contains(" Do"), "{content}");
}

#[test]
fn pdf_omits_alpha_graphics_state_for_opaque_colors() {
    let list = DisplayList {
        width: 1.0,
        height: 1.0,
        depth: 0.0,
        items: vec![DisplayItem::Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            color: Color::new(1.0, 0.0, 0.0, 1.0),
        }],
    };
    let opts = PdfOptions {
        font_dir: katex_font_dir(),
        ..Default::default()
    };
    let pdf = render_to_pdf(&list, &opts).expect("render_to_pdf");
    let pdf_text = String::from_utf8_lossy(&pdf);

    assert!(!pdf_text.contains("/ExtGState"));
    assert!(!decoded_pdf_streams(&pdf).join("\n").contains(" gs"));
}

#[test]
fn zero_padding_pdf_media_box_keeps_vertical_antialias_guard() {
    let list = latex_to_display_list(r"x = \frac{-b \pm \sqrt{b^2-4ac}}{2a}");
    let opts = PdfOptions {
        font_size: 40.0,
        padding: 0.0,
        font_dir: katex_font_dir(),
        ..Default::default()
    };
    let pdf = render_to_pdf(&list, &opts).expect("render_to_pdf");
    let media_box = extract_media_box(&pdf);

    assert_close(media_box[0], 0.0);
    assert_close(media_box[1], 0.0);
    assert_close(media_box[2], list.width * opts.font_size);
    assert_close(
        media_box[3],
        (list.height + list.depth) * opts.font_size + 2.0,
    );
}

#[test]
#[cfg(not(feature = "embed-fonts"))]
fn missing_font_dir_returns_font_error_unless_fonts_are_unified() {
    let nodes = parse("x").expect("parse LaTeX");
    let lbox = layout(
        &nodes,
        &LayoutOptions::default().with_style(MathStyle::Display),
    );
    let list = to_display_list(&lbox);
    let opts = PdfOptions {
        font_dir: "/definitely/not/a/ratex/font/dir".to_string(),
        ..Default::default()
    };
    match render_to_pdf(&list, &opts) {
        Ok(pdf) => assert!(
            pdf.starts_with(b"%PDF-"),
            "embedded-font feature unification should still render a valid PDF"
        ),
        Err(err) => assert!(
            err.to_string().contains("Missing required font"),
            "unexpected error: {err}"
        ),
    }
}

/// Color emoji in PDF: `EmojiFallback` → image XObjects (sbix PNG), not empty outlines.
#[cfg(target_os = "macos")]
mod macos_emoji_pdf {
    use ratex_layout::to_display_list;
    use ratex_layout::{layout, LayoutOptions};
    use ratex_parser::parser::parse;
    use ratex_pdf::{render_to_pdf, PdfOptions};

    #[test]
    fn single_emoji_pdf_contains_image_xobject() {
        std::env::set_var(
            "RATEX_UNICODE_FONT",
            "/System/Library/Fonts/Supplemental/AppleGothic.ttf",
        );
        let ast = parse(r"\text{😀}").unwrap();
        let lbox = layout(&ast, &LayoutOptions::default());
        let dl = to_display_list(&lbox);
        let opts = PdfOptions {
            font_dir: concat!(env!("CARGO_MANIFEST_DIR"), "/../../fonts").to_string(),
            ..Default::default()
        };
        let pdf = render_to_pdf(&dl, &opts).expect("pdf");
        let s = String::from_utf8_lossy(&pdf);
        assert!(
            s.contains("/Subtype /Image"),
            "expected at least one image XObject for color emoji"
        );
        assert!(
            s.contains("/ImageC"),
            "expected /ProcSet to include ImageC so color image XObjects paint in strict viewers"
        );
        assert!(
            s.contains("/XObject") && s.contains("/E0"),
            "expected page Resources to map at least one emoji XObject (e.g. /E0)"
        );
    }
}
