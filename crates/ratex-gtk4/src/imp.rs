use std::cell::RefCell;
use std::path::PathBuf;
use std::sync::LazyLock;

use gtk::gdk;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk4 as gtk;
use ratex_cairo::{measure_display_list, render_to_cairo, CairoOptions, RenderMetrics};
use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parse;
use ratex_types::{Color, DisplayList, MathStyle};

#[derive(Debug, Clone, Copy)]
pub struct FormulaMetrics {
    pub width: i32,
    pub height: i32,
    pub baseline: i32,
}

impl Default for FormulaMetrics {
    fn default() -> Self {
        Self {
            width: 1,
            height: 1,
            baseline: 0,
        }
    }
}

#[derive(Debug, Clone)]
struct FormulaState {
    latex: String,
    display_mode: bool,
    font_size: f64,
    padding: f64,
    color: Option<gdk::RGBA>,
    font_dir: Option<PathBuf>,
    display_list: Option<DisplayList>,
    error_message: Option<String>,
    metrics: FormulaMetrics,
}

impl Default for FormulaState {
    fn default() -> Self {
        Self {
            latex: String::new(),
            display_mode: true,
            font_size: 24.0,
            padding: 4.0,
            color: None,
            font_dir: None,
            display_list: None,
            error_message: None,
            metrics: FormulaMetrics::default(),
        }
    }
}

#[derive(Default)]
pub struct RatexFormula {
    state: RefCell<FormulaState>,
}

#[glib::object_subclass]
impl ObjectSubclass for RatexFormula {
    const NAME: &'static str = "RaTeXFormula";
    type Type = super::RatexFormula;
    type ParentType = gtk::Widget;
}

impl ObjectImpl for RatexFormula {
    fn constructed(&self) {
        self.parent_constructed();
        self.relayout();
    }

    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: LazyLock<Vec<glib::ParamSpec>> = LazyLock::new(|| {
            vec![
                glib::ParamSpecString::builder("latex")
                    .explicit_notify()
                    .build(),
                glib::ParamSpecBoolean::builder("display-mode")
                    .default_value(true)
                    .explicit_notify()
                    .build(),
                glib::ParamSpecDouble::builder("font-size")
                    .minimum(1.0)
                    .maximum(4096.0)
                    .default_value(24.0)
                    .explicit_notify()
                    .build(),
                glib::ParamSpecDouble::builder("padding")
                    .minimum(0.0)
                    .maximum(512.0)
                    .default_value(4.0)
                    .explicit_notify()
                    .build(),
                glib::ParamSpecBoxed::builder::<gdk::RGBA>("color")
                    .explicit_notify()
                    .build(),
                glib::ParamSpecString::builder("font-dir")
                    .explicit_notify()
                    .build(),
                glib::ParamSpecString::builder("error-message")
                    .read_only()
                    .build(),
            ]
        });
        PROPERTIES.as_ref()
    }

    fn set_property(&self, id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        let mut state = self.state.borrow_mut();
        let mut relayout = false;
        match pspec.name() {
            "latex" => {
                let latex = value
                    .get::<String>()
                    .expect("latex property should be a string");
                if state.latex != latex {
                    state.latex = latex;
                    relayout = true;
                }
            }
            "display-mode" => {
                let display_mode = value
                    .get::<bool>()
                    .expect("display-mode property should be a bool");
                if state.display_mode != display_mode {
                    state.display_mode = display_mode;
                    relayout = true;
                }
            }
            "font-size" => {
                let font_size = value
                    .get::<f64>()
                    .expect("font-size property should be a double");
                if (state.font_size - font_size).abs() > f64::EPSILON {
                    state.font_size = font_size;
                    relayout = true;
                }
            }
            "padding" => {
                let padding = value
                    .get::<f64>()
                    .expect("padding property should be a double");
                if (state.padding - padding).abs() > f64::EPSILON {
                    state.padding = padding;
                    relayout = true;
                }
            }
            "color" => {
                let color = value
                    .get::<Option<gdk::RGBA>>()
                    .expect("color property should be an RGBA or None");
                if state.color != color {
                    state.color = color;
                    relayout = true;
                }
            }
            "font-dir" => {
                let font_dir = value
                    .get::<Option<String>>()
                    .expect("font-dir property should be a string or None")
                    .map(PathBuf::from);
                if state.font_dir != font_dir {
                    state.font_dir = font_dir;
                    relayout = true;
                }
            }
            _ => unreachable!("unknown property {id}"),
        }
        drop(state);

        if relayout {
            self.relayout();
            self.obj().notify(pspec.name());
        }
    }

    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        let state = self.state.borrow();
        match pspec.name() {
            "latex" => state.latex.to_value(),
            "display-mode" => state.display_mode.to_value(),
            "font-size" => state.font_size.to_value(),
            "padding" => state.padding.to_value(),
            "color" => state.color.to_value(),
            "font-dir" => state
                .font_dir
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned())
                .to_value(),
            "error-message" => state.error_message.to_value(),
            _ => unreachable!(),
        }
    }
}

impl WidgetImpl for RatexFormula {
    fn request_mode(&self) -> gtk::SizeRequestMode {
        gtk::SizeRequestMode::ConstantSize
    }

    fn measure(&self, orientation: gtk::Orientation, _for_size: i32) -> (i32, i32, i32, i32) {
        let metrics = self.state.borrow().metrics;
        match orientation {
            gtk::Orientation::Horizontal => (metrics.width, metrics.width, -1, -1),
            gtk::Orientation::Vertical => (
                metrics.height,
                metrics.height,
                metrics.baseline,
                metrics.baseline,
            ),
            _ => (1, 1, -1, -1),
        }
    }

    fn snapshot(&self, snapshot: &gtk::Snapshot) {
        let obj = self.obj();
        let width = obj.width().max(1) as f32;
        let height = obj.height().max(1) as f32;
        let bounds = gtk::graphene::Rect::new(0.0, 0.0, width, height);
        let cr = snapshot.append_cairo(&bounds);

        let state = self.state.borrow();
        let Some(display_list) = state.display_list.as_ref() else {
            return;
        };

        let options = CairoOptions {
            font_size: state.font_size,
            padding: state.padding,
            font_dir: state.font_dir.clone(),
        };
        let _ = render_to_cairo(&cr, display_list, &options);
    }
}

impl RatexFormula {
    pub fn metrics(&self) -> RenderMetrics {
        let state = self.state.borrow();
        RenderMetrics {
            width: state.metrics.width as f64,
            total_height: state.metrics.height as f64,
            baseline: state.metrics.baseline as f64,
        }
    }

    fn relayout(&self) {
        let obj = self.obj();
        let old_metrics = self.state.borrow().metrics;

        let mut state = self.state.borrow_mut();
        let result = layout_formula(
            &state.latex,
            state.display_mode,
            state.color,
            state.font_size,
            state.padding,
            state.font_dir.clone(),
        );

        match result {
            Ok((display_list, metrics)) => {
                state.display_list = Some(display_list);
                state.error_message = None;
                state.metrics = metrics;
            }
            Err(message) => {
                state.display_list = None;
                state.error_message = Some(message);
                state.metrics = FormulaMetrics::default();
            }
        }

        let metrics_changed = state.metrics.width != old_metrics.width
            || state.metrics.height != old_metrics.height
            || state.metrics.baseline != old_metrics.baseline;
        drop(state);

        if metrics_changed {
            obj.queue_resize();
        }
        obj.queue_draw();
        obj.notify("error-message");
    }
}

fn layout_formula(
    latex: &str,
    display_mode: bool,
    color: Option<gdk::RGBA>,
    font_size: f64,
    padding: f64,
    font_dir: Option<PathBuf>,
) -> Result<(DisplayList, FormulaMetrics), String> {
    if latex.trim().is_empty() {
        return Ok((DisplayList::default(), FormulaMetrics::default()));
    }

    let ast = parse(latex).map_err(|err| format!("Parse error: {err}"))?;
    let style = if display_mode {
        MathStyle::Display
    } else {
        MathStyle::Text
    };
    let color = color.map(color_from_rgba).unwrap_or(Color::BLACK);
    let layout_options = LayoutOptions::default().with_style(style).with_color(color);
    let layout_box = layout(&ast, &layout_options);
    let display_list = to_display_list(&layout_box);
    let metrics = measure_display_list(
        &display_list,
        &CairoOptions {
            font_size,
            padding,
            font_dir,
        },
    );

    Ok((
        display_list,
        FormulaMetrics {
            width: metrics.width.ceil() as i32,
            height: metrics.total_height.ceil() as i32,
            baseline: metrics.baseline.round() as i32,
        },
    ))
}

fn color_from_rgba(rgba: gdk::RGBA) -> Color {
    Color::new(rgba.red(), rgba.green(), rgba.blue(), rgba.alpha())
}
