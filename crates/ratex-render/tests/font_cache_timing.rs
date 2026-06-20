// Quick timing: measure PNG render cold (disk I/O) vs hot (OnceLock cache).
// Run: cargo test --package ratex-render --test font_cache_timing -- --nocapture

use std::time::Instant;

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parser::parse;
use ratex_render::{render_to_png, RenderOptions};

#[test]
#[ignore = "run manually: cargo test -p ratex-render --test font_cache_timing -- --ignored --nocapture"]
fn font_cache_speedup() {
    let formulas = [
        r"x^2 + y^2 = z^2",
        r"\frac{a}{b} + \int_0^\infty e^{-x} dx + \sum_{i=1}^n i",
        r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}",
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

    // Parse all formulas
    let dls: Vec<_> = formulas
        .iter()
        .map(|f| {
            let ast = parse(f).expect("parse");
            let layout = layout(&ast, &layout_opts);
            to_display_list(&layout)
        })
        .collect();

    println!("\n=========== Font Cache Timing ===========\n");

    // Cold: first render loads fonts from disk
    let t0 = Instant::now();
    for dl in &dls {
        render_to_png(dl, &opts).expect("render cold");
    }
    let cold_us = t0.elapsed().as_micros();
    let cold_per = cold_us / dls.len() as u128;

    println!(
        "COLD (3 formulas, first load): {} μs total, {:.1} ms avg/formula",
        cold_us,
        cold_per as f64 / 1000.0
    );

    // Hot: subsequent renders use OnceLock cache (no disk I/O)
    let t1 = Instant::now();
    for dl in &dls {
        render_to_png(dl, &opts).expect("render hot");
    }
    let hot_us = t1.elapsed().as_micros();
    let hot_per = hot_us / dls.len() as u128;

    println!(
        "HOT  (3 formulas, cached):   {} μs total, {:.1} ms avg/formula",
        hot_us,
        hot_per as f64 / 1000.0
    );

    if cold_per > 0 {
        let speedup = cold_per as f64 / hot_per.max(1) as f64;
        println!("\nSpeedup: {:.1}x", speedup);
    }

    // Also measure individual hot render
    let dl = &dls[0];
    let n = 5;
    let t2 = Instant::now();
    for _ in 0..n {
        render_to_png(dl, &opts).expect("render");
    }
    let hot_individual_us = t2.elapsed().as_micros() / n;
    println!(
        "HOT single formula ({n}× avg): {hot_individual_us} μs ({:.1} ms)",
        hot_individual_us as f64 / 1000.0
    );

    println!("\n========================================\n");
}
