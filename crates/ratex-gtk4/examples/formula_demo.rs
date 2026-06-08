use gtk4::prelude::*;
use gtk4::{
    Align, Application, ApplicationWindow, Box as GtkBox, CheckButton, Entry, Label, Orientation,
    SpinButton,
};
use ratex_gtk4::RatexFormula;

fn main() {
    let app = Application::builder()
        .application_id("io.ratex.demo.gtk4")
        .build();
    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let formula = RatexFormula::new();
    formula.set_hexpand(true);
    formula.set_vexpand(true);
    formula.set_halign(Align::Start);
    formula.set_valign(Align::Start);
    formula.set_latex(r"\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}");
    formula.set_font_size(36.0);
    formula.set_margin_top(12);

    let entry = Entry::builder()
        .hexpand(true)
        .text(r"\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}")
        .build();
    {
        let formula = formula.clone();
        entry.connect_changed(move |entry| {
            formula.set_latex(entry.text().as_ref());
        });
    }

    let display_mode = CheckButton::builder()
        .label("Display mode")
        .active(true)
        .build();
    {
        let formula = formula.clone();
        display_mode.connect_toggled(move |button| {
            formula.set_display_mode(button.is_active());
        });
    }

    let font_size = SpinButton::with_range(8.0, 128.0, 1.0);
    font_size.set_value(36.0);
    {
        let formula = formula.clone();
        font_size.connect_value_changed(move |spin| {
            formula.set_font_size(spin.value());
        });
    }

    let error_label = Label::new(None);
    error_label.set_halign(Align::Start);
    {
        let formula = formula.clone();
        let error_label = error_label.clone();
        formula.connect_notify_local(Some("error-message"), move |formula, _| {
            error_label.set_text(formula.error_message().as_deref().unwrap_or(""));
        });
    }

    let controls = GtkBox::builder()
        .orientation(Orientation::Horizontal)
        .spacing(12)
        .build();
    controls.append(&display_mode);
    controls.append(&Label::new(Some("Font size")));
    controls.append(&font_size);

    let content = GtkBox::builder()
        .orientation(Orientation::Vertical)
        .spacing(12)
        .margin_top(24)
        .margin_bottom(24)
        .margin_start(24)
        .margin_end(24)
        .build();
    content.append(&entry);
    content.append(&controls);
    content.append(&formula);
    content.append(&error_label);

    let window = ApplicationWindow::builder()
        .application(app)
        .title("RaTeX GTK4 Demo")
        .default_width(900)
        .default_height(400)
        .child(&content)
        .build();
    window.present();
}
