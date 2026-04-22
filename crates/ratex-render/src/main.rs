use std::io::{self, BufRead};
use std::path::PathBuf;

use ratex_layout::{layout, LayoutOptions};
use ratex_layout::to_display_list;
use ratex_parser::parser::parse;
use ratex_render::{render_to_png, RenderOptions};
use ratex_types::color::Color;
use ratex_types::math_style::MathStyle;

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

    let font_size = args
        .iter()
        .position(|a| a == "--font-size")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<f32>().ok())
        .unwrap_or(40.0);

    let color = args
        .iter()
        .position(|a| a == "--color")
        .and_then(|i| args.get(i + 1))
        .map(|value| parse_color_arg(value))
        .transpose()
        .unwrap_or_else(|msg| {
            eprintln!("ERR {}", msg);
            std::process::exit(2);
        })
        .unwrap_or(Color::BLACK);

    std::fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    let options = RenderOptions {
        font_size,
        padding: 10.0,
        font_dir,
        device_pixel_ratio,
    };

    let inline = args.contains(&"--inline".to_string());
    let style = if inline { MathStyle::Text } else { MathStyle::Display };
    let layout_opts = LayoutOptions::default().with_style(style).with_color(color);

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
    // If this file exists, the directory is a full KaTeX `.ttf` tree (incl. KaTeX_Fraktur-Bold.ttf).
    const MARKER: &str = "KaTeX_Main-Regular.ttf";
    let candidates = [
        "fonts",
        "../fonts",
        "../../fonts",
        "../../../fonts",
        "tools/lexer_compare/node_modules/katex/dist/fonts",
        "../tools/lexer_compare/node_modules/katex/dist/fonts",
        "../../tools/lexer_compare/node_modules/katex/dist/fonts",
    ];
    for c in &candidates {
        let p = std::path::Path::new(c);
        if p.join(MARKER).is_file() {
            return c.to_string();
        }
    }
    "fonts".to_string()
}

fn parse_color_arg(value: &str) -> Result<Color, String> {
    Color::parse(value).ok_or_else(|| {
        format!(
            "invalid --color '{}': expected a named color, #rgb, #rrggbb, or [MODEL]value",
            value
        )
    })
}
