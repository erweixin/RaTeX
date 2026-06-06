#!/usr/bin/env python3
"""
Integration tests for ratex-py with Sphinx.

Tests math rendering for Sphinx documentation builds.
Run: python tests/test_sphinx_integration.py
"""

import sys
import json
from pathlib import Path

# Add ratex-py to path
sys.path.insert(0, str(Path(__file__).parent.parent / "target" / "debug"))

import ratex_py


def test_sphinx_math_role_inline():
    """Test Sphinx :math:`...` inline role."""
    print("Testing Sphinx inline math role...")
    
    formulas = [
        r"x^2 + y^2 = r^2",
        r"\alpha + \beta",
        r"\sqrt{2}",
    ]
    
    for formula in formulas:
        svg = ratex_py.render_svg_inline(formula)
        assert "<svg" in svg
        print(f"  ✓ {formula}")


def test_sphinx_math_directive():
    """Test Sphinx :math: directive (block math)."""
    print("Testing Sphinx block math directive...")
    
    # Example from Sphinx docs
    latex = r"""
\frac{1 - \sqrt{5}}{2} \approx -0.618
"""
    
    svg = ratex_py.render_svg(latex, display_mode=True)
    assert "<svg" in svg
    print(f"  ✓ Block math directive")


def test_sphinx_math_domain():
    """Test Sphinx math domain."""
    print("Testing Sphinx math domain...")
    
    # Math domain uses :math:`...` role
    role_formula = r"E = mc^2"
    svg = ratex_py.render_svg_inline(role_formula)
    assert "<svg" in svg
    
    # Can also be used in directives
    directive_formula = r"\nabla \cdot \vec{E} = \frac{\rho}{\epsilon_0}"
    svg = ratex_py.render_svg(directive_formula, display_mode=True)
    assert "<svg" in svg
    
    print("  ✓ Math domain")


def test_sphinx_display_mode():
    """Test Sphinx display vs inline modes."""
    print("Testing Sphinx display vs inline modes...")
    
    formula = r"\sum_{i=1}^{n} i = \frac{n(n+1)}{2}"
    
    # Inline (for flow text)
    inline_svg = ratex_py.render_svg_inline(formula)
    assert "<svg" in inline_svg
    
    # Display (for standalone)
    display_svg = ratex_py.render_svg(formula, display_mode=True)
    assert "<svg" in display_svg
    
    # They should be different sizes
    import xml.etree.ElementTree as ET
    
    try:
        inline_elem = ET.fromstring(inline_svg)
        display_elem = ET.fromstring(display_svg)
        
        inline_height = float(inline_elem.get("height", "0"))
        display_height = float(display_elem.get("height", "0"))
        
        print(f"  ✓ Inline height: {inline_height}, Display height: {display_height}")
    except:
        print(f"  ✓ Display vs inline modes work")


def test_sphinx_html_builder():
    """Simulate Sphinx HTML builder workflow."""
    print("Testing Sphinx HTML builder workflow...")
    
    # Math formulas in a Sphinx document
    document = {
        "title": "Mathematical Methods",
        "content": [
            {"type": "text", "text": "Consider the quadratic formula:"},
            {"type": "math_display", "latex": r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"},
            {"type": "text", "text": "For inline math, we use :math:`\\alpha`"},
            {"type": "math_inline", "latex": r"\alpha"},
        ],
    }
    
    # Process each element
    for element in document["content"]:
        if element["type"] == "math_display":
            svg = ratex_py.render_svg(element["latex"], display_mode=True)
            assert "<svg" in svg
            print(f"  ✓ HTML: Rendered display math")
        elif element["type"] == "math_inline":
            svg = ratex_py.render_svg_inline(element["latex"])
            assert "<svg" in svg
            print(f"  ✓ HTML: Rendered inline math")
    
    print("  ✓ HTML builder workflow complete")


def test_sphinx_latex_builder():
    """Simulate Sphinx LaTeX builder workflow (for PDF)."""
    print("Testing Sphinx LaTeX builder workflow...")
    
    # For LaTeX/PDF output, we need PDFs of math
    formulas = [
        r"E = mc^2",
        r"\int_0^\infty e^{-x^2} dx",
        r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}",
    ]
    
    pdfs = []
    for formula in formulas:
        pdf = ratex_py.render_pdf(formula, display_mode=True)
        assert pdf.startswith(b'%PDF-1')
        pdfs.append(pdf)
    
    print(f"  ✓ LaTeX builder: Generated {len(pdfs)} PDF equations")


def test_sphinx_cache_performance():
    """Test caching for Sphinx performance."""
    print("Testing Sphinx caching for performance...")
    
    # Large document with repeated formulas
    formulas = [
        r"\alpha + \beta",
        r"\int_0^\infty e^{-x^2} dx",
        r"\frac{d}{dx}",  # repeated
        r"\alpha + \beta",  # repeated
        r"\sqrt{x}",
    ]
    
    # Strategy 1: Cache DisplayLists
    cache = {}
    cached_renders = []
    
    for formula in formulas:
        if formula not in cache:
            # First encounter: parse and cache
            dl = ratex_py.render_display_list(formula)
            cache[formula] = dl
        
        # Use cached DisplayList for all formats
        dl = cache[formula]
        svg = ratex_py.render_svg_from_display_list(dl)
        cached_renders.append(svg)
    
    # Verify all rendered
    assert len(cached_renders) == len(formulas)
    assert all("<svg" in svg for svg in cached_renders)
    
    # Statistics
    unique_formulas = len(cache)
    print(f"  ✓ Cache: {unique_formulas} unique, {len(formulas)} total")


def test_sphinx_config_customization():
    """Test Sphinx config-level customization."""
    print("Testing Sphinx configuration customization...")
    
    # Simulate Sphinx app config for math rendering
    class MathConfig:
        def __init__(self):
            self.font_size = 32.0
            self.color = "black"
            self.use_svg = True
            self.use_png = False
            self.use_pdf = False
    
    config = MathConfig()
    
    formula = r"\alpha + \beta"
    
    # Render based on config
    if config.use_svg:
        result = ratex_py.render_svg(formula, font_size=config.font_size, color=config.color)
        assert "<svg" in result
        print(f"  ✓ SVG output (font_size={config.font_size})")
    
    if config.use_png:
        result = ratex_py.render_png(formula, font_size=config.font_size)
        assert result.startswith(b'\x89PNG')
        print(f"  ✓ PNG output")
    
    if config.use_pdf:
        result = ratex_py.render_pdf(formula, font_size=config.font_size)
        assert result.startswith(b'%PDF-1')
        print(f"  ✓ PDF output")


def test_sphinx_extension_pattern():
    """Test Sphinx extension pattern for custom math rendering."""
    print("Testing Sphinx extension pattern...")
    
    class MathExtension:
        """Simulated Sphinx extension for math rendering."""
        
        def __init__(self, config):
            self.config = config
            self.render_cache = {}
        
        def render_role(self, name, rawtext, text, lineno):
            """Render :math:`...` role."""
            if text not in self.render_cache:
                try:
                    svg = ratex_py.render_svg_inline(text)
                    self.render_cache[text] = svg
                except ValueError as e:
                    return f"<error>{e}</error>"
            return self.render_cache[text]
        
        def render_directive(self, name, arguments, options, content, lineno):
            """Render .. math:: directive."""
            latex = '\n'.join(content)
            try:
                svg = ratex_py.render_svg(latex, display_mode=True)
                return svg
            except ValueError as e:
                return f"<error>{e}</error>"
        
        def stats(self):
            """Extension statistics."""
            return {"cached_renders": len(self.render_cache)}
    
    # Create extension
    ext = MathExtension(config={})
    
    # Test role rendering
    svg1 = ext.render_role("math", "", r"\alpha", 1)
    svg2 = ext.render_role("math", "", r"\beta", 2)
    svg3 = ext.render_role("math", "", r"\alpha", 3)  # Same as first
    
    assert "<svg" in svg1
    assert "<svg" in svg2
    assert "<svg" in svg3
    
    # Check cache worked
    stats = ext.stats()
    print(f"  ✓ Extension: Rendered 3 roles, cached {stats['cached_renders']} unique")


def test_sphinx_themed_colors():
    """Test Sphinx theme colors integration."""
    print("Testing Sphinx themed colors...")
    
    # Different Sphinx themes have different colors
    themes = {
        "light": {"text_color": "black", "bg_color": "white"},
        "dark": {"text_color": "white", "bg_color": "#1a1a1a"},
        "custom": {"text_color": "#0066cc", "bg_color": "#f5f5f5"},
    }
    
    formula = r"\alpha + \beta"
    
    for theme_name, colors in themes.items():
        # For display, use text color
        svg = ratex_py.render_svg(formula, color=colors["text_color"])
        assert "<svg" in svg
        
        # For PNG, use background color
        png = ratex_py.render_png(
            formula,
            background_color=colors["bg_color"],
        )
        assert png.startswith(b'\x89PNG')
        
        print(f"  ✓ {theme_name} theme")


def test_sphinx_multilingual():
    """Test Sphinx multilingual support (all languages support math via LaTeX)."""
    print("Testing Sphinx multilingual math support...")
    
    # Math is universal across languages
    formulas = [
        (r"\alpha + \beta", "Greek"),
        (r"\sum_{i=1}^{n} i", "Sum notation"),
        (r"\frac{\partial^2 u}{\partial t^2}", "Partial derivatives"),
        (r"\mathbb{R}^n", "Set notation"),
    ]
    
    for latex, description in formulas:
        svg = ratex_py.render_svg(latex, display_mode=True)
        assert "<svg" in svg
        print(f"  ✓ {description}")


def test_sphinx_cross_references():
    """Test Sphinx equation cross-references with math."""
    print("Testing Sphinx equation cross-references...")
    
    # Sphinx numbered equations
    equations = {
        "eq-quadratic": {
            "label": "Quadratic Formula",
            "latex": r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}",
        },
        "eq-gaussian": {
            "label": "Gaussian Distribution",
            "latex": r"f(x) = \frac{1}{\sigma\sqrt{2\pi}} e^{-\frac{(x-\mu)^2}{2\sigma^2}}",
        },
    }
    
    # Render all equations
    for eq_id, eq_data in equations.items():
        svg = ratex_py.render_svg(eq_data["latex"], display_mode=True)
        assert "<svg" in svg
        print(f"  ✓ {eq_id}: {eq_data['label']}")


def test_sphinx_code_blocks_with_math():
    """Test Sphinx code blocks that include math (via comments)."""
    print("Testing Sphinx code-math integration...")
    
    # Some Sphinx extensions allow math in code comments
    code_example = {
        "language": "python",
        "code": "# Solution: x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}",
        "math": r"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}",
    }
    
    # Extract and render math
    svg = ratex_py.render_svg(code_example["math"], display_mode=True)
    assert "<svg" in svg
    print(f"  ✓ Math in code comments")


def main():
    """Run all Sphinx integration tests."""
    print("=" * 60)
    print("ratex-py Sphinx Integration Tests")
    print("=" * 60)
    
    tests = [
        test_sphinx_math_role_inline,
        test_sphinx_math_directive,
        test_sphinx_math_domain,
        test_sphinx_display_mode,
        test_sphinx_html_builder,
        test_sphinx_latex_builder,
        test_sphinx_cache_performance,
        test_sphinx_config_customization,
        test_sphinx_extension_pattern,
        test_sphinx_themed_colors,
        test_sphinx_multilingual,
        test_sphinx_cross_references,
        test_sphinx_code_blocks_with_math,
    ]
    
    passed = 0
    failed = 0
    
    for test in tests:
        try:
            test()
            passed += 1
        except Exception as e:
            print(f"  ✗ FAILED: {e}")
            import traceback
            traceback.print_exc()
            failed += 1
    
    print("=" * 60)
    print(f"Results: {passed} passed, {failed} failed")
    print("=" * 60)
    
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
