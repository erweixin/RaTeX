use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parser::parse;
use ratex_svg::{render_to_svg, SvgOptions};
use ratex_types::display_item::{DisplayItem, DisplayList};

#[test]
fn svg_options_remains_constructible_with_the_original_fields() {
    let opts = SvgOptions {
        font_size: 24.0,
        padding: 4.0,
        stroke_width: 1.0,
        embed_glyphs: false,
        font_dir: String::new(),
    };

    assert_eq!(opts.font_size, 24.0);
}

fn formula_display_list(latex: &str) -> DisplayList {
    let ast = parse(latex).expect("parse formula");
    to_display_list(&layout(&ast, &LayoutOptions::default()))
}

#[test]
fn zero_height_rule_reserves_space_without_painting() {
    let unspaced = formula_display_list("xy");
    let spaced = formula_display_list(r"x\rule{1em}{0pt}y");
    let glyph_x = |list: &DisplayList| {
        list.items
            .iter()
            .find_map(|item| match item {
                DisplayItem::GlyphPath { x, char_code, .. } if *char_code == u32::from('y') => {
                    Some(*x)
                }
                _ => None,
            })
            .expect("find the glyph after the rule")
    };

    assert!((spaced.width - unspaced.width - 1.0).abs() < 1e-9);
    assert!((glyph_x(&spaced) - glyph_x(&unspaced) - 1.0).abs() < 1e-9);

    let svg = render_to_svg(&spaced, &SvgOptions::default());
    assert!(!svg.contains("<rect") && !svg.contains("<line"), "{svg}");
}

#[test]
fn positive_thin_rule_is_still_painted_at_one_pixel() {
    let list = formula_display_list(r"x\rule{1em}{0.01pt}y");
    let svg = render_to_svg(&list, &SvgOptions::default());
    let rect = svg
        .split_once("<rect ")
        .and_then(|(_, tail)| tail.split_once("/>"))
        .map(|(rect, _)| rect)
        .unwrap_or_else(|| panic!("expected a painted rule: {svg}"));

    assert!(rect.contains(r#"width="40" height="1""#), "{rect}");
}
