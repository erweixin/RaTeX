/// Integration tests comparing ratex-layout box dimensions against KaTeX.
///
/// These expected values were extracted from KaTeX 0.16.38 using
/// `tools/layout_compare/katex_layout.mjs`. They represent the strut
/// ascent (height) and depth in em units for display mode.
///
/// Tolerance: 0.001em (well within the 0.02em threshold from the plan).
use ratex_layout::{layout, LayoutOptions};
use ratex_parser::parser::parse;

const TOLERANCE: f64 = 0.002;

fn check(input: &str, expected_height: f64, expected_depth: f64) {
    let ast = parse(input).unwrap_or_else(|e| panic!("Parse error for `{input}`: {e}"));
    let options = LayoutOptions::default();
    let lbox = layout(&ast, &options);

    let h_diff = (lbox.height - expected_height).abs();
    let d_diff = (lbox.depth - expected_depth).abs();

    assert!(
        h_diff < TOLERANCE,
        "`{input}` height: expected {expected_height:.5}, got {:.5} (Δ={h_diff:.5})",
        lbox.height
    );
    assert!(
        d_diff < TOLERANCE,
        "`{input}` depth: expected {expected_depth:.5}, got {:.5} (Δ={d_diff:.5})",
        lbox.depth
    );
}

#[test]
fn single_char_x() {
    check("x", 0.43056, 0.0);
}

#[test]
fn single_char_uppercase() {
    check("A", 0.68333, 0.0);
}

#[test]
fn single_digit() {
    check("1", 0.64444, 0.0);
}

#[test]
fn binary_op_a_plus_b() {
    check("a+b", 0.69444, 0.08333);
}

#[test]
fn relational_eq() {
    check("a+b=c", 0.69444, 0.08333);
}

#[test]
fn superscript_x_squared() {
    check("x^2", 0.86411, 0.0);
}

#[test]
fn subscript_x_i() {
    check("x_i", 0.43056, 0.15);
}

#[test]
fn both_sup_and_sub() {
    check("x^2_i", 0.86411, 0.247);
}

#[test]
fn fraction_a_over_b() {
    check("\\frac{a}{b}", 1.10756, 0.686);
}

#[test]
fn fraction_1_over_2() {
    check("\\frac{1}{2}", 1.32144, 0.686);
}

#[test]
fn fraction_with_ops() {
    check("\\frac{x+y}{z}", 1.26033, 0.686);
}

#[test]
fn fraction_with_scripts() {
    check("\\frac{a^2}{b^2}", 1.49111, 0.686);
}

#[test]
fn sqrt_x() {
    check("\\sqrt{x}", 0.84916, 0.19084);
}

#[test]
fn sqrt_2() {
    check("\\sqrt{2}", 0.95610, 0.08390);
}

#[test]
fn sqrt_a_plus_b() {
    check("\\sqrt{a+b}", 0.93943, 0.10057);
}

#[test]
fn nested_frac_in_sqrt() {
    check("\\sqrt{\\frac{a}{b}}", 1.54466, 0.89535);
}

#[test]
fn nested_sqrt_in_frac() {
    check("\\frac{\\sqrt{a}}{b}", 1.47728, 0.686);
}

#[test]
fn nested_superscripts() {
    check("x^{x^2}", 1.03688, 0.0);
}

#[test]
fn quadratic_formula() {
    check("\\frac{-b \\pm \\sqrt{b^2-4ac}}{2a}", 1.59044, 0.686);
}

// ============================================================================
// Phase 4.2: Operator layout tests
// ============================================================================

#[test]
fn sum_with_limits() {
    check("\\sum_{i=1}^{n} x_i", 1.7314, 1.3577);
}

#[test]
fn int_nolimits() {
    check("\\int_0^1 f(x)\\,dx", 1.5641, 0.9119);
}

#[test]
fn prod_with_limits() {
    check("\\prod_{i=1}^{n} x_i", 1.7314, 1.3577);
}

#[test]
fn sum_standalone() {
    check("\\sum x", 1.05, 0.55);
}

#[test]
fn int_standalone() {
    check("\\int x", 1.36, 0.8622);
}

#[test]
fn lim_with_limits() {
    check("\\lim_{x\\to 0} \\frac{\\sin x}{x}", 1.3449, 0.7971);
}

#[test]
fn sum_infinity_series() {
    check("\\sum_{n=0}^{\\infty} a_n x^n", 1.7314, 1.3471);
}

#[test]
fn coprod_sub_only() {
    check("\\coprod_{i} A_i", 1.05, 1.3577);
}

#[test]
fn bigcap_sub_only() {
    check("\\bigcap_{i} A_i", 1.05, 1.3577);
}

#[test]
fn bigcup_sub_only() {
    check("\\bigcup_{i} A_i", 1.05, 1.3577);
}

#[test]
fn sin_text_op() {
    check("\\sin x", 0.6679, 0.0);
}

#[test]
fn cos_with_sup() {
    check("\\cos^2 \\theta", 0.8641, 0.0);
}

#[test]
fn det_text_op() {
    check("\\det A", 0.6944, 0.0);
}

#[test]
fn max_text_op() {
    check("\\max S", 0.6833, 0.0);
}

#[test]
fn int_with_explicit_limits() {
    check("\\int\\limits_2^2 3", 2.1922, 1.6582);
}

// ============================================================================
// Phase 4.3: Accent layout tests
// ============================================================================

#[test]
fn accent_hat_x() {
    check("\\hat{x}", 0.78056, 0.0);
}

#[test]
fn accent_bar_a() {
    check("\\bar{a}", 0.78056, 0.0);
}

#[test]
fn accent_tilde_n() {
    check("\\tilde{n}", 0.66786, 0.0);
}

#[test]
fn accent_nested_tilde_x() {
    // KaTeX 0.16.38 strut from tools/layout_compare/katex_layout.mjs
    check("\\tilde{\\tilde{x}}", 0.9047, 0.0);
}

#[test]
fn accent_nested_hat_x() {
    check("\\hat{\\hat{x}}", 0.9579, 0.0);
}

#[test]
fn accent_dot_y() {
    check("\\dot{y}", 0.78056, 0.1944);
}

#[test]
fn accent_ddot_x() {
    check("\\ddot{x}", 0.78056, 0.0);
}

// ============================================================================
// Phase 4.3: Left/Right delimiter tests
// ============================================================================

#[test]
fn left_right_simple() {
    check("\\left( x \\right)", 0.75, 0.25);
}

#[test]
fn left_right_frac() {
    check("\\left( \\frac{a}{b} \\right)", 1.15, 0.686);
}

#[test]
fn left_right_brackets() {
    check("\\left[ x^2 \\right]", 0.8641, 0.35);
}

#[test]
fn left_right_braces() {
    check("\\left\\{ a + b \\right\\}", 0.75, 0.25);
}

#[test]
fn left_right_bars() {
    check("\\left| x \\right|", 0.75, 0.25);
}

#[test]
fn left_right_sum() {
    // RaTeX clamps `\left`/`\right` height to `inner_height + inner_depth` after the
    // TeX formula; KaTeX 0.16.38 `makeLeftRightDelim` does not, so this differs from HTML parity.
    check("\\left( \\sum_{i=1}^{n} x_i \\right)", 1.79453, 1.3577);
}

// ============================================================================
// Phase 4.3: Delimiter sizing tests
// ============================================================================

#[test]
fn bigl_bigr() {
    check("\\bigl( x \\bigr)", 0.85, 0.35);
}

#[test]
fn big_bigl() {
    check("\\Bigl( x \\Bigr)", 1.15, 0.65);
}

#[test]
fn biggl_biggr() {
    check("\\biggl( x \\biggr)", 1.45, 0.95);
}

#[test]
fn biggest_biggl() {
    check("\\Biggl( x \\Biggr)", 1.75, 1.25);
}

// ============================================================================
// Phase 4.3: Array/Matrix tests
// ============================================================================

#[test]
fn pmatrix_2x2() {
    check("\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}", 1.45, 0.95);
}

#[test]
fn bmatrix_identity() {
    check("\\begin{bmatrix} 1 & 0 \\\\ 0 & 1 \\end{bmatrix}", 1.45, 0.95);
}

#[test]
fn matrix_2x2() {
    check("\\begin{matrix} a & b \\\\ c & d \\end{matrix}", 1.45, 0.95);
}

// ============================================================================
// Phase 4.3: Text/Font tests
// ============================================================================

#[test]
fn text_hello() {
    check("\\text{hello}", 0.6944, 0.0);
}

#[test]
fn mathrm_sin() {
    check("\\mathrm{sin}", 0.6679, 0.0);
}

/// `\mathrm{mm^{2}}` (e.g. mhchem `\pu{123 mm2}`): base of superscript must stay roman, not math italic.
#[test]
fn mathrm_mm_squared_both_m_upright() {
    use ratex_font::FontId;
    use ratex_layout::layout_box::{BoxContent, LayoutBox};

    fn collect_m_fonts(lb: &LayoutBox) -> Vec<FontId> {
        let mut v = Vec::new();
        match &lb.content {
            BoxContent::Glyph { font_id, char_code } => {
                if *char_code == 'm' as u32 {
                    v.push(*font_id);
                }
            }
            BoxContent::HBox(children) => {
                for c in children {
                    v.extend(collect_m_fonts(c));
                }
            }
            BoxContent::SupSub { base, sup, sub, .. } => {
                v.extend(collect_m_fonts(base));
                if let Some(s) = sup {
                    v.extend(collect_m_fonts(s));
                }
                if let Some(s) = sub {
                    v.extend(collect_m_fonts(s));
                }
            }
            BoxContent::Scaled { body, .. } => v.extend(collect_m_fonts(body)),
            _ => {}
        }
        v
    }

    let ast = parse(r"\mathrm{mm^{2}}").expect("parse");
    let options = LayoutOptions::default();
    let lbox = layout(&ast, &options);
    let m_fonts = collect_m_fonts(&lbox);
    assert_eq!(m_fonts.len(), 2, "expected exactly two 'm' glyphs");
    assert!(
        m_fonts.iter().all(|&f| f == FontId::MainRegular),
        "both m should be MainRegular, got {m_fonts:?}"
    );
}
