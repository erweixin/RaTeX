# GTK4 Linux Support

This repository now includes an early native GTK4 integration for Linux:

- `crates/ratex-cairo` renders a RaTeX `DisplayList` into a Cairo context.
- `crates/ratex-gtk4` provides a `RatexFormula` GTK4 widget implemented as a real `GtkWidget` subclass.

This first pass is intentionally small and boring:

- bundled KaTeX fonts by default
- optional `font-dir` override
- synchronous parse/layout on property changes
- manual test example before GObject Introspection packaging work

## Run the example

Install GTK4 development packages first.

On Debian / Ubuntu:

```bash
sudo apt-get install -y libgtk-4-dev libcairo2-dev
```

From the repository root:

```bash
cargo run -p ratex-gtk4 --example formula_demo
```

The demo window includes:

- a LaTeX entry field
- a display-mode toggle
- a font-size spinner
- a live `RatexFormula` widget

## Current scope

Implemented now:

- Cairo renderer for `DisplayList`
- GTK4 widget subclass with measurement and snapshot drawing
- bundled-font default behavior
- local runnable example

Planned next:

- theme-derived default foreground color
- GObject Introspection (`.gir` / `.typelib`)
- C, Python, and Vala smoke examples
- CI coverage for GTK/GI tooling
