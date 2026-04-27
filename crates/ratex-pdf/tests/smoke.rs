//! Smoke: LaTeX → layout → [`ratex_pdf::render_to_pdf`], using workspace `fonts/` KaTeX TTFs.

use std::path::Path;

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parser::parse;
use ratex_pdf::{render_to_pdf, PdfOptions};
use ratex_types::math_style::MathStyle;

fn katex_font_dir() -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fonts")
        .canonicalize()
        .expect("expected ../../fonts from crates/ratex-pdf (repo KaTeX TTFs)")
        .to_string_lossy()
        .into_owned()
}

fn latex_to_pdf(latex: &str) -> Vec<u8> {
    let nodes = parse(latex).expect("parse LaTeX");
    let lbox = layout(
        &nodes,
        &LayoutOptions::default().with_style(MathStyle::Display),
    );
    let list = to_display_list(&lbox);
    let opts = PdfOptions {
        font_dir: katex_font_dir(),
        ..Default::default()
    };
    render_to_pdf(&list, &opts).expect("render_to_pdf")
}

#[test]
fn smoke_fraction_renders_valid_pdf() {
    let pdf = latex_to_pdf(r"\frac{1}{2}");
    assert!(
        pdf.starts_with(b"%PDF-"),
        "expected %PDF- header, got {:?}",
        pdf.get(..12).map(|s| std::str::from_utf8(s).unwrap_or("<binary>"))
    );
    assert!(
        pdf.len() > 256,
        "PDF unexpectedly small: {} bytes",
        pdf.len()
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
