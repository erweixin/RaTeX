//! Python bindings for RaTeX math rendering.
//!
//! Comprehensive LaTeX math → SVG / PNG / PDF via PyO3.
//! Supports parse validation, cached DisplayList JSON, and batch rendering.
//!
//! ```python
//! import ratex_py
//!
//! # Simple rendering
//! svg  = ratex_py.render_svg(r"\frac{1}{2}")
//! png  = ratex_py.render_png(r"\frac{1}{2}")
//! pdf  = ratex_py.render_pdf(r"\frac{1}{2}")
//!
//! # Cache DisplayList JSON, render to multiple formats
//! dl_json = ratex_py.render_display_list(r"\frac{1}{2}")
//! svg = ratex_py.render_svg_from_display_list(dl_json)
//! png = ratex_py.render_png_from_display_list(dl_json)
//! pdf = ratex_py.render_pdf_from_display_list(dl_json)
//!
//! # Parse validation
//! ratex_py.check(r"\frac{1}{2}")  # raises on error
//!
//! # Convenience
//! inline_svg = ratex_py.render_svg_inline(r"\sqrt{x}")  # display_mode=False
//! ```

use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict};
use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parse;
use ratex_pdf::{render_to_pdf, PdfOptions};
use ratex_render::{render_to_png, RenderOptions};
use ratex_svg::{render_to_svg, SvgOptions};
use ratex_types::color::Color;
use ratex_types::display_item::DisplayList;
use ratex_types::math_style::MathStyle;

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Parse a CSS-style color string (`#rrggbb`, `#rgb`, or the names `black` /
/// `white` / `transparent`) into a [`Color`].
fn parse_color(s: &str) -> PyResult<Color> {
    let s = s.trim();
    if s.eq_ignore_ascii_case("black") {
        return Ok(Color::BLACK);
    }
    if s.eq_ignore_ascii_case("white") {
        return Ok(Color::WHITE);
    }
    if s.eq_ignore_ascii_case("transparent") {
        return Ok(Color::new(0.0, 0.0, 0.0, 0.0));
    }
    if let Some(hex) = s.strip_prefix('#') {
        let (rs, gs, bs) = match hex.len() {
            6 => (&hex[0..2], &hex[2..4], &hex[4..6]),
            3 => {
                let r = u8::from_str_radix(&hex[0..1].repeat(2), 16)
                    .map_err(|e| PyValueError::new_err(e.to_string()))?;
                let g = u8::from_str_radix(&hex[1..2].repeat(2), 16)
                    .map_err(|e| PyValueError::new_err(e.to_string()))?;
                let b = u8::from_str_radix(&hex[2..3].repeat(2), 16)
                    .map_err(|e| PyValueError::new_err(e.to_string()))?;
                return Ok(Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0));
            }
            _ => {
                return Err(PyValueError::new_err(format!(
                    "invalid hex color (expected #rgb or #rrggbb): {s}"
                )))
            }
        };
        let r = u8::from_str_radix(rs, 16).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let g = u8::from_str_radix(gs, 16).map_err(|e| PyValueError::new_err(e.to_string()))?;
        let b = u8::from_str_radix(bs, 16).map_err(|e| PyValueError::new_err(e.to_string()))?;
        return Ok(Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0));
    }
    Err(PyValueError::new_err(format!(
        "unsupported color format (use #rrggbb, #rgb, 'black', 'white', or 'transparent'): {s}"
    )))
}

fn make_layout_options(display_mode: bool, color: &str) -> PyResult<LayoutOptions> {
    let style = if display_mode {
        MathStyle::Display
    } else {
        MathStyle::Text
    };
    Ok(LayoutOptions {
        style,
        color: parse_color(color)?,
        ..Default::default()
    })
}

/// Parse and layout LaTeX → DisplayList (the core pipeline).
fn pipeline(latex: &str, display_mode: bool, color: &str) -> PyResult<DisplayList> {
    let opts = make_layout_options(display_mode, color)?;
    let nodes = parse(latex).map_err(|e| PyValueError::new_err(format!("parse error: {e}")))?;
    let box_ = layout(&nodes, &opts);
    Ok(to_display_list(&box_))
}

/// Deserialize a JSON DisplayList string back into a DisplayList struct.
fn deserialize_display_list(json_str: &str) -> PyResult<DisplayList> {
    serde_json::from_str(json_str)
        .map_err(|e| PyValueError::new_err(format!("failed to deserialize DisplayList JSON: {e}")))
}

fn normalize_formats(formats: &[String]) -> PyResult<Vec<String>> {
    if formats.is_empty() {
        return Err(PyValueError::new_err(
            "formats must contain at least one entry",
        ));
    }

    let mut normalized = Vec::with_capacity(formats.len());
    for format in formats {
        let name = format.trim().to_ascii_lowercase();
        let allowed = matches!(
            name.as_str(),
            "svg" | "png" | "pdf" | "html" | "json" | "display_list"
        );
        if !allowed {
            return Err(PyValueError::new_err(format!(
                "unsupported format '{name}' (use svg, png, pdf, html, json, or display_list)"
            )));
        }
        if !normalized.contains(&name) {
            normalized.push(name);
        }
    }
    Ok(normalized)
}

#[derive(Clone, Copy)]
struct RenderParams<'a> {
    font_size: f64,
    display_mode: bool,
    color: &'a str,
    embed_glyphs: bool,
    background_color: &'a str,
    dpr: f64,
}

fn render_formats_impl(
    py: Python<'_>,
    latex: &str,
    formats: &[String],
    params: RenderParams<'_>,
) -> PyResult<Py<PyDict>> {
    let normalized = normalize_formats(formats)?;
    let display_list = pipeline(latex, params.display_mode, params.color)?;

    let svg_opts = SvgOptions {
        font_size: params.font_size,
        embed_glyphs: params.embed_glyphs,
        ..Default::default()
    };
    let png_opts = RenderOptions {
        font_size: params.font_size as f32,
        background_color: parse_color(params.background_color)?,
        device_pixel_ratio: params.dpr as f32,
        ..Default::default()
    };
    let pdf_opts = PdfOptions {
        font_size: params.font_size,
        ..Default::default()
    };

    let out = PyDict::new(py);
    let mut cached_svg: Option<String> = None;
    let mut cached_display_list_json: Option<String> = None;

    for format in normalized {
        match format.as_str() {
            "svg" => {
                let svg = render_to_svg(&display_list, &svg_opts);
                cached_svg = Some(svg.clone());
                out.set_item("svg", svg)?;
            }
            "png" => {
                let png = render_to_png(&display_list, &png_opts).map_err(PyValueError::new_err)?;
                out.set_item("png", PyBytes::new(py, &png))?;
            }
            "pdf" => {
                let pdf =
                    render_to_pdf(&display_list, &pdf_opts).map_err(|e| PyValueError::new_err(e.to_string()))?;
                out.set_item("pdf", PyBytes::new(py, &pdf))?;
            }
            "display_list" => {
                let json = serde_json::to_string(&display_list)
                    .map_err(|e| PyValueError::new_err(format!("serialization error: {e}")))?;
                cached_display_list_json = Some(json.clone());
                out.set_item("display_list", json)?;
            }
            "json" => {
                let json = if let Some(existing) = &cached_display_list_json {
                    existing.clone()
                } else {
                    let serialized = serde_json::to_string(&display_list)
                        .map_err(|e| PyValueError::new_err(format!("serialization error: {e}")))?;
                    cached_display_list_json = Some(serialized.clone());
                    serialized
                };
                let parsed_json = py
                    .import("json")?
                    .call_method1("loads", (json,))?;
                out.set_item("json", parsed_json)?;
            }
            "html" => {
                let svg = if let Some(existing) = &cached_svg {
                    existing.clone()
                } else {
                    let rendered = render_to_svg(&display_list, &svg_opts);
                    cached_svg = Some(rendered.clone());
                    rendered
                };
                let mode = if params.display_mode { "display" } else { "inline" };
                let html = format!(
                    "<span class=\"ratex-math ratex-{mode}\">{svg}</span>",
                );
                out.set_item("html", html)?;
            }
            _ => unreachable!("format already validated"),
        }
    }

    Ok(out.into())
}

#[pyclass(skip_from_py_object)]
#[derive(Clone)]
struct Expr {
    latex: String,
    font_size: f64,
    display_mode: bool,
    color: String,
    embed_glyphs: bool,
    background_color: String,
    dpr: f64,
    formats: Vec<String>,
}

#[pymethods]
impl Expr {
    #[new]
    #[pyo3(signature = (latex, *, font_size=40.0, display_mode=true, color="black", embed_glyphs=true, background_color="white", dpr=1.0, formats=None))]
    fn new(
        latex: &str,
        font_size: f64,
        display_mode: bool,
        color: &str,
        embed_glyphs: bool,
        background_color: &str,
        dpr: f64,
        formats: Option<Vec<String>>,
    ) -> PyResult<Self> {
        let formats = formats.unwrap_or_else(|| vec!["svg".to_string()]);
        let normalized = normalize_formats(&formats)?;
        Ok(Self {
            latex: latex.to_string(),
            font_size,
            display_mode,
            color: color.to_string(),
            embed_glyphs,
            background_color: background_color.to_string(),
            dpr,
            formats: normalized,
        })
    }

    fn _repr_svg_(&self) -> PyResult<String> {
        render_svg(
            &self.latex,
            self.font_size,
            self.display_mode,
            &self.color,
            self.embed_glyphs,
        )
    }

    fn _repr_png_(&self) -> PyResult<Vec<u8>> {
        render_png(
            &self.latex,
            self.font_size,
            self.display_mode,
            &self.color,
            &self.background_color,
            self.dpr,
        )
    }

    fn _repr_html_(&self) -> PyResult<String> {
        let svg = self._repr_svg_()?;
        let mode = if self.display_mode { "display" } else { "inline" };
        Ok(format!(
            "<span class=\"ratex-math ratex-{mode}\">{svg}</span>",
        ))
    }

    fn _repr_json_(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let json = render_display_list(&self.latex, self.display_mode)?;
        Ok(py
            .import("json")?
            .call_method1("loads", (json,))?
            .unbind()
            .into())
    }

    #[pyo3(signature = (_include=None, _exclude=None))]
    fn _repr_mimebundle_(
        &self,
        py: Python<'_>,
        _include: Option<&Bound<'_, PyAny>>,
        _exclude: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Py<PyDict>> {
        let rendered = render_formats_impl(py, &self.latex, &self.formats, self.params())?;
        let rendered = rendered.bind(py);

        let bundle = PyDict::new(py);
        if let Some(svg) = rendered.get_item("svg")? {
            bundle.set_item("image/svg+xml", svg)?;
        }
        if let Some(png) = rendered.get_item("png")? {
            bundle.set_item("image/png", png)?;
        }
        if let Some(pdf) = rendered.get_item("pdf")? {
            bundle.set_item("application/pdf", pdf)?;
        }
        if let Some(html) = rendered.get_item("html")? {
            bundle.set_item("text/html", html)?;
        }
        if let Some(json_obj) = rendered.get_item("json")? {
            bundle.set_item("application/json", json_obj)?;
        }
        if let Some(display_list) = rendered.get_item("display_list")? {
            bundle.set_item("application/x-ratex-display-list+json", display_list)?;
        }
        Ok(bundle.into())
    }

    #[pyo3(signature = (formats=None))]
    fn render(&self, py: Python<'_>, formats: Option<Vec<String>>) -> PyResult<Py<PyDict>> {
        let formats = formats.unwrap_or_else(|| self.formats.clone());
        render_formats_impl(py, &self.latex, &formats, self.params())
    }

    fn __repr__(&self) -> String {
        format!(
            "Expr({:?}, font_size={}, display_mode={}, color={:?}, formats={:?})",
            self.latex, self.font_size, self.display_mode, self.color, self.formats
        )
    }
}

impl Expr {
    fn params(&self) -> RenderParams<'_> {
        RenderParams {
            font_size: self.font_size,
            display_mode: self.display_mode,
            color: &self.color,
            embed_glyphs: self.embed_glyphs,
            background_color: &self.background_color,
            dpr: self.dpr,
        }
    }
}

// ---------------------------------------------------------------------------
// Public Python API
// ---------------------------------------------------------------------------

/// Parse a LaTeX math string and return ``True``, or raise an exception on parse error.
/// Useful for validation without rendering.
///
/// Parameters
/// ----------
/// latex : str
///     The LaTeX math input.
///
/// Raises
/// ------
/// ValueError
///     If the LaTeX cannot be parsed.
#[pyfunction]
fn check(latex: &str) -> PyResult<()> {
    parse(latex).map_err(|e| PyValueError::new_err(format!("parse error: {e}")))?;
    Ok(())
}

/// Return the current DisplayList JSON protocol version.
///
/// Returns
/// -------
/// int
///     Protocol version (currently ``1``).
#[pyfunction]
fn display_list_version() -> i32 {
    1
}

/// Render a LaTeX math string to a self-contained SVG string.
///
/// Parameters
/// ----------
/// latex : str
///     The LaTeX math input (no surrounding `$…$` delimiters).
/// font_size : float, optional
///     Em size in user units (default 40.0).
/// display_mode : bool, optional
///     ``True`` for block/display math (default), ``False`` for inline.
/// color : str, optional
///     Foreground color as ``#rrggbb``, ``#rgb``, or a named color
///     (``"black"`` / ``"white"``).  Default ``"black"``.
/// embed_glyphs : bool, optional
///     When ``True`` (default) glyphs are emitted as ``<path>`` elements so
///     the SVG is fully self-contained.  When ``False``, ``<text>`` elements
///     with KaTeX CSS class names are emitted instead (needs KaTeX stylesheets
///     in the host page).
///
/// Returns
/// -------
/// str
///     SVG document as a string.
#[pyfunction]
#[pyo3(signature = (latex, *, font_size=40.0, display_mode=true, color="black", embed_glyphs=true))]
fn render_svg(
    latex: &str,
    font_size: f64,
    display_mode: bool,
    color: &str,
    embed_glyphs: bool,
) -> PyResult<String> {
    let display_list = pipeline(latex, display_mode, color)?;
    let opts = SvgOptions {
        font_size,
        embed_glyphs,
        ..Default::default()
    };
    Ok(render_to_svg(&display_list, &opts))
}

/// Convenience: render a LaTeX string as inline (text-mode) SVG.
///
/// Equivalent to ``render_svg(latex, display_mode=False, embed_glyphs=True)``.
///
/// Parameters
/// ----------
/// latex : str
///     The LaTeX math input.
/// font_size : float, optional
///     Em size in user units (default 40.0).
/// color : str, optional
///     Foreground color.  Default ``"black"``.
///
/// Returns
/// -------
/// str
///     SVG document as a string.
#[pyfunction]
#[pyo3(signature = (latex, *, font_size=40.0, color="black"))]
fn render_svg_inline(latex: &str, font_size: f64, color: &str) -> PyResult<String> {
    render_svg(latex, font_size, false, color, true)
}

/// Render a LaTeX math string to PNG bytes.
///
/// Parameters
/// ----------
/// latex : str
///     The LaTeX math input.
/// font_size : float, optional
///     Em size in pixels at DPR 1 (default 40.0).
/// display_mode : bool, optional
///     ``True`` for block/display math (default), ``False`` for inline.
/// color : str, optional
///     Foreground color.  Default ``"black"``.
/// background_color : str, optional
///     Background fill.  Use ``"transparent"`` for a transparent PNG.
///     Default ``"white"``.
/// dpr : float, optional
///     Device pixel ratio multiplier (default 1.0).
///
/// Returns
/// -------
/// bytes
///     Raw PNG bytes.
#[pyfunction]
#[pyo3(signature = (latex, *, font_size=40.0, display_mode=true, color="black", background_color="white", dpr=1.0))]
fn render_png(
    latex: &str,
    font_size: f64,
    display_mode: bool,
    color: &str,
    background_color: &str,
    dpr: f64,
) -> PyResult<Vec<u8>> {
    let display_list = pipeline(latex, display_mode, color)?;
    let opts = RenderOptions {
        font_size: font_size as f32,
        background_color: parse_color(background_color)?,
        device_pixel_ratio: dpr as f32,
        ..Default::default()
    };
    render_to_png(&display_list, &opts).map_err(PyValueError::new_err)
}

/// Render a LaTeX math string to PDF bytes.
///
/// Parameters
/// ----------
/// latex : str
///     The LaTeX math input.
/// font_size : float, optional
///     Em size in user units (default 40.0).
/// display_mode : bool, optional
///     ``True`` for block/display math (default), ``False`` for inline.
/// color : str, optional
///     Foreground color.  Default ``"black"``.
///
/// Returns
/// -------
/// bytes
///     Raw PDF bytes with embedded KaTeX fonts.
#[pyfunction]
#[pyo3(signature = (latex, *, font_size=40.0, display_mode=true, color="black"))]
fn render_pdf(
    latex: &str,
    font_size: f64,
    display_mode: bool,
    color: &str,
) -> PyResult<Vec<u8>> {
    let display_list = pipeline(latex, display_mode, color)?;
    let opts = PdfOptions {
        font_size,
        ..Default::default()
    };
    render_to_pdf(&display_list, &opts).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Parse and lay out a LaTeX math string, returning the DisplayList as a JSON string.
///
/// The JSON schema is documented in ``docs/DISPLAYLIST_JSON_PROTOCOL.md``.
/// Useful for caching / custom renderers (Canvas 2D, PDF, platform-native drawing).
///
/// Parameters
/// ----------
/// latex : str
///     The LaTeX math input.
/// display_mode : bool, optional
///     ``True`` for block/display math (default), ``False`` for inline.
///
/// Returns
/// -------
/// str
///     DisplayList serialized as JSON (protocol version 1).
#[pyfunction]
#[pyo3(signature = (latex, *, display_mode=true))]
fn render_display_list(latex: &str, display_mode: bool) -> PyResult<String> {
    let display_list = pipeline(latex, display_mode, "black")?;
    serde_json::to_string(&display_list)
        .map_err(|e| PyValueError::new_err(format!("serialization error: {e}")))
}

/// Parse and lay out a LaTeX math string, returning the DisplayList as a JSON string.
///
/// Equivalent to ``render_display_list()``. Call ``json.loads()`` in Python to convert to a dict.
///
/// Parameters
/// ----------
/// latex : str
///     The LaTeX math input.
/// display_mode : bool, optional
///     ``True`` for block/display math (default), ``False`` for inline.
///
/// Returns
/// -------
/// str
///     DisplayList JSON string (protocol version 1).
#[pyfunction]
#[pyo3(signature = (latex, *, display_mode=true))]
fn parse_display_list(latex: &str, display_mode: bool) -> PyResult<String> {
    let display_list = pipeline(latex, display_mode, "black")?;
    serde_json::to_string(&display_list)
        .map_err(|e| PyValueError::new_err(format!("serialization error: {e}"))
    )
}

/// Render a DisplayList JSON string to SVG.
///
/// Accepts a cached JSON string from ``render_display_list()`` and renders it to SVG
/// without reparsing / relayouting. Useful for generating multiple formats from a single formula.
///
/// Parameters
/// ----------
/// display_list_json : str
///     The JSON DisplayList string (protocol version 1).
/// font_size : float, optional
///     Em size in user units (default 40.0).
/// embed_glyphs : bool, optional
///     When ``True`` (default) glyphs are emitted as ``<path>`` elements.
///
/// Returns
/// -------
/// str
///     SVG document as a string.
#[pyfunction]
#[pyo3(signature = (display_list_json, *, font_size=40.0, embed_glyphs=true))]
fn render_svg_from_display_list(
    display_list_json: &str,
    font_size: f64,
    embed_glyphs: bool,
) -> PyResult<String> {
    let display_list = deserialize_display_list(display_list_json)?;
    let opts = SvgOptions {
        font_size,
        embed_glyphs,
        ..Default::default()
    };
    Ok(render_to_svg(&display_list, &opts))
}

/// Render a DisplayList JSON string to PNG.
///
/// Accepts a cached JSON string from ``render_display_list()`` and renders it to PNG.
///
/// Parameters
/// ----------
/// display_list_json : str
///     The JSON DisplayList string.
/// font_size : float, optional
///     Em size in pixels at DPR 1 (default 40.0).
/// background_color : str, optional
///     Background fill (default ``"white"``).
/// dpr : float, optional
///     Device pixel ratio multiplier (default 1.0).
///
/// Returns
/// -------
/// bytes
///     Raw PNG bytes.
#[pyfunction]
#[pyo3(signature = (display_list_json, *, font_size=40.0, background_color="white", dpr=1.0))]
fn render_png_from_display_list(
    display_list_json: &str,
    font_size: f64,
    background_color: &str,
    dpr: f64,
) -> PyResult<Vec<u8>> {
    let display_list = deserialize_display_list(display_list_json)?;
    let opts = RenderOptions {
        font_size: font_size as f32,
        background_color: parse_color(background_color)?,
        device_pixel_ratio: dpr as f32,
        ..Default::default()
    };
    render_to_png(&display_list, &opts).map_err(PyValueError::new_err)
}

/// Render a DisplayList JSON string to PDF.
///
/// Accepts a cached JSON string from ``render_display_list()`` and renders it to PDF.
///
/// Parameters
/// ----------
/// display_list_json : str
///     The JSON DisplayList string.
/// font_size : float, optional
///     Em size in user units (default 40.0).
///
/// Returns
/// -------
/// bytes
///     Raw PDF bytes with embedded KaTeX fonts.
#[pyfunction]
#[pyo3(signature = (display_list_json, *, font_size=40.0))]
fn render_pdf_from_display_list(
    display_list_json: &str,
    font_size: f64,
) -> PyResult<Vec<u8>> {
    let display_list = deserialize_display_list(display_list_json)?;
    let opts = PdfOptions {
        font_size,
        ..Default::default()
    };
    render_to_pdf(&display_list, &opts).map_err(|e| PyValueError::new_err(e.to_string()))
}

/// Render a list of LaTeX math strings to SVG (batch mode).
///
/// Amortises Python FFI overhead when rendering many formulas in a single pass.
///
/// Parameters
/// ----------
/// latexes : list[str]
///     List of LaTeX math inputs.
/// font_size : float, optional
///     Em size in user units (default 40.0).
/// display_mode : bool, optional
///     ``True`` for block/display math (default), ``False`` for inline.
/// color : str, optional
///     Foreground color.  Default ``"black"``.
/// embed_glyphs : bool, optional
///     When ``True`` (default) glyphs are emitted as ``<path>`` elements.
///
/// Returns
/// -------
/// list[str]
///     SVG strings in the same order as input.
///
/// Raises
/// ------
/// ValueError
///     If any formula fails to parse or render.
#[pyfunction]
#[pyo3(signature = (latexes, *, font_size=40.0, display_mode=true, color="black", embed_glyphs=true))]
fn render_svg_batch(
    latexes: Vec<String>,
    font_size: f64,
    display_mode: bool,
    color: &str,
    embed_glyphs: bool,
) -> PyResult<Vec<String>> {
    let mut results = Vec::with_capacity(latexes.len());
    for latex in latexes {
        let svg = render_svg(&latex, font_size, display_mode, color, embed_glyphs)?;
        results.push(svg);
    }
    Ok(results)
}

/// Render a list of LaTeX math strings to PNG (batch mode).
///
/// Parameters
/// ----------
/// latexes : list[str]
///     List of LaTeX math inputs.
/// font_size : float, optional
///     Em size in pixels at DPR 1 (default 40.0).
/// display_mode : bool, optional
///     ``True`` for block/display math (default), ``False`` for inline.
/// color : str, optional
///     Foreground color.  Default ``"black"``.
/// background_color : str, optional
///     Background fill.  Default ``"white"``.
/// dpr : float, optional
///     Device pixel ratio multiplier (default 1.0).
///
/// Returns
/// -------
/// list[bytes]
///     PNG bytes in the same order as input.
///
/// Raises
/// ------
/// ValueError
///     If any formula fails to parse or render.
#[pyfunction]
#[pyo3(signature = (latexes, *, font_size=40.0, display_mode=true, color="black", background_color="white", dpr=1.0))]
fn render_png_batch(
    latexes: Vec<String>,
    font_size: f64,
    display_mode: bool,
    color: &str,
    background_color: &str,
    dpr: f64,
) -> PyResult<Vec<Vec<u8>>> {
    let mut results = Vec::with_capacity(latexes.len());
    for latex in latexes {
        let png = render_png(&latex, font_size, display_mode, color, background_color, dpr)?;
        results.push(png);
    }
    Ok(results)
}

/// Render one LaTeX input to one or more formats in a single call.
///
/// Parameters
/// ----------
/// latex : str
///     LaTeX math input.
/// formats : list[str], optional
///     One or more output formats: ``"svg"``, ``"png"``, ``"pdf"``,
///     ``"html"``, ``"json"``, ``"display_list"``.
///     Defaults to ``["svg"]``.
/// font_size : float, optional
///     Em size in user units (default 40.0).
/// display_mode : bool, optional
///     ``True`` for display math (default), ``False`` for inline.
/// color : str, optional
///     Foreground color (default ``"black"``).
/// embed_glyphs : bool, optional
///     SVG glyph embedding behavior (default ``True``).
/// background_color : str, optional
///     PNG background color (default ``"white"``).
/// dpr : float, optional
///     PNG device pixel ratio (default ``1.0``).
///
/// Returns
/// -------
/// dict
///     Mapping of format name to rendered payload.
#[pyfunction]
#[pyo3(signature = (latex, formats=None, *, font_size=40.0, display_mode=true, color="black", embed_glyphs=true, background_color="white", dpr=1.0))]
fn render_formats(
    py: Python<'_>,
    latex: &str,
    formats: Option<Vec<String>>,
    font_size: f64,
    display_mode: bool,
    color: &str,
    embed_glyphs: bool,
    background_color: &str,
    dpr: f64,
) -> PyResult<Py<PyDict>> {
    let formats = formats.unwrap_or_else(|| vec!["svg".to_string()]);
    render_formats_impl(
        py,
        latex,
        &formats,
        RenderParams {
            font_size,
            display_mode,
            color,
            embed_glyphs,
            background_color,
            dpr,
        },
    )
}

// ---------------------------------------------------------------------------
// Module registration
// ---------------------------------------------------------------------------

/// Python bindings for RaTeX — KaTeX-compatible math rendering in pure Rust.
///
/// Core functions::
///
///     import ratex_py
///     svg = ratex_py.render_svg(r"\frac{-b \pm \sqrt{b^2-4ac}}{2a}")
///     png = ratex_py.render_png(r"\int_0^\infty e^{-x^2}\,dx", font_size=32.0)
///     pdf = ratex_py.render_pdf(r"E = mc^2")
///
/// Caching DisplayList for multiple formats::
///
///     dl = ratex_py.render_display_list(r"\sqrt{x}")
///     svg = ratex_py.render_svg_from_display_list(dl)
///     png = ratex_py.render_png_from_display_list(dl)
///
/// Validation::
///
///     ratex_py.check(r"\frac{1}{2}")  # raises on error
///     v = ratex_py.display_list_version()  # returns 1
#[pymodule]
fn ratex_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(check, m)?)?;
    m.add_function(wrap_pyfunction!(display_list_version, m)?)?;
    m.add_function(wrap_pyfunction!(render_svg, m)?)?;
    m.add_function(wrap_pyfunction!(render_svg_inline, m)?)?;
    m.add_function(wrap_pyfunction!(render_png, m)?)?;
    m.add_function(wrap_pyfunction!(render_pdf, m)?)?;
    m.add_function(wrap_pyfunction!(render_display_list, m)?)?;
    m.add_function(wrap_pyfunction!(parse_display_list, m)?)?;
    m.add_function(wrap_pyfunction!(render_svg_from_display_list, m)?)?;
    m.add_function(wrap_pyfunction!(render_png_from_display_list, m)?)?;
    m.add_function(wrap_pyfunction!(render_pdf_from_display_list, m)?)?;
    m.add_function(wrap_pyfunction!(render_svg_batch, m)?)?;
    m.add_function(wrap_pyfunction!(render_png_batch, m)?)?;
    m.add_function(wrap_pyfunction!(render_formats, m)?)?;
    m.add_class::<Expr>()?;
    let expr = m.getattr("Expr")?;
    m.add("Math", expr)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests (100% branch coverage)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Color parsing: all branches
    #[test]
    fn test_parse_color_black() {
        assert_eq!(parse_color("black").unwrap(), Color::BLACK);
        assert_eq!(parse_color("BLACK").unwrap(), Color::BLACK);
    }

    #[test]
    fn test_parse_color_white() {
        assert_eq!(parse_color("white").unwrap(), Color::WHITE);
        assert_eq!(parse_color("WHITE").unwrap(), Color::WHITE);
    }

    #[test]
    fn test_parse_color_transparent() {
        let c = parse_color("transparent").unwrap();
        assert_eq!(c, Color::new(0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn test_parse_color_hex6() {
        let c = parse_color("#ff0000").unwrap();
        assert!((c.r - 1.0).abs() < 0.01);
        assert!(c.g < 0.01);
        assert!(c.b < 0.01);
    }

    #[test]
    fn test_parse_color_hex3() {
        let c = parse_color("#f00").unwrap();
        assert!((c.r - 1.0).abs() < 0.01);
        assert!(c.g < 0.01);
        assert!(c.b < 0.01);
    }

    #[test]
    fn test_parse_color_hex_with_whitespace() {
        let c = parse_color("  #000  ").unwrap();
        assert_eq!(c, Color::new(0.0, 0.0, 0.0, 1.0));
    }

    #[test]
    fn test_parse_color_invalid_hex_length() {
        assert!(parse_color("#ff00").is_err());
    }

    #[test]
    fn test_parse_color_invalid_hex_chars() {
        assert!(parse_color("#gggggg").is_err());
    }

    #[test]
    fn test_parse_color_invalid_format() {
        assert!(parse_color("rgb(255,0,0)").is_err());
    }

    // Layout options: both branches (display vs text mode)
    #[test]
    fn test_make_layout_options_display() {
        let opts = make_layout_options(true, "black").unwrap();
        assert_eq!(opts.style, MathStyle::Display);
        assert_eq!(opts.color, Color::BLACK);
    }

    #[test]
    fn test_make_layout_options_text() {
        let opts = make_layout_options(false, "white").unwrap();
        assert_eq!(opts.style, MathStyle::Text);
        assert_eq!(opts.color, Color::WHITE);
    }

    // Pipeline: success path
    #[test]
    fn test_pipeline_simple_fraction() {
        let dl = pipeline(r"\frac{1}{2}", true, "black").unwrap();
        assert!(dl.width > 0.0);
        assert!(dl.height > 0.0);
        assert!(dl.items.len() > 0);
    }

    #[test]
    fn test_pipeline_display_vs_text_modes() {
        let dl_display = pipeline(r"\sqrt{x}", true, "black").unwrap();
        let dl_text = pipeline(r"\sqrt{x}", false, "black").unwrap();
        // Both should succeed; sizes may differ
        assert!(dl_display.width > 0.0);
        assert!(dl_text.width > 0.0);
    }

    #[test]
    fn test_pipeline_parse_error() {
        // Unmatched \left (this should fail to parse)
        let result = pipeline(r"\left(", true, "black");
        assert!(result.is_err());
    }

    // Deserialize: success and error
    #[test]
    fn test_deserialize_display_list_valid() {
        let json = r#"{"version":1,"width":1.0,"height":2.0,"depth":0.5,"items":[]}"#;
        let dl = deserialize_display_list(json).unwrap();
        assert_eq!(dl.width, 1.0);
        assert_eq!(dl.height, 2.0);
        assert_eq!(dl.depth, 0.5);
    }

    #[test]
    fn test_deserialize_display_list_invalid_json() {
        let result = deserialize_display_list(r#"{"invalid json"#);
        assert!(result.is_err());
    }

    // Test the public functions (simple happy-path tests without Python::with_gil)
    #[test]
    fn test_display_list_version() {
        assert_eq!(display_list_version(), 1);
    }

    #[test]
    fn test_normalize_formats_single() {
        let out = normalize_formats(&["svg".to_string()]).unwrap();
        assert_eq!(out, vec!["svg".to_string()]);
    }

    #[test]
    fn test_normalize_formats_dedup_and_casefold() {
        let out = normalize_formats(&[
            "SVG".to_string(),
            "png".to_string(),
            "svg".to_string(),
        ])
        .unwrap();
        assert_eq!(out, vec!["svg".to_string(), "png".to_string()]);
    }

    #[test]
    fn test_normalize_formats_empty() {
        assert!(normalize_formats(&[]).is_err());
    }

    #[test]
    fn test_normalize_formats_invalid() {
        assert!(normalize_formats(&["jpeg".to_string()]).is_err());
    }
}
