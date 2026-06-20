use std::path::PathBuf;

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parser::parse;
use ratex_render::{render_to_png, RenderOptions};
use ratex_types::color::Color;
use ratex_types::display_item::{DisplayItem, DisplayList};
use ratex_types::path_command::PathCommand;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn font_dir() -> String {
    project_root()
        .join("tools/lexer_compare/node_modules/katex/dist/fonts")
        .to_string_lossy()
        .to_string()
}

fn render_sample(background_color: Color) -> Option<Vec<u8>> {
    render_formula(Color::BLACK, background_color)
}

fn render_formula(foreground_color: Color, background_color: Color) -> Option<Vec<u8>> {
    let layout_options = LayoutOptions {
        color: foreground_color,
        ..LayoutOptions::default()
    };
    render_latex_with_options("x", layout_options, background_color)
}

fn render_latex(latex: &str, background_color: Color) -> Option<Vec<u8>> {
    render_latex_with_options(latex, LayoutOptions::default(), background_color)
}

fn render_latex_with_options(
    latex: &str,
    layout_options: LayoutOptions,
    background_color: Color,
) -> Option<Vec<u8>> {
    let font_dir = font_dir();
    if !std::path::Path::new(&font_dir).exists() {
        return None;
    }

    let ast = parse(latex).expect("parse sample formula");
    let layout = layout(&ast, &layout_options);
    let display_list = to_display_list(&layout);

    Some(
        render_to_png(
            &display_list,
            &RenderOptions {
                font_size: 40.0,
                padding: 8.0,
                background_color,
                font_dir,
                device_pixel_ratio: 1.0,
            },
        )
        .expect("render PNG"),
    )
}

fn render_display_list(display_list: DisplayList) -> Option<Vec<u8>> {
    let font_dir = font_dir();
    if !std::path::Path::new(&font_dir).exists() {
        return None;
    }

    Some(
        render_to_png(
            &display_list,
            &RenderOptions {
                font_size: 10.0,
                padding: 0.0,
                background_color: Color::new(0.0, 0.0, 0.0, 0.0),
                font_dir,
                device_pixel_ratio: 1.0,
            },
        )
        .expect("render PNG"),
    )
}

fn first_pixel_rgba(png_bytes: &[u8]) -> [u8; 4] {
    let buf = decode_png_rgba(png_bytes);
    [buf[0], buf[1], buf[2], buf[3]]
}

fn max_alpha(png_bytes: &[u8]) -> u8 {
    decode_png_rgba(png_bytes)
        .chunks_exact(4)
        .map(|rgba| rgba[3])
        .max()
        .unwrap_or(0)
}

fn max_alpha_pixel(png_bytes: &[u8]) -> [u8; 4] {
    let buf = decode_png_rgba(png_bytes);
    let pixel = buf
        .chunks_exact(4)
        .max_by_key(|rgba| rgba[3])
        .unwrap_or(&[0, 0, 0, 0]);
    [pixel[0], pixel[1], pixel[2], pixel[3]]
}

fn decode_png_rgba(png_bytes: &[u8]) -> Vec<u8> {
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder.read_info().expect("decode PNG info");
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("decode PNG frame");
    buf.truncate(info.buffer_size());
    buf
}

#[test]
fn render_to_png_uses_transparent_background() {
    let Some(png) = render_sample(Color::new(0.0, 0.0, 0.0, 0.0)) else {
        eprintln!("SKIP transparent_background: KaTeX font_dir missing");
        return;
    };
    assert_eq!(first_pixel_rgba(&png), [0, 0, 0, 0]);
}

#[test]
fn render_to_png_keeps_opaque_background_by_default() {
    let Some(png) = render_sample(Color::WHITE) else {
        eprintln!("SKIP transparent_background: KaTeX font_dir missing");
        return;
    };
    assert_eq!(first_pixel_rgba(&png), [255, 255, 255, 255]);
}

#[test]
fn render_to_png_preserves_rect_alpha() {
    let Some(png) = render_display_list(DisplayList {
        items: vec![DisplayItem::Rect {
            x: 0.0,
            y: 0.0,
            width: 1.0,
            height: 1.0,
            color: Color::new(1.0, 0.0, 0.0, 0.5),
        }],
        width: 1.0,
        height: 1.0,
        depth: 0.0,
    }) else {
        eprintln!("SKIP transparent_background: KaTeX font_dir missing");
        return;
    };
    assert_eq!(max_alpha_pixel(&png), [255, 0, 0, 128]);
}

#[test]
fn render_to_png_preserves_line_alpha() {
    let Some(png) = render_display_list(DisplayList {
        items: vec![DisplayItem::Line {
            x: 0.0,
            y: 0.5,
            width: 1.0,
            thickness: 0.2,
            color: Color::new(0.0, 1.0, 0.0, 0.5),
            dashed: false,
        }],
        width: 1.0,
        height: 1.0,
        depth: 0.0,
    }) else {
        eprintln!("SKIP transparent_background: KaTeX font_dir missing");
        return;
    };
    assert_eq!(max_alpha(&png), 128);
}

#[test]
fn render_to_png_preserves_path_alpha() {
    let Some(png) = render_display_list(DisplayList {
        items: vec![DisplayItem::Path {
            x: 0.0,
            y: 0.0,
            commands: vec![
                PathCommand::MoveTo { x: 0.0, y: 0.0 },
                PathCommand::LineTo { x: 1.0, y: 0.0 },
                PathCommand::LineTo { x: 1.0, y: 1.0 },
                PathCommand::LineTo { x: 0.0, y: 1.0 },
                PathCommand::Close,
            ],
            fill: true,
            color: Color::new(0.0, 0.0, 1.0, 0.5),
        }],
        width: 1.0,
        height: 1.0,
        depth: 0.0,
    }) else {
        eprintln!("SKIP transparent_background: KaTeX font_dir missing");
        return;
    };
    assert_eq!(max_alpha(&png), 128);
}

#[test]
fn render_to_png_preserves_glyph_alpha() {
    let Some(png) = render_formula(
        Color::new(1.0, 0.0, 0.0, 0.5),
        Color::new(0.0, 0.0, 0.0, 0.0),
    ) else {
        eprintln!("SKIP transparent_background: KaTeX font_dir missing");
        return;
    };
    let alpha = max_alpha(&png);
    assert!(alpha > 0);
    assert!(alpha <= 128);
}

#[test]
fn render_to_png_preserves_emoji_raster_alpha() {
    let ch = '😀';
    if ratex_unicode_font::emoji_png_raster_for_char(ch, 10.0).is_none() {
        eprintln!("SKIP transparent_background: PNG emoji raster missing");
        return;
    }

    let Some(png) = render_display_list(DisplayList {
        items: vec![DisplayItem::GlyphPath {
            x: 0.0,
            y: 1.0,
            scale: 1.0,
            font: "Emoji-Fallback".to_string(),
            char_code: ch as u32,
            color: Color::new(1.0, 0.0, 0.0, 0.5),
        }],
        width: 1.2,
        height: 2.0,
        depth: 0.0,
    }) else {
        eprintln!("SKIP transparent_background: KaTeX font_dir missing");
        return;
    };

    let alpha = max_alpha(&png);
    assert!(alpha > 0, "expected emoji raster ink");
    assert!(alpha <= 128, "expected emoji alpha <= 128, got {alpha}");
}

#[test]
fn render_to_png_preserves_textcolor_hex_alpha() {
    let Some(png) = render_latex(r"\textcolor{#ff000010}{x}", Color::new(0.0, 0.0, 0.0, 0.0))
    else {
        eprintln!("SKIP transparent_background: KaTeX font_dir missing");
        return;
    };
    let pixel = max_alpha_pixel(&png);
    assert_eq!(pixel[0], 255);
    assert_eq!(pixel[1], 0);
    assert_eq!(pixel[2], 0);
    assert!(pixel[3] > 0);
    assert!(pixel[3] <= 16);
}
