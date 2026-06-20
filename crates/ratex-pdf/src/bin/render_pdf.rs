//! Batch-export golden cases to PDF using pdf-writer.

use std::fs::File;
use std::io::{self, BufRead};
use std::path::PathBuf;

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parser::parse;
use ratex_pdf::{render_to_pdf, PdfOptions};
use ratex_types::math_style::MathStyle;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        print!(
            "{}",
            help_text(args.first().map(String::as_str).unwrap_or("render-pdf"))
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
        .unwrap_or_else(|| "output_pdf".to_string());

    let device_pixel_ratio = args
        .iter()
        .position(|a| a == "--dpr")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(1.0);

    let font_size = args
        .iter()
        .position(|a| a == "--font-size")
        .and_then(|i| args.get(i + 1))
        .and_then(|s| s.parse::<f64>().ok())
        .unwrap_or(40.0);

    let input_file = args
        .iter()
        .position(|a| a == "--input")
        .and_then(|i| args.get(i + 1))
        .cloned();

    std::fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    let dpr = device_pixel_ratio.clamp(0.01, 16.0);
    let pdf_opts = PdfOptions {
        font_size: font_size * dpr,
        padding: 10.0 * dpr,
        stroke_width: 1.5 * dpr,
        font_dir,
    };

    let inline = args.iter().any(|a| a == "--inline");
    let style = if inline {
        MathStyle::Text
    } else {
        MathStyle::Display
    };
    let layout_opts = LayoutOptions::default().with_style(style);

    let mut idx = 0;
    let mut ok_count = 0;
    let reader: Box<dyn BufRead> = match input_file {
        Some(path) => {
            Box::new(io::BufReader::new(File::open(&path).unwrap_or_else(|e| {
                panic!("Failed to open input file '{}': {}", path, e)
            })))
        }
        None => Box::new(io::BufReader::new(io::stdin())),
    };
    for line in reader.lines() {
        let line = line.expect("Failed to read line");
        let expr = line.trim();
        if expr.is_empty() || expr.starts_with('#') || expr.starts_with('%') {
            continue;
        }

        idx += 1;
        match pdf_formula(expr, &layout_opts, &pdf_opts) {
            Ok(bytes) => {
                let path = PathBuf::from(&output_dir).join(format!("{:04}.pdf", idx));
                std::fs::write(&path, &bytes).expect("Failed to write PDF");
                ok_count += 1;
                println!("OK  {:4} {}", idx, expr);
            }
            Err(e) => {
                eprintln!("ERR {:4} {} — {}", idx, expr, e);
            }
        }
    }

    println!(
        "\nProcessed {} formula(s), wrote {} PDF(s) to {}/",
        idx, ok_count, output_dir
    );
}

fn pdf_formula(
    expr: &str,
    layout_opts: &LayoutOptions,
    pdf_opts: &PdfOptions,
) -> Result<Vec<u8>, String> {
    let ast = parse(expr).map_err(|e| format!("Parse error: {e}"))?;
    let lbox = layout(&ast, layout_opts);
    let display_list = to_display_list(&lbox);
    render_to_pdf(&display_list, pdf_opts).map_err(|e| format!("{e}"))
}

fn default_font_dir() -> String {
    const MARKER: &str = "KaTeX_Main-Regular.ttf";
    let candidates = ["fonts", "../fonts", "../../fonts", "../../../fonts"];
    for c in &candidates {
        let p = std::path::Path::new(c);
        if p.join(MARKER).is_file() {
            return c.to_string();
        }
    }
    "fonts".to_string()
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
        "  --font-dir <DIR>         Directory containing KaTeX font files\n".to_string()
    };
    format!(
        "\
Usage: {program} [OPTIONS]

Read formulas from --input <FILE> or stdin, one per line.
Skip empty lines and lines starting with '#' or '%'.
{font_mode}

Options:
  -h, --help             Show this help message
  --input <FILE>         Read formulas from file instead of stdin
{font_dir_option}  --output-dir <DIR>     Write PDFs to this directory [default: output_pdf]
  --dpr <FACTOR>         Scale font size, padding, and stroke width [default: 1.0]
  --font-size <SIZE>     Base PDF font size in user units [default: 40.0]
  --inline               Use inline math style instead of display style
"
    )
}
