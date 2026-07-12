use ratex_svg::SvgOptions;

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
