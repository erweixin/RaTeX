#!/usr/bin/env python3
"""
Integration tests for ratex-py with docutils and Sphinx.

Tests math rendering integration with docutils-based document pipelines.
Run: python tests/test_docutils_integration.py
"""

import sys
import base64
import io
from pathlib import Path

# Add ratex-py to path
sys.path.insert(0, str(Path(__file__).parent.parent / "target" / "debug"))

import ratex_py


def test_math_role_inline():
    """Test inline math role integration."""
    print("Testing inline math role...")
    
    latex = r"\sqrt{2}"
    svg = ratex_py.render_svg_inline(latex, color="black")
    
    assert isinstance(svg, str)
    assert "<svg" in svg
    assert "sqrt" in svg.lower() or "symbol" in svg.lower()
    print(f"  ✓ Inline math: {latex}")


def test_math_role_with_data_uri():
    """Test math role that produces data URIs (for HTML embedding)."""
    print("Testing math role with data URIs...")
    
    latex = r"\alpha + \beta"
    svg = ratex_py.render_svg_inline(latex)
    
    # Create data URI
    data_uri = f"data:image/svg+xml;base64,{base64.b64encode(svg.encode()).decode()}"
    
    assert data_uri.startswith("data:image/svg+xml;base64,")
    assert len(data_uri) > 100
    print(f"  ✓ Data URI created: {data_uri[:80]}...")


def test_math_directive_svg():
    """Test block math directive with SVG output."""
    print("Testing block math directive (SVG)...")
    
    latex = r"\frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"
    svg = ratex_py.render_svg(latex, font_size=32.0, display_mode=True)
    
    assert isinstance(svg, str)
    assert "<svg" in svg
    assert 'viewBox' in svg
    print(f"  ✓ Block math SVG: {len(svg)} chars")


def test_math_directive_png():
    """Test block math directive with PNG output."""
    print("Testing block math directive (PNG)...")
    
    latex = r"E = mc^2"
    png = ratex_py.render_png(latex, font_size=40.0, display_mode=True)
    
    assert isinstance(png, bytes)
    assert png.startswith(b'\x89PNG')  # PNG magic bytes
    print(f"  ✓ Block math PNG: {len(png)} bytes")


def test_math_directive_pdf():
    """Test block math directive with PDF output."""
    print("Testing block math directive (PDF)...")
    
    latex = r"\int_0^\infty e^{-x^2} dx"
    pdf = ratex_py.render_pdf(latex, font_size=40.0, display_mode=True)
    
    assert isinstance(pdf, bytes)
    assert pdf.startswith(b'%PDF-1')
    print(f"  ✓ Block math PDF: {len(pdf)} bytes")


def test_display_list_caching():
    """Test DisplayList caching for multi-format output."""
    print("Testing DisplayList caching...")
    
    formulas = [
        r"\alpha + \beta",
        r"\int_0^\infty e^{-x^2} dx",
        r"\frac{1}{\sqrt{2\pi}} e^{-\frac{x^2}{2}}",
    ]
    
    # Parse once per formula
    display_lists = {}
    for formula in formulas:
        dl_json = ratex_py.render_display_list(formula)
        display_lists[formula] = dl_json
        
        # Verify it's valid JSON
        import json
        dl = json.loads(dl_json)
        assert "version" in dl
        assert dl["version"] == 1
        assert "width" in dl
        assert "height" in dl
    
    # Render to multiple formats from cached lists
    for formula, dl_json in display_lists.items():
        svg = ratex_py.render_svg_from_display_list(dl_json, embed_glyphs=True)
        png = ratex_py.render_png_from_display_list(dl_json)
        pdf = ratex_py.render_pdf_from_display_list(dl_json)
        
        assert "<svg" in svg
        assert png.startswith(b'\x89PNG')
        assert pdf.startswith(b'%PDF-1')
    
    print(f"  ✓ Cached {len(display_lists)} formulas in 3 formats")


def test_batch_rendering():
    """Test batch rendering for document-scale rendering."""
    print("Testing batch rendering...")
    
    formulas = [
        r"\alpha",
        r"\beta",
        r"\gamma",
        r"\delta",
        r"\epsilon",
    ]
    
    # Batch SVG rendering
    svgs = ratex_py.render_svg_batch(
        formulas,
        font_size=40.0,
        display_mode=False,
        color="black",
        embed_glyphs=True,
    )
    
    assert len(svgs) == len(formulas)
    for svg in svgs:
        assert isinstance(svg, str)
        assert "<svg" in svg
    
    # Batch PNG rendering
    pngs = ratex_py.render_png_batch(
        formulas,
        font_size=40.0,
        display_mode=False,
        color="black",
        background_color="white",
        dpr=1.0,
    )
    
    assert len(pngs) == len(formulas)
    for png in pngs:
        assert isinstance(png, bytes)
        assert png.startswith(b'\x89PNG')
    
    print(f"  ✓ Batch rendered {len(formulas)} formulas")


def test_error_handling():
    """Test error handling for invalid math."""
    print("Testing error handling...")
    
    # Valid formulas should not raise
    ratex_py.check(r"\alpha")
    ratex_py.check(r"\frac{1}{2}")
    print("  ✓ Valid formulas accepted")
    
    # Invalid formulas should raise
    invalid_formulas = [
        r"\left(",  # unmatched delimiter
        r"\\",  # incomplete command
    ]
    
    for formula in invalid_formulas:
        try:
            ratex_py.check(formula)
            assert False, f"Should have raised for: {formula}"
        except ValueError as e:
            print(f"  ✓ Error for '{formula}': {str(e)[:50]}...")


def test_color_support():
    """Test color support for themed documents."""
    print("Testing color support...")
    
    formula = r"\alpha + \beta"
    
    # Test named colors
    for color in ["black", "white", "red", "blue", "green"]:
        try:
            svg = ratex_py.render_svg(formula, color=color)
            assert "<svg" in svg
            print(f"  ✓ Color '{color}' works")
        except ValueError:
            print(f"  ⚠ Color '{color}' not supported")
    
    # Test hex colors
    for color in ["#000000", "#ffffff", "#ff0000"]:
        svg = ratex_py.render_svg(formula, color=color)
        assert "<svg" in svg
    
    print(f"  ✓ Hex colors work")


def test_high_dpi_rendering():
    """Test high-DPI rendering for print/export."""
    print("Testing high-DPI rendering...")
    
    formula = r"E = mc^2"
    
    # Standard DPI (1.0)
    png_1x = ratex_py.render_png(formula, dpr=1.0)
    assert png_1x.startswith(b'\x89PNG')
    
    # 2x DPI
    png_2x = ratex_py.render_png(formula, dpr=2.0)
    assert png_2x.startswith(b'\x89PNG')
    
    # 2x should be larger (roughly 4x for same quality)
    assert len(png_2x) > len(png_1x)
    
    print(f"  ✓ 1x: {len(png_1x)} bytes, 2x: {len(png_2x)} bytes")


def test_sphinx_workflow():
    """Simulate a Sphinx document build workflow."""
    print("Testing Sphinx workflow...")
    
    # Simulate Sphinx rendering different math in a document
    document = {
        "title": "Linear Algebra",
        "sections": [
            {
                "title": "Vectors",
                "math": [
                    r"\vec{v} = (x, y, z)",
                    r"|\vec{v}| = \sqrt{x^2 + y^2 + z^2}",
                ],
            },
            {
                "title": "Matrices",
                "math": [
                    r"A = \begin{pmatrix} a & b \\ c & d \end{pmatrix}",
                ],
            },
        ],
    }
    
    # Collect all formulas
    all_formulas = []
    for section in document["sections"]:
        all_formulas.extend(section["math"])
    
    # Parse all formulas once (per-document cache)
    display_lists = {}
    for formula in all_formulas:
        display_lists[formula] = ratex_py.render_display_list(formula)
    
    # Render for HTML
    html_images = []
    for formula in all_formulas:
        dl = display_lists[formula]
        svg = ratex_py.render_svg_from_display_list(dl, embed_glyphs=True)
        html_images.append(svg)
        assert "<svg" in svg
    
    # Render for PDF
    pdf_images = []
    for formula in all_formulas:
        dl = display_lists[formula]
        pdf = ratex_py.render_pdf_from_display_list(dl)
        pdf_images.append(pdf)
        assert pdf.startswith(b'%PDF-1')
    
    print(f"  ✓ Sphinx workflow: {len(document['sections'])} sections, {len(all_formulas)} formulas")
    print(f"    - HTML: {len(html_images)} SVG images")
    print(f"    - PDF: {len(pdf_images)} PDF images")


def test_docutils_role_api():
    """Test docutils role API simulation."""
    print("Testing docutils role API...")
    
    def create_math_role(font_size=40.0):
        """Create a docutils math role."""
        def math_role(latex_expr):
            try:
                svg = ratex_py.render_svg_inline(latex_expr, font_size=font_size)
                return {
                    "type": "image",
                    "source": f"data:image/svg+xml;base64,{base64.b64encode(svg.encode()).decode()}",
                    "alt": latex_expr,
                }
            except ValueError as e:
                return {
                    "type": "error",
                    "message": str(e),
                }
        return math_role
    
    # Create a math role
    math = create_math_role(font_size=32.0)
    
    # Test valid math
    result = math(r"\sqrt{2}")
    assert result["type"] == "image"
    assert result["source"].startswith("data:image/svg+xml;base64,")
    
    # Test invalid math
    error = math(r"\left(")
    assert error["type"] == "error"
    
    print(f"  ✓ Docutils role API works")


def test_docutils_directive_api():
    """Test docutils directive API simulation."""
    print("Testing docutils directive API...")
    
    class MathDirective:
        def __init__(self, content, font_size=40.0, format="svg"):
            self.content = content
            self.font_size = font_size
            self.format = format
        
        def render(self):
            try:
                if self.format == "svg":
                    return ratex_py.render_svg(
                        self.content,
                        font_size=self.font_size,
                        display_mode=True,
                    )
                elif self.format == "png":
                    return ratex_py.render_png(
                        self.content,
                        font_size=self.font_size,
                        display_mode=True,
                    )
                elif self.format == "pdf":
                    return ratex_py.render_pdf(
                        self.content,
                        font_size=self.font_size,
                        display_mode=True,
                    )
            except ValueError as e:
                return None
    
    # Test SVG output
    directive_svg = MathDirective(r"\frac{1}{2}", format="svg")
    svg = directive_svg.render()
    assert isinstance(svg, str)
    assert "<svg" in svg
    
    # Test PNG output
    directive_png = MathDirective(r"x^2", format="png")
    png = directive_png.render()
    assert isinstance(png, bytes)
    assert png.startswith(b'\x89PNG')
    
    # Test PDF output
    directive_pdf = MathDirective(r"\int", format="pdf")
    pdf = directive_pdf.render()
    assert isinstance(pdf, bytes)
    assert pdf.startswith(b'%PDF-1')
    
    print(f"  ✓ Docutils directive API works")


def main():
    """Run all integration tests."""
    print("=" * 60)
    print("ratex-py Docutils & Sphinx Integration Tests")
    print("=" * 60)
    
    tests = [
        test_math_role_inline,
        test_math_role_with_data_uri,
        test_math_directive_svg,
        test_math_directive_png,
        test_math_directive_pdf,
        test_display_list_caching,
        test_batch_rendering,
        test_error_handling,
        test_color_support,
        test_high_dpi_rendering,
        test_sphinx_workflow,
        test_docutils_role_api,
        test_docutils_directive_api,
    ]
    
    passed = 0
    failed = 0
    
    for test in tests:
        try:
            test()
            passed += 1
        except Exception as e:
            print(f"  ✗ FAILED: {e}")
            failed += 1
    
    print("=" * 60)
    print(f"Results: {passed} passed, {failed} failed")
    print("=" * 60)
    
    return 0 if failed == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
