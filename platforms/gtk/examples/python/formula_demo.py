#!/usr/bin/env python3

import gi

gi.require_version("Gtk", "4.0")
gi.require_version("RatexGtk", "1.0")

from gi.repository import Gtk, RatexGtk


class Demo(Gtk.Application):
    def __init__(self):
        super().__init__(application_id="io.ratex.demo.gtk4.python")

    def do_activate(self):
        formula = RatexGtk.Formula()
        formula.set_property("latex", r"\int_0^\infty e^{-x^2} dx = \frac{\sqrt{\pi}}{2}")
        formula.set_property("font-size", 36.0)
        formula.set_property("display-mode", True)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_margin_top(24)
        box.set_margin_bottom(24)
        box.set_margin_start(24)
        box.set_margin_end(24)
        box.append(formula)

        window = Gtk.ApplicationWindow(application=self, title="RaTeX GTK4 Python Demo")
        window.set_default_size(900, 240)
        window.set_child(box)
        window.present()


if __name__ == "__main__":
    raise SystemExit(Demo().run())
