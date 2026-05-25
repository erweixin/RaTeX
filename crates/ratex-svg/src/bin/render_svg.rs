//! Batch-export golden cases to standalone SVG (path glyphs, same scale as `ratex-render` + DPR).

use std::fs::File;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parser::parse;
use ratex_svg::{render_to_svg, SvgOptions};
use ratex_types::color::Color;
use ratex_types::math_style::MathStyle;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!(
            "{}",
            help_text(args.first().map(String::as_str).unwrap_or("render-svg"))
        );
        return;
    }

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
        .unwrap_or_else(|| "output_svg".to_string());

    let stdout = args.iter().any(|a| a == "--stdout");

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
        .and_then(|s| s.parse::<f64>().ok())
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

    let input_file = args
        .iter()
        .position(|a| a == "--input")
        .and_then(|i| args.get(i + 1))
        .cloned();

    if !stdout {
        std::fs::create_dir_all(&output_dir).expect("Failed to create output dir");
    }

    let dpr = device_pixel_ratio.clamp(0.01, 16.0) as f64;
    let svg_opts = SvgOptions {
        font_size: font_size * dpr,
        padding: 10.0 * dpr,
        stroke_width: 1.5 * dpr,
        embed_glyphs: true,
        font_dir,
    };

    let inline = args.contains(&"--inline".to_string());
    let style = if inline { MathStyle::Text } else { MathStyle::Display };
    let layout_opts = LayoutOptions::default().with_style(style).with_color(color);

    let mut idx = 0;
    let reader: Box<dyn BufRead> = match input_file {
        Some(path) => Box::new(io::BufReader::new(
            File::open(&path).unwrap_or_else(|e| panic!("Failed to open input file '{}': {}", path, e)),
        )),
        None => Box::new(io::BufReader::new(io::stdin())),
    };
    for line in reader.lines() {
        let line = line.expect("Failed to read line");
        let expr = line.trim();
        if expr.is_empty() || expr.starts_with('#') || expr.starts_with('%') {
            continue;
        }

        idx += 1;
        match svg_formula(expr, &layout_opts, &svg_opts) {
            Ok(svg) => {
                if stdout {
                    let mut handle = io::stdout().lock();
                    handle
                        .write_all(svg.as_bytes())
                        .expect("Failed to write SVG to stdout");
                    handle
                        .write_all(b"\n")
                        .expect("Failed to write SVG to stdout");
                    eprintln!("OK  {:4} {}", idx, expr);
                } else {
                    let path = PathBuf::from(&output_dir).join(format!("{:04}.svg", idx));
                    std::fs::write(&path, svg.as_bytes()).expect("Failed to write SVG");
                    println!("OK  {:4} {}", idx, expr);
                }
            }
            Err(e) => {
                eprintln!("ERR {:4} {} — {}", idx, expr, e);
            }
        }
    }

    if stdout {
        eprintln!("\nWrote {} SVG(s) to stdout", idx);
    } else {
        println!("\nWrote {} SVG(s) to {}/", idx, output_dir);
    }
}

fn svg_formula(
    expr: &str,
    layout_opts: &LayoutOptions,
    svg_opts: &SvgOptions,
) -> Result<String, String> {
    let ast = parse(expr).map_err(|e| format!("Parse error: {e}"))?;
    let lbox = layout(&ast, layout_opts);
    let display_list = to_display_list(&lbox);
    Ok(render_to_svg(&display_list, svg_opts))
}

fn default_font_dir() -> String {
    const MARKER: &str = "KaTeX_Main-Regular.ttf";
    let candidates = [
        "fonts",
        "../fonts",
        "../../fonts",
        "../../../fonts",
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

fn help_text(program: &str) -> String {
    let font_mode = if cfg!(feature = "embed-fonts") {
        "This binary is currently built with embedded fonts."
    } else {
        "This binary is currently built without embedded fonts."
    };
    let font_dir_option = if cfg!(feature = "embed-fonts") {
        String::new()
    } else {
        "  --font-dir <DIR>           Directory containing KaTeX font files for outlined glyphs\n"
            .to_string()
    };
    format!(
        "\
Usage: {program} [OPTIONS]

Read formulas from --input <FILE> or stdin, one per line.
Skip empty lines and lines starting with '#' or '%'.
{font_mode}

Options:
  -h, --help                 Show this help message
  --input <FILE>             Read formulas from file instead of stdin
{font_dir_option}  --output-dir <DIR>         Write SVGs to this directory [default: output_svg]
  --stdout                   Write SVGs to stdout instead of files
  --dpr <FACTOR>             Scale font size, padding, and stroke width [default: 1.0]
  --font-size <SIZE>         Base SVG font size in user units [default: 40.0]
  --color <COLOR>            Formula color: named, #rgb, #rrggbb, or [MODEL]value
                             [default: black]
  --inline                   Use inline math style instead of display style
"
    )
}
