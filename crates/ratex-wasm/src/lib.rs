//! RaTeX WASM bindings: parse LaTeX and return DisplayList as JSON for browser rendering.

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parse;
use ratex_types::color::Color;
use ratex_types::display_item::{DisplayItem, DisplayList};
use ratex_types::math_style::MathStyle;
use ratex_types::path_command::PathCommand;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(typescript_type = "RenderLatexOptions")]
    pub type JsRenderLatexOptions;
}

#[wasm_bindgen(typescript_custom_section)]
const RENDER_LATEX_OPTIONS_TYPES: &str = r#"
export interface RenderColorRgba {
    r: number;
    g: number;
    b: number;
    a: number;
}

export type RenderColor = string | RenderColorRgba;

export interface RenderLatexOptions {
    /** `true` (default) for display/block style; `false` for inline/text style. */
    displayMode?: boolean;
    /** Default formula color as a supported color string or normalized RGBA components. */
    color?: RenderColor;
}
"#;

#[derive(serde::Serialize)]
struct VersionedDisplayList<'a> {
    version: u32,
    #[serde(flatten)]
    display_list: &'a DisplayList,
}

/// Parse LaTeX string and return the display list as JSON.
/// The browser can deserialize this and draw with Canvas 2D (web-render).
///
/// `displayMode` defaults to `true` (display/block style). Pass `false` to
/// use inline/text style.
///
/// # Errors
/// Returns a JS error string if parsing fails.
#[wasm_bindgen(js_name = "renderLatex")]
#[allow(non_snake_case)]
pub fn render_latex(
    latex: &str,
    color: Option<String>,
    displayMode: Option<bool>,
) -> Result<String, JsValue> {
    let color = color
        .as_deref()
        .map(parse_color_string)
        .transpose()
        .map_err(|error| JsValue::from_str(&error))?;
    render_latex_impl(latex, color, displayMode).map_err(|error| JsValue::from_str(&error))
}

/// Parse LaTeX using a forward-compatible options object.
///
/// This is the preferred API for new integrations. Existing `renderLatex`
/// callers remain supported. Unknown option fields are ignored.
#[wasm_bindgen(js_name = "renderLatexWithOptions")]
pub fn render_latex_with_options(
    latex: &str,
    options: Option<JsRenderLatexOptions>,
) -> Result<String, JsValue> {
    let (color, display_mode) =
        parse_render_options(options).map_err(|error| JsValue::from_str(&error))?;
    render_latex_impl(latex, color, display_mode).map_err(|error| JsValue::from_str(&error))
}

fn render_latex_impl(
    latex: &str,
    color: Option<Color>,
    display_mode: Option<bool>,
) -> Result<String, String> {
    let nodes = parse(latex).map_err(|e| e.to_string())?;
    let style = if display_mode.unwrap_or(true) {
        MathStyle::Display
    } else {
        MathStyle::Text
    };
    let options = LayoutOptions::default().with_style(style);
    let options = if let Some(color) = color {
        options.with_color(color)
    } else {
        options
    };
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
    serde_json::to_string(&versioned).map_err(|e| e.to_string())
}

fn parse_render_options(
    options: Option<JsRenderLatexOptions>,
) -> Result<(Option<Color>, Option<bool>), String> {
    let Some(options) = options else {
        return Ok((None, None));
    };
    let options: JsValue = options.into();
    if !options.is_object() {
        return Err("invalid options: expected an object".to_string());
    }

    let display_mode = match optional_property(&options, "displayMode")? {
        Some(value) => Some(
            value
                .as_bool()
                .ok_or_else(|| "invalid options.displayMode: expected a boolean".to_string())?,
        ),
        None => None,
    };

    let color = match optional_property(&options, "color")? {
        Some(value) => Some(parse_js_color(&value)?),
        None => None,
    };

    Ok((color, display_mode))
}

fn optional_property(target: &JsValue, name: &str) -> Result<Option<JsValue>, String> {
    let value = js_sys::Reflect::get(target, &JsValue::from_str(name))
        .map_err(|_| format!("failed to read options.{name}"))?;
    if value.is_null() || value.is_undefined() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

fn parse_js_color(value: &JsValue) -> Result<Color, String> {
    if let Some(color) = value.as_string() {
        return parse_color_string(&color);
    }
    if !value.is_object() {
        return Err(
            "invalid options.color: expected a color string or { r, g, b, a } object".to_string(),
        );
    }

    let r = required_color_component(value, "r")?;
    let g = required_color_component(value, "g")?;
    let b = required_color_component(value, "b")?;
    let a = required_color_component(value, "a")?;
    rgba_color(r, g, b, a)
}

fn required_color_component(value: &JsValue, name: &str) -> Result<f64, String> {
    optional_property(value, name)?
        .and_then(|component| component.as_f64())
        .ok_or_else(|| format!("invalid options.color.{name}: expected a number in [0, 1]"))
}

fn rgba_color(r: f64, g: f64, b: f64, a: f64) -> Result<Color, String> {
    for (name, value) in [("r", r), ("g", g), ("b", b), ("a", a)] {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(format!(
                "invalid options.color.{name}: expected a finite number in [0, 1], got {value}"
            ));
        }
    }
    Ok(Color::new(r as f32, g as f32, b as f32, a as f32))
}

fn parse_color_string(color: &str) -> Result<Color, String> {
    Color::parse(color).ok_or_else(|| {
        format!(
            "invalid color: '{}'. Expected a named color, #rgb, #rgba, #rrggbb, #rrggbbaa, or [MODEL]value",
            color
        )
    })
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
        DisplayItem::GlyphPath { x, y, scale, .. } => {
            sanitize_f64(x);
            sanitize_f64(y);
            sanitize_f64(scale);
        }
        DisplayItem::Line {
            x,
            y,
            width,
            thickness,
            ..
        } => {
            sanitize_f64(x);
            sanitize_f64(y);
            sanitize_f64(width);
            sanitize_f64(thickness);
        }
        DisplayItem::Rect {
            x,
            y,
            width,
            height,
            ..
        } => {
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
        PathCommand::CubicTo {
            x1,
            y1,
            x2,
            y2,
            x,
            y,
        } => {
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

#[cfg(test)]
mod tests {
    use super::{render_latex_impl, rgba_color};

    #[test]
    fn display_mode_defaults_to_display_and_supports_inline() {
        let latex = r"\frac{1}{2}";
        let default_display = render_latex_impl(latex, None, None).unwrap();
        let explicit_display = render_latex_impl(latex, None, Some(true)).unwrap();
        let inline = render_latex_impl(latex, None, Some(false)).unwrap();

        assert_eq!(default_display, explicit_display);
        assert_ne!(explicit_display, inline);
    }

    #[test]
    fn structured_rgba_matches_native_color_range() {
        let color = rgba_color(0.125, 0.25, 0.5, 0.75).unwrap();
        assert_eq!(
            (color.r, color.g, color.b, color.a),
            (0.125, 0.25, 0.5, 0.75)
        );

        assert!(rgba_color(-0.1, 0.0, 0.0, 1.0).is_err());
        assert!(rgba_color(0.0, 0.0, 0.0, f64::NAN).is_err());
    }
}
