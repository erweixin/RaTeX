// Detailed rendering phase breakdown.
// Run: cargo test --package ratex-render --test phase_breakdown --release -- --nocapture

use std::time::Instant;

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parser::parse;
use ratex_render::{render_to_png, RenderOptions};

#[test]
#[ignore = "run manually: cargo test -p ratex-render --test phase_breakdown --release -- --ignored --nocapture"]
fn phase_breakdown() {
    let formulas: &[(&str, bool)] = &[
        ("x^2 + y^2 = z^2", false),
        ("a+b=c", false),
        (
            r"\frac{a}{b} + \int_0^\infty e^{-x} dx + \sum_{i=1}^n i \cdot \sqrt{x}",
            false,
        ),
        (
            r"\begin{pmatrix} a & b \\ c & d \end{pmatrix} \cdot \begin{pmatrix} e & f \\ g & h \end{pmatrix}",
            false,
        ),
        // CJK
        (r"\text{你好世界}\quad x^2 + y^2 = z^2", true),
        // emoji
        (r"\text{😊}\quad x^2 + y^2 = z^2 \quad \text{✅}", true),
    ];

    let font_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fonts");
    let font_dir = font_dir.to_string_lossy().to_string();

    let layout_opts = LayoutOptions::default();

    let opts = RenderOptions {
        font_size: 40.0,
        padding: 10.0,
        background_color: ratex_types::color::Color::WHITE,
        font_dir,
        device_pixel_ratio: 1.0,
    };

    println!("\n============ Render Phase Breakdown (release) ============\n");
    println!(
        "{:<40} {:>6} {:>12} {:>10} {:>10}",
        "Formula", "Glyphs", "Parse+Layout", "Render", "Total"
    );
    println!("{:-<40} {:-<6} {:-<12} {:-<10} {:-<10}", "", "", "", "", "");

    for &(expr, _has_cjk_emoji) in formulas {
        const WARMUP: u32 = 3;
        const ITERS: u32 = 10;

        // Warmup
        for _ in 0..WARMUP {
            let ast = parse(expr).expect("parse");
            let l = layout(&ast, &layout_opts);
            let dl = to_display_list(&l);
            let _ = render_to_png(&dl, &opts);
        }

        let mut parse_us = 0u128;
        let mut render_us = 0u128;
        let mut glyph_count = 0usize;

        for _ in 0..ITERS {
            let t0 = Instant::now();
            let ast = parse(expr).expect("parse");
            let l = layout(&ast, &layout_opts);
            let dl = to_display_list(&l);
            parse_us += t0.elapsed().as_micros();

            glyph_count = dl
                .items
                .iter()
                .filter(|i| matches!(i, ratex_types::display_item::DisplayItem::GlyphPath { .. }))
                .count();

            let t1 = Instant::now();
            let _ = render_to_png(&dl, &opts);
            render_us += t1.elapsed().as_micros();
        }

        let parse_avg = parse_us / ITERS as u128;
        let render_avg = render_us / ITERS as u128;
        let total_avg = parse_avg + render_avg;

        // Truncate long formulas for display
        let label = if expr.len() > 38 {
            format!("{}…", &expr[..37])
        } else {
            expr.to_string()
        };

        println!(
            "{:<40} {:>6} {:>8} μs  {:>6} μs  {:>6} μs",
            label, glyph_count, parse_avg, render_avg, total_avg
        );
    }

    println!("\n============================================================\n");
}
