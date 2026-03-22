use std::io::{self, BufRead};
use std::path::PathBuf;

use ratex_layout::{layout, LayoutOptions};
use ratex_layout::to_display_list;
use ratex_parser::parser::parse;
use ratex_render::{render_to_png, RenderOptions};

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let font_dir = args
        .iter()
        .position(|a| a == "--font-dir")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(default_font_dir);

    let output_dir = args
        .iter()
        .position(|a| a == "--output-dir")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "output".to_string());

    let device_pixel_ratio = args
        .iter()
        .position(|a| a == "--dpr")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(1.0);

    std::fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    let options = RenderOptions {
        font_size: 40.0,
        padding: 10.0,
        font_dir,
        device_pixel_ratio,
    };

    let layout_opts = LayoutOptions::default();

    let stdin = io::stdin();
    let mut idx = 0;
    for line in stdin.lock().lines() {
        let line = line.expect("Failed to read line");
        let expr = line.trim();
        if expr.is_empty() || expr.starts_with('#') {
            continue;
        }

        idx += 1;
        match render_formula(expr, &layout_opts, &options) {
            Ok(png_data) => {
                let path = PathBuf::from(&output_dir).join(format!("{:04}.png", idx));
                std::fs::write(&path, &png_data).expect("Failed to write PNG");
                println!("OK  {:4} {}", idx, expr);
            }
            Err(e) => {
                eprintln!("ERR {:4} {} — {}", idx, expr, e);
            }
        }
    }

    println!("\nRendered {} formulas to {}/", idx, output_dir);
}

fn render_formula(
    expr: &str,
    layout_opts: &LayoutOptions,
    render_opts: &RenderOptions,
) -> Result<Vec<u8>, String> {
    let ast = parse(expr).map_err(|e| format!("Parse error: {}", e))?;
    let lbox = layout(&ast, layout_opts);
    let display_list = to_display_list(&lbox);
    render_to_png(&display_list, render_opts)
}

fn default_font_dir() -> String {
    let candidates = [
        "tools/lexer_compare/node_modules/katex/dist/fonts",
        "../tools/lexer_compare/node_modules/katex/dist/fonts",
        "../../tools/lexer_compare/node_modules/katex/dist/fonts",
    ];
    for c in &candidates {
        if std::path::Path::new(c).exists() {
            return c.to_string();
        }
    }
    candidates[0].to_string()
}
