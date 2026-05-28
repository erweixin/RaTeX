//! Smoke: LaTeX → layout → [`ratex_pdf::render_to_pdf`], using workspace `fonts/` KaTeX TTFs.

use std::path::Path;

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parser::parse;
use ratex_pdf::{render_to_pdf, PdfOptions};
use ratex_types::display_item::DisplayList;
use ratex_types::math_style::MathStyle;

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
fn missing_font_dir_returns_font_error() {
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
    let err = render_to_pdf(&list, &opts).expect_err("bad font_dir must fail");
    assert!(
        err.to_string().contains("Missing required font"),
        "unexpected error: {err}"
    );
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
