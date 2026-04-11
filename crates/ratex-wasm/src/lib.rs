//! RaTeX WASM bindings: parse LaTeX and return DisplayList as JSON for browser rendering.

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parse;
use ratex_types::display_item::{DisplayItem, DisplayList};
use ratex_types::path_command::PathCommand;
use wasm_bindgen::prelude::*;

#[derive(serde::Serialize)]
struct VersionedDisplayList<'a> {
    version: u32,
    #[serde(flatten)]
    display_list: &'a DisplayList,
}

/// Parse LaTeX string and return the display list as JSON.
/// The browser can deserialize this and draw with Canvas 2D (web-render).
///
/// # Errors
/// Returns a JS error string if parsing fails.
#[wasm_bindgen(js_name = "renderLatex")]
pub fn render_latex(latex: &str) -> Result<String, JsValue> {
    let nodes = parse(latex).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let options = LayoutOptions::default();
    let layout_box = layout(&nodes, &options);
    let mut display_list = to_display_list(&layout_box);
    // serde_json's default f64 serializer errors on NaN/Infinity. Walk the
    // tree once in place and clamp non-finite values to 0 so we can call
    // to_string directly without going through Value (which used to double
    // the work and triple the allocations).
    sanitize_display_list(&mut display_list);
    let versioned = VersionedDisplayList {
        version: 1,
        display_list: &display_list,
    };
    serde_json::to_string(&versioned).map_err(|e| JsValue::from_str(&e.to_string()))
}

fn sanitize_display_list(dl: &mut DisplayList) {
    sanitize_f64(&mut dl.width);
    sanitize_f64(&mut dl.height);
    sanitize_f64(&mut dl.depth);
    for item in &mut dl.items {
        sanitize_item(item);
    }
}

fn sanitize_item(item: &mut DisplayItem) {
    match item {
        DisplayItem::GlyphPath { x, y, scale, commands, .. } => {
            sanitize_f64(x);
            sanitize_f64(y);
            sanitize_f64(scale);
            for cmd in commands {
                sanitize_path_command(cmd);
            }
        }
        DisplayItem::Line { x, y, width, thickness, .. } => {
            sanitize_f64(x);
            sanitize_f64(y);
            sanitize_f64(width);
            sanitize_f64(thickness);
        }
        DisplayItem::Rect { x, y, width, height, .. } => {
            sanitize_f64(x);
            sanitize_f64(y);
            sanitize_f64(width);
            sanitize_f64(height);
        }
        DisplayItem::Path { x, y, commands, .. } => {
            sanitize_f64(x);
            sanitize_f64(y);
            for cmd in commands {
                sanitize_path_command(cmd);
            }
        }
    }
}

fn sanitize_path_command(cmd: &mut PathCommand) {
    match cmd {
        PathCommand::MoveTo { x, y } | PathCommand::LineTo { x, y } => {
            sanitize_f64(x);
            sanitize_f64(y);
        }
        PathCommand::CubicTo { x1, y1, x2, y2, x, y } => {
            sanitize_f64(x1);
            sanitize_f64(y1);
            sanitize_f64(x2);
            sanitize_f64(y2);
            sanitize_f64(x);
            sanitize_f64(y);
        }
        PathCommand::QuadTo { x1, y1, x, y } => {
            sanitize_f64(x1);
            sanitize_f64(y1);
            sanitize_f64(x);
            sanitize_f64(y);
        }
        PathCommand::Close => {}
    }
}

#[inline]
fn sanitize_f64(v: &mut f64) {
    if !v.is_finite() {
        *v = 0.0;
    }
}
