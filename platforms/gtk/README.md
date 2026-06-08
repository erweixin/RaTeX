# GTK4 Linux Support

This repository now includes an early native GTK4 integration for Linux:

- `crates/ratex-cairo` renders a RaTeX `DisplayList` into a Cairo context.
- `crates/ratex-gtk4` provides a `RatexFormula` GTK4 widget implemented as a real `GtkWidget` subclass.

This first pass is intentionally small and boring:

- bundled KaTeX fonts by default
- optional `font-dir` override
- synchronous parse/layout on property changes
- Rust, C, Python, and Vala smoke examples
- GObject Introspection namespace: `RatexGtk-1.0`

## Run the example

Install GTK4 development packages first.

On Debian / Ubuntu:

```bash
sudo apt-get install -y \
  build-essential \
  libgtk-4-dev \
  libadwaita-1-dev \
  libcairo2-dev \
  gobject-introspection \
  libgirepository1.0-dev \
  python3-gi \
  python3-gi-cairo \
  valac \
  xvfb
```

From the repository root, run the preferred GNOME/libadwaita showcase:

```bash
cargo run -p ratex-gtk4 --example adw_formula_demo
```

The Adwaita demo includes:

- a native Adwaita application window and header bar
- a LaTeX entry field
- a display-mode toggle
- a font-size spinner
- an appearance selector for System / Light / Dark
- a live `RatexFormula` widget

For a minimal plain-GTK smoke demo, run:

```bash
cargo run -p ratex-gtk4 --example formula_demo
```

The plain GTK demo includes:

- a LaTeX entry field
- a display-mode toggle
- a font-size spinner
- a live `RatexFormula` widget

## Current scope

Implemented now:

- Cairo renderer for `DisplayList`
- GTK4 widget subclass with measurement and snapshot drawing
- C/GObject entry points: `ratex_formula_get_type()` and `ratex_formula_new()`
- `RatexGtk-1.0.gir` metadata and Vala `.vapi`
- theme-derived foreground text color when the `color` property is unset
- bundled-font default behavior
- local runnable Rust GTK, Rust Adwaita, C, Python, and Vala examples

Color behavior:

- If the widget `color` property is unset, default-colored formula items render with GTK's current foreground text color.
- If the widget `color` property is set, that color is used as the formula default.
- Inline LaTeX colors such as `\color` and `\textcolor` remain per-item overrides.

Planned next:

- install helper or distro packaging glue for copying headers, shared libraries, `.gir`, `.typelib`, `.vapi`, and `.pc` files into a prefix

## GObject / GI package shape

The public GObject namespace is `RatexGtk-1.0`. The intended install layout is:

```text
include/ratex-gtk-1.0/ratex-gtk.h
lib/libratex_gtk4.so
lib/pkgconfig/ratex-gtk-1.0.pc
share/gir-1.0/RatexGtk-1.0.gir
lib/girepository-1.0/RatexGtk-1.0.typelib
share/vala/vapi/ratex-gtk-1.0.vapi
```

The checked-in metadata lives under `platforms/gtk/`:

- `include/ratex-gtk-1.0/ratex-gtk.h`
- `gir/RatexGtk-1.0.gir`
- `pkgconfig/ratex-gtk-1.0.pc.in`
- `vapi/ratex-gtk-1.0.vapi`
- `examples/c/formula_demo.c`
- `examples/c/init_contract.c`
- `examples/python/formula_demo.py`
- `examples/vala/formula_demo.vala`

Initialization contract:

- `RatexFormula` is implemented as a gtk-rs `GtkWidget` subclass.
- GTK must be initialized on the main thread before querying `RatexFormula`'s type, calling `ratex_formula_new()`, or constructing `RatexGtk.Formula` through GI.
- Before GTK initialization, `ratex_formula_get_type()` returns `G_TYPE_INVALID` and `ratex_formula_new()` returns `NULL`.
- C/Python/Vala examples construct the widget from application activation, after GTK has initialized.

Binding metadata maintenance:

- `RatexGtk-1.0.gir` and `ratex-gtk-1.0.vapi` are hand-maintained metadata for the exported Rust GObject type.
- CI checks the exported C symbols and expected property names to catch basic drift between Rust, C, GIR, and VAPI metadata.

## Local C / GI smoke commands

Build the Rust shared library and the Rust demo:

```bash
cargo build -p ratex-gtk4 --lib --example formula_demo --example adw_formula_demo
```

Compile the typelib when `g-ir-compiler` is installed:

```bash
g-ir-compiler platforms/gtk/gir/RatexGtk-1.0.gir \
  -o target/debug/RatexGtk-1.0.typelib
```

Build and run the C example:

```bash
gcc platforms/gtk/examples/c/formula_demo.c \
  -o target/debug/ratex-gtk-c-demo \
  -Iplatforms/gtk/include/ratex-gtk-1.0 \
  -Ltarget/debug -lratex_gtk4 \
  $(pkg-config --cflags --libs gtk4) \
  -Wl,-rpath,$PWD/target/debug

LD_LIBRARY_PATH=$PWD/target/debug target/debug/ratex-gtk-c-demo
```

Build and run the C initialization-contract smoke test:

```bash
gcc platforms/gtk/examples/c/init_contract.c \
  -o target/debug/ratex-gtk-c-init-contract \
  -Iplatforms/gtk/include/ratex-gtk-1.0 \
  -Ltarget/debug -lratex_gtk4 \
  $(pkg-config --cflags --libs gtk4) \
  -Wl,-rpath,$PWD/target/debug

LD_LIBRARY_PATH=$PWD/target/debug target/debug/ratex-gtk-c-init-contract
```

Run the Python GI example:

```bash
LD_LIBRARY_PATH=$PWD/target/debug \
GI_TYPELIB_PATH=$PWD/target/debug \
python3 platforms/gtk/examples/python/formula_demo.py
```

Build and run the Vala example:

```bash
valac --vapidir=platforms/gtk/vapi \
  --pkg gtk4 --pkg ratex-gtk-1.0 \
  -X -Iplatforms/gtk/include/ratex-gtk-1.0 \
  -X -Ltarget/debug -X -lratex_gtk4 \
  -X -Wl,-rpath,$PWD/target/debug \
  platforms/gtk/examples/vala/formula_demo.vala \
  -o target/debug/ratex-gtk-vala-demo

LD_LIBRARY_PATH=$PWD/target/debug target/debug/ratex-gtk-vala-demo
```
