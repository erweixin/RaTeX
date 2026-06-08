mod imp;

use std::path::PathBuf;

use gtk::gdk;
use gtk::glib;
use gtk::glib::translate::{IntoGlib, IntoGlibPtr};
use gtk::prelude::*;
use gtk::subclass::prelude::ObjectSubclassIsExt;
use gtk4 as gtk;

glib::wrapper! {
    pub struct RatexFormula(ObjectSubclass<imp::RatexFormula>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl RatexFormula {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn latex(&self) -> String {
        self.property::<String>("latex")
    }

    pub fn set_latex(&self, latex: &str) {
        self.set_property("latex", latex);
    }

    pub fn display_mode(&self) -> bool {
        self.property::<bool>("display-mode")
    }

    pub fn set_display_mode(&self, display_mode: bool) {
        self.set_property("display-mode", display_mode);
    }

    pub fn font_size(&self) -> f64 {
        self.property::<f64>("font-size")
    }

    pub fn set_font_size(&self, font_size: f64) {
        self.set_property("font-size", font_size);
    }

    pub fn padding(&self) -> f64 {
        self.property::<f64>("padding")
    }

    pub fn set_padding(&self, padding: f64) {
        self.set_property("padding", padding);
    }

    pub fn color(&self) -> Option<gdk::RGBA> {
        self.property::<Option<gdk::RGBA>>("color")
    }

    pub fn set_color(&self, color: Option<&gdk::RGBA>) {
        self.set_property("color", color.cloned());
    }

    pub fn font_dir(&self) -> Option<PathBuf> {
        self.property::<Option<String>>("font-dir")
            .map(PathBuf::from)
    }

    pub fn set_font_dir(&self, font_dir: Option<&std::path::Path>) {
        let value = font_dir.map(|path| path.to_string_lossy().into_owned());
        self.set_property("font-dir", value);
    }

    pub fn error_message(&self) -> Option<String> {
        self.property::<Option<String>>("error-message")
    }

    pub fn baseline_px(&self) -> f64 {
        self.imp().metrics().baseline
    }
}

impl Default for RatexFormula {
    fn default() -> Self {
        Self::new()
    }
}

pub use imp::FormulaMetrics;

fn ensure_gtk_initialized_for_c() -> bool {
    if gtk::is_initialized_main_thread() {
        return true;
    }

    unsafe {
        if gtk::glib::translate::from_glib::<_, bool>(gtk::ffi::gtk_is_initialized()) {
            // C/GI applications initialize GTK themselves. Bridge that state into gtk-rs
            // without taking ownership of application initialization here.
            gtk::set_initialized();
            return true;
        }
    }

    false
}

#[no_mangle]
pub extern "C" fn ratex_formula_get_type() -> glib::ffi::GType {
    if !ensure_gtk_initialized_for_c() {
        return 0;
    }
    RatexFormula::static_type().into_glib()
}

#[no_mangle]
pub extern "C" fn ratex_formula_new() -> *mut gtk::ffi::GtkWidget {
    if !ensure_gtk_initialized_for_c() {
        return std::ptr::null_mut();
    }
    unsafe { RatexFormula::new().upcast::<gtk::Widget>().into_glib_ptr() }
}
