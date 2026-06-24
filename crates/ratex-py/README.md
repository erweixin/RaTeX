# ratex-py: Python Bindings for RaTeX Math Rendering

PyO3 bindings for **RaTeX**, a fast, lightweight, feature-complete LaTeX math renderer in Rust. Renders LaTeX to SVG, PNG, PDF, and a structured display list format.

## Features

- **Fast**: Parse, layout, and render math in milliseconds
- **Accurate**: Full TeX math mode semantics via KaTeX
- **Multiple formats**: SVG (with or without embedded glyphs), PNG (with DPR support), PDF (with embedded fonts)
- **Caching**: Separate parse/layout from rendering via DisplayList JSON protocol
- **Batch rendering**: Render multiple formulas efficiently
- **Python-first API**: Type hints, keyword arguments, comprehensive error messages
- **Fontless**: Bundled KaTeX fonts (DejaVu, Latin Modern, Lato, Noto Sans)

## Installation

Install from source (requires Rust toolchain):

```bash
pip install src/RaTeX/
```

Or as part of the full dsport stack:

```bash
pip install -r requirements.txt
```

## Quick Start

```python
import ratex_py

# Render to SVG
svg = ratex_py.render_svg(r"\frac{-b \pm \sqrt{b^2-4ac}}{2a}")
print(svg)  # <svg ...>...</svg>

# Render to PNG (bytes)
png_bytes = ratex_py.render_png(r"E = mc^2")
with open("equation.png", "wb") as f:
    f.write(png_bytes)

# Render to PDF (bytes)
pdf_bytes = ratex_py.render_pdf(r"\int_0^\infty e^{-x^2}\,dx")
with open("equation.pdf", "wb") as f:
    f.write(pdf_bytes)

# Validate without rendering
ratex_py.check(r"\frac{1}{2}")  # returns None; raises on error
```

## API Reference

### Validation

#### `check(latex: str) -> None`
Parse LaTeX without rendering. Raises `ValueError` if parsing fails.

```python
ratex_py.check(r"\sqrt{2}")  # OK
ratex_py.check(r"\left(")     # raises ValueError
```

### Rendering

#### `render_svg(latex: str, *, font_size=40.0, display_mode=True, color="black", embed_glyphs=True) -> str`
Render to self-contained SVG.

- `font_size` (float): Em size in user units; affects absolute dimensions
- `display_mode` (bool): `True` for display/block math, `False` for inline
- `color` (str): Foreground color (CSS format: `#rrggbb`, `#rgb`, or named: `"black"`, `"white"`, etc.)
- `embed_glyphs` (bool): If `True`, embeds glyphs as `<path>` elements; if `False`, uses `<text>` elements (requires CSS)

```python
# Display mode (larger, centered)
svg = ratex_py.render_svg(r"\sqrt{x}")

# Inline mode (smaller, no vertical centering)
svg = ratex_py.render_svg_inline(r"\sqrt{x}")

# Custom color
svg = ratex_py.render_svg(r"x", color="#ff0000")
```

#### `render_svg_inline(latex: str, *, font_size=40.0, color="black") -> str`
Convenience wrapper for inline math SVG (equivalent to `render_svg(..., display_mode=False)`).

#### `render_png(latex: str, *, font_size=40.0, display_mode=True, color="black", background_color="white", dpr=1.0) -> bytes`
Render to PNG with optional transparency and DPR scaling.

- `background_color` (str): Background color; use `"transparent"` for RGBA
- `dpr` (float): Device pixel ratio; `2.0` produces 2x resolution

```python
# Standard white background
png = ratex_py.render_png(r"x^2")

# Transparent background for compositing
png = ratex_py.render_png(r"x^2", background_color="transparent")

# High-DPI output (2x)
png = ratex_py.render_png(r"x^2", dpr=2.0)
```

#### `render_pdf(latex: str, *, font_size=40.0, display_mode=True, color="black") -> bytes`
Render to PDF with embedded KaTeX fonts.

```python
pdf = ratex_py.render_pdf(r"E = mc^2")
with open("eq.pdf", "wb") as f:
    f.write(pdf)
```

### DisplayList Protocol

The **DisplayList** is a structured JSON representation of parsed and laid-out math. It decouples rendering from parsing, enabling caching and multi-format output.

See: [docs/DISPLAYLIST_JSON_PROTOCOL.md](https://github.com/erweixin/RaTeX/blob/main/docs/DISPLAYLIST_JSON_PROTOCOL.md)


#### `render_display_list(latex: str, *, display_mode=True) -> str`
Parse and layout LaTeX, returning DisplayList as JSON string (protocol version 1).

```python
import json

dl_json = ratex_py.render_display_list(r"\alpha + \beta")
dl = json.loads(dl_json)
print(dl["version"])   # 1
print(dl["width"])     # em units
print(dl["height"])    # em units
print(dl["items"])     # list of layout items
```

#### `parse_display_list(latex: str, *, display_mode=True) -> str`
Alias for `render_display_list()` for API symmetry.

#### `render_svg_from_display_list(dl_json: str, *, font_size=40.0, embed_glyphs=True) -> str`
Render cached DisplayList to SVG without re-parsing/laying out.

```python
# Parse once
dl_json = ratex_py.render_display_list(r"\frac{1}{2}")

# Render to multiple formats
svg = ratex_py.render_svg_from_display_list(dl_json)
png = ratex_py.render_png_from_display_list(dl_json)
pdf = ratex_py.render_pdf_from_display_list(dl_json)
```

#### `render_png_from_display_list(dl_json: str, *, font_size=40.0, background_color="white", dpr=1.0) -> bytes`
Render cached DisplayList to PNG.

#### `render_pdf_from_display_list(dl_json: str, *, font_size=40.0) -> bytes`
Render cached DisplayList to PDF.

### Batch Rendering

#### `render_svg_batch(latexes: list[str], *, font_size=40.0, display_mode=True, color="black", embed_glyphs=True) -> list[str]`
Render multiple LaTeX strings to SVG in one call (amortizes FFI overhead).

```python
svgs = ratex_py.render_svg_batch([
    r"\alpha",
    r"\beta",
    r"\gamma",
])
```

#### `render_png_batch(latexes: list[str], *, font_size=40.0, display_mode=True, color="black", background_color="white", dpr=1.0) -> list[bytes]`
Render multiple LaTeX strings to PNG in one call.

### Multi-Format Rendering

#### `render_formats(latex: str, formats: list[str] | None = None, *, font_size=40.0, display_mode=True, color="black", embed_glyphs=True, background_color="white", dpr=1.0) -> dict`
Render one expression to one or more output formats in a single call.

- Supported format names: `"svg"`, `"png"`, `"pdf"`, `"html"`, `"json"`, `"display_list"`
- Default formats: `["svg"]`
- Returns a dict keyed by format name with the rendered payload.

```python
# One format
single = ratex_py.render_formats(r"x^2", ["svg"])
print(single["svg"][:32])

# Multiple formats from one parse/layout pass
multi = ratex_py.render_formats(
    r"\frac{-b \pm \sqrt{b^2-4ac}}{2a}",
    ["svg", "png", "json", "display_list"],
    dpr=2.0,
)
svg = multi["svg"]            # str
png = multi["png"]            # bytes
meta = multi["json"]          # dict/list JSON object
dl_json = multi["display_list"]  # str
```

### Jupyter Rich Display

`ratex_py.Expr` is a generic rich-display wrapper (and `ratex_py.Math` is an alias).

- Implements `_repr_svg_`, `_repr_png_`, `_repr_html_`, `_repr_json_`
- Implements `_repr_mimebundle_` for multi-backend Jupyter output
- `formats=` controls which MIME payloads are emitted in `_repr_mimebundle_`

```python
import ratex_py

expr = ratex_py.Expr(
    r"\int_0^\infty e^{-x^2}\,dx",
    display_mode=True,
    formats=["svg", "png", "html", "json"],
)

# In Jupyter, this triggers _repr_mimebundle_
expr

# Optional explicit render API on the object
parts = expr.render(["svg", "pdf"])
```

```python
# Short alias for convenience
math_obj = ratex_py.Math(r"E = mc^2", formats=["svg", "html"])
```

### Constants

#### `display_list_version() -> int`
Returns the DisplayList JSON protocol version (currently `1`).

---

## Integration: Docutils

Use ratex-py to render math roles and directives in docutils-based workflows.

### Example: Custom Math Role

```python
# myconf.py
from docutils import nodes
from docutils.parsers.rst import directives, Directive
from docutils.parsers.rst.roles import register_canonical_role
import ratex_py
import base64

def math_role(name, rawtext, text, lineno, inliner, options=None, content=None):
    """Role to render inline math."""
    options = options or {}
    try:
        svg = ratex_py.render_svg_inline(text, color="black")
        # Create an image node from SVG
        image = nodes.image(uri=f"data:image/svg+xml;base64,{base64.b64encode(svg.encode()).decode()}")
        return [image], []
    except ValueError as e:
        msg = nodes.system_message(f"Math error: {e}", level=2)
        return [msg], [inliner.reporter.warning(str(e), line=lineno)]

register_canonical_role('math', math_role)
```

### Example: Math Block Directive

```python
class MathDirective(Directive):
    """Directive for block math equations."""
    required_arguments = 0
    optional_arguments = 0
    final_argument_whitespace = True
    has_content = True
    option_spec = {
        'label': directives.unchanged,
        'font_size': float,
    }

    def run(self):
        latex = '\n'.join(self.content)
        font_size = self.options.get('font_size', 40.0)
        try:
            svg = ratex_py.render_svg(latex, font_size=font_size)
            image = nodes.image(uri=f"data:image/svg+xml;base64,...")
            return [image]
        except ValueError as e:
            return [self.state_machine.reporter.error(f"Math error: {e}", line=self.lineno)]

directives.register_directive('math', MathDirective)
```

### Example: Docutils API

```python
import ratex_py
from docutils.core import publish_parts

# Configure custom math role/directive before processing
rst = """
This is inline :math:`x^2 + y^2 = r^2` math.

.. math::

   \\frac{1}{2}
"""

parts = publish_parts(rst, writer_name='html')
print(parts['html_body'])
```

---

## Integration: Sphinx

Use ratex-py with Sphinx to render math in documentation.

### Method 1: Custom Math Renderer (Recommended)

Create a Sphinx extension that patches the math role and domain:

```python
# sphinx_math_ext.py
from sphinx.roles import XRefRole
from sphinx.domains.c_cpp import CPPDomain
import ratex_py
import base64

def setup(app):
    """Register ratex-py as math backend."""
    
    def render_math_role(name, rawtext, text, lineno, inliner):
        """Render :math:`...` roles with ratex-py."""
        try:
            svg = ratex_py.render_svg_inline(text)
            uri = f"data:image/svg+xml;base64,{base64.b64encode(svg.encode()).decode()}"
            image = nodes.image(uri=uri)
            return [image], []
        except Exception as e:
            return [nodes.system_message(str(e), level=2)], []
    
    app.add_role('math', render_math_role)
```

### Method 2: Using Docutils Extension

In `conf.py`:

```python
# conf.py
import ratex_py
import os

extensions = [
    'sphinx.ext.mathjax',  # or 'sphinx.ext.imgmath'
]

# Override imgmath to use ratex-py
imgmath_latex_preamble = ''  # Not needed for ratex-py
imgmath_use_preview = False

# Custom image converter
def convert_svg_to_png(svg_str):
    """Convert SVG to PNG for compatibility."""
    return ratex_py.render_svg(svg_str)
```

### Example: Sphinx Document

```rst
.. math::
   :label: eq-quadratic

   x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}

Inline math :math:`E = mc^2` is also supported.
```

---

## Performance Considerations

### DisplayList Caching

For documents with many math expressions, parse once and render multiple times:

```python
# Slow: parse → SVG, parse → PNG, parse → PDF (3 parses)
svg = ratex_py.render_svg(formula)
png = ratex_py.render_png(formula)
pdf = ratex_py.render_pdf(formula)

# Fast: parse once, render 3 times (1 parse)
dl = ratex_py.render_display_list(formula)
svg = ratex_py.render_svg_from_display_list(dl)
png = ratex_py.render_png_from_display_list(dl)
pdf = ratex_py.render_pdf_from_display_list(dl)
```

### Batch Rendering

Render multiple formulas at once to reduce FFI overhead:

```python
# Slow: 100 FFI calls
for formula in formulas:
    svg = ratex_py.render_svg(formula)

# Fast: 1 FFI call
svgs = ratex_py.render_svg_batch(formulas)
```

---

## Color Format Support

Colors are specified as strings in CSS format:

- **Named colors**: `"black"`, `"white"`, `"red"`, `"green"`, `"blue"`, `"transparent"`
- **Hex 6-digit**: `"#ff0000"` (red)
- **Hex 3-digit**: `"#f00"` (red, expanded to `#ff0000`)
- **Hex with alpha**: `"#ff000080"` (semi-transparent red) — PNG only

```python
svg = ratex_py.render_svg(r"\alpha", color="#ff0000")
svg = ratex_py.render_svg(r"\beta", color="blue")
svg = ratex_py.render_svg(r"\gamma", color="#ff0000")
```

---

## DisplayList JSON Protocol v1

The DisplayList JSON schema (simplified):

```json
{
  "version": 1,
  "width": 2.5,
  "height": 1.2,
  "depth": 0.3,
  "items": [
    {
      "type": "glyph",
      "x": 0.1,
      "y": 0.5,
      "font": "KaTeX_Main",
      "glyph": "x",
      "advance": 0.6
    },
    {
      "type": "line",
      "x1": 0.0,
      "y1": 0.8,
      "x2": 1.0,
      "y2": 0.8,
      "thickness": 0.05
    }
  ]
}
```

- **Coordinate system**: x-right, y-down (text baseline)
- **Units**: Em units (relative to font size)
- **Items**: Glyphs, lines, horizontal/vertical rules, nested groups

See [DISPLAYLIST_JSON_PROTOCOL.md](../../../docs/DISPLAYLIST_JSON_PROTOCOL.md) for the full schema.

---

## Error Handling

All functions raise `ValueError` on invalid input:

```python
import ratex_py

try:
    ratex_py.render_svg(r"\left(")  # unmatched delimiter
except ValueError as e:
    print(f"Math error: {e}")
```

---

## Contributing


- Write Tests: `cd RaTeX/crates/ratex-py && cargo test --lib`
- Build: `cd RaTeX/crates/ratex-py && maturin build -r` (release build)
- Goal: 100% branch coverage
- This package was developed as part of the **dsport** project
  (docutils + sphinxdoc port to Rust).
- This package is to be contributed to RaTeX as `RaTeX/crates/ratex-py` and also `/pyproject.toml`

---

## License

ratex-py is licensed under the MIT License (inherited from RaTeX).

KaTeX fonts are licensed under the SIL Open Font License (OFL).
