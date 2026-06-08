using Gtk;
using RatexGtk;

public class Demo : Gtk.Application {
    public Demo () {
        Object (application_id: "io.ratex.demo.gtk4.vala");
    }

    protected override void activate () {
        var formula = new RatexGtk.Formula ();
        formula.latex = "\\int_0^\\infty e^{-x^2} dx = \\frac{\\sqrt{\\pi}}{2}";
        formula.font_size = 36.0;
        formula.display_mode = true;

        var box = new Gtk.Box (Gtk.Orientation.VERTICAL, 12);
        box.margin_top = 24;
        box.margin_bottom = 24;
        box.margin_start = 24;
        box.margin_end = 24;
        box.append (formula);

        var window = new Gtk.ApplicationWindow (this) {
            title = "RaTeX GTK4 Vala Demo",
            default_width = 900,
            default_height = 240,
            child = box
        };
        window.present ();
    }

    public static int main (string[] args) {
        return new Demo ().run (args);
    }
}
