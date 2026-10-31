//! Contracts at the boundaries between independently maintained layout domains.
use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parse;
use ratex_types::display_item::DisplayItem;
use ratex_types::{Color, MathStyle};

#[test]
fn compound_domains_preserve_inherited_color_in_every_style() {
    let color = Color::new(0.2, 0.4, 0.6, 0.75);
    for style in [MathStyle::Display, MathStyle::Text, MathStyle::Script] {
        for formula in [
            r"\left(\frac{\widehat{x}_i}{\sqrt{y}}\middle|z\right)",
            r"\begin{array}{c|c}\hline x&\overline{y}\\[1em]z&\sqrt{w}\end{array}",
            r"\begin{CD} A @>f>> B \\ @VVgV @AAhA \\ C @= D \end{CD}",
            r"\begin{prooftree}\LeftLabel{L}\AxiomC{P}\UnaryInfC{Q}\end{prooftree}",
        ] {
            let options = LayoutOptions::default().with_style(style).with_color(color);
            let display = to_display_list(&layout(&parse(formula).unwrap(), &options));
            assert!(!display.items.is_empty(), "{formula}");
            for item in display.items {
                let actual = match item {
                    DisplayItem::GlyphPath { color, .. }
                    | DisplayItem::Line { color, .. }
                    | DisplayItem::Rect { color, .. }
                    | DisplayItem::Path { color, .. } => color,
                };
                assert_eq!(actual, color, "{formula}: {style:?}");
            }
        }
    }
}

#[test]
fn explicit_array_tags_survive_row_gap_layout() {
    let plain = r"\begin{align}x&=y\tag{A}\\z&=w\tag{B}\end{align}";
    let spaced = r"\begin{align}x&=y\tag{A}\\[1em]z&=w\tag{B}\end{align}";
    let render =
        |formula| to_display_list(&layout(&parse(formula).unwrap(), &LayoutOptions::default()));
    let before = render(plain);
    let after = render(spaced);
    assert!(after.height + after.depth > before.height + before.depth);
    for character in ['A', 'B'] {
        let count = |items: &[DisplayItem]| {
            items
                .iter()
                .filter(|item| {
                    matches!(item,
            DisplayItem::GlyphPath { char_code, .. } if *char_code == character as u32)
                })
                .count()
        };
        assert_eq!(count(&before.items), 1);
        assert_eq!(count(&after.items), 1);
    }
}

#[test]
fn tag_labels_are_set_as_upright_text() {
    let font_of = |formula: &str, character: char| {
        let display = to_display_list(&layout(&parse(formula).unwrap(), &LayoutOptions::default()));
        display.items.into_iter().find_map(|item| match item {
            DisplayItem::GlyphPath {
                char_code, font, ..
            } if char_code == character as u32 => Some(font),
            _ => None,
        })
    };
    assert_eq!(font_of(r"\tag{A}x", 'A').as_deref(), Some("Main-Regular"));
    assert_eq!(font_of(r"\tag*{A}x", 'A').as_deref(), Some("Main-Regular"));
    assert_eq!(font_of(r"\tag{$x$}y", 'x').as_deref(), Some("Math-Italic"));
}

#[test]
fn multi_digit_auto_equation_numbers_draw_every_digit_upright() {
    // Rows (1)..(10): digits only come from the tags, `0` only from `(10)`.
    let formula = r"\begin{gather}a\\b\\c\\d\\e\\f\\g\\h\\i\\j\end{gather}";
    let display = to_display_list(&layout(&parse(formula).unwrap(), &LayoutOptions::default()));
    let digits: Vec<(char, &str)> = display
        .items
        .iter()
        .filter_map(|item| match item {
            DisplayItem::GlyphPath {
                char_code, font, ..
            } => char::from_u32(*char_code)
                .filter(char::is_ascii_digit)
                .map(|c| (c, font.as_str())),
            _ => None,
        })
        .collect();
    let count = |digit: char| digits.iter().filter(|(c, _)| *c == digit).count();
    assert_eq!(count('1'), 2, "{digits:?}");
    assert_eq!(count('0'), 1, "{digits:?}");
    assert!(digits.iter().all(|(_, font)| *font == "Main-Regular"));
}

#[test]
fn proof_labels_add_ink_without_changing_the_inference_count() {
    let plain = r"\begin{prooftree}\AxiomC{P}\UnaryInfC{Q}\end{prooftree}";
    let labeled =
        r"\begin{prooftree}\LeftLabel{L}\RightLabel{R}\AxiomC{P}\UnaryInfC{Q}\end{prooftree}";
    let render =
        |formula| to_display_list(&layout(&parse(formula).unwrap(), &LayoutOptions::default()));
    let before = render(plain);
    let after = render(labeled);
    let rules = |items: &[DisplayItem]| {
        items
            .iter()
            .filter(|item| matches!(item, DisplayItem::Line { .. }))
            .count()
    };
    assert_eq!(rules(&before.items), 1);
    assert_eq!(rules(&after.items), 1);
    assert!(after.items.len() > before.items.len());
    assert!(after.width > before.width);
}
