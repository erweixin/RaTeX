/// Comprehensive test suite for ratex-parser, organized by feature area.
/// Tests are designed to validate against KaTeX's parsing behavior.
#[cfg(test)]
mod core_parsing {
    use crate::parser::parse;
    use crate::parse_node::ParseNode;

    // ── Basic characters ─────────────────────────────────

    #[test]
    fn single_letter() {
        let ast = parse("x").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "mathord");
        assert_eq!(ast[0].symbol_text(), Some("x"));
    }

    #[test]
    fn multiple_letters() {
        let ast = parse("abc").unwrap();
        assert_eq!(ast.len(), 3);
        assert!(ast.iter().all(|n| n.type_name() == "mathord"));
    }

    #[test]
    fn digit() {
        let ast = parse("5").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "textord");
        assert_eq!(ast[0].symbol_text(), Some("5"));
    }

    #[test]
    fn empty_input() {
        let ast = parse("").unwrap();
        assert!(ast.is_empty());
    }

    // ── Operators and symbols ────────────────────────────

    #[test]
    fn binary_operator_plus() {
        let ast = parse("a+b").unwrap();
        assert_eq!(ast.len(), 3);
        assert_eq!(ast[1].type_name(), "atom");
        if let ParseNode::Atom { family, text, .. } = &ast[1] {
            assert_eq!(*family, crate::parse_node::AtomFamily::Bin);
            assert_eq!(text, "+");
        }
    }

    #[test]
    fn relation_equals() {
        let ast = parse("a=b").unwrap();
        assert_eq!(ast.len(), 3);
        assert_eq!(ast[1].type_name(), "atom");
        if let ParseNode::Atom { family, .. } = &ast[1] {
            assert_eq!(*family, crate::parse_node::AtomFamily::Rel);
        }
    }

    #[test]
    fn open_paren() {
        let ast = parse("(").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "atom");
        if let ParseNode::Atom { family, .. } = &ast[0] {
            assert_eq!(*family, crate::parse_node::AtomFamily::Open);
        }
    }

    #[test]
    fn close_paren() {
        let ast = parse(")").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::Atom { family, .. } = &ast[0] {
            assert_eq!(*family, crate::parse_node::AtomFamily::Close);
        }
    }

    // ── Grouping ─────────────────────────────────────────

    #[test]
    fn braced_group() {
        let ast = parse("{a+b}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "ordgroup");
        if let ParseNode::OrdGroup { body, .. } = &ast[0] {
            assert_eq!(body.len(), 3);
        }
    }

    #[test]
    fn nested_groups() {
        let ast = parse("{{x}}").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::OrdGroup { body, .. } = &ast[0] {
            assert_eq!(body.len(), 1);
            assert_eq!(body[0].type_name(), "ordgroup");
        }
    }

    #[test]
    fn empty_group() {
        let ast = parse("{}").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::OrdGroup { body, .. } = &ast[0] {
            assert!(body.is_empty());
        }
    }

    // ── Super/subscripts ─────────────────────────────────

    #[test]
    fn superscript() {
        let ast = parse("x^2").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::SupSub { base, sup, sub, .. } = &ast[0] {
            assert!(base.is_some());
            assert!(sup.is_some());
            assert!(sub.is_none());
            assert_eq!(sup.as_ref().unwrap().symbol_text(), Some("2"));
        }
    }

    #[test]
    fn subscript() {
        let ast = parse("a_i").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::SupSub { base, sup, sub, .. } = &ast[0] {
            assert!(base.is_some());
            assert!(sup.is_none());
            assert!(sub.is_some());
        }
    }

    #[test]
    fn both_sup_sub() {
        let ast = parse("x^2_i").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::SupSub { sup, sub, .. } = &ast[0] {
            assert!(sup.is_some());
            assert!(sub.is_some());
        }
    }

    #[test]
    fn sub_then_sup() {
        let ast = parse("x_i^2").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::SupSub { sup, sub, .. } = &ast[0] {
            assert!(sup.is_some());
            assert!(sub.is_some());
        }
    }

    #[test]
    fn grouped_superscript() {
        let ast = parse("x^{2+3}").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::SupSub { sup, .. } = &ast[0] {
            let s = sup.as_ref().unwrap();
            assert_eq!(s.type_name(), "ordgroup");
        }
    }

    #[test]
    fn double_superscript_error() {
        assert!(parse("x^2^3").is_err());
    }

    #[test]
    fn double_subscript_error() {
        assert!(parse("x_2_3").is_err());
    }
}

#[cfg(test)]
mod fractions_and_radicals {
    use crate::parser::parse;
    use crate::parse_node::ParseNode;

    #[test]
    fn simple_frac() {
        let ast = parse("\\frac{a}{b}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "genfrac");
        if let ParseNode::GenFrac { has_bar_line, numer, denom, .. } = &ast[0] {
            assert!(has_bar_line);
            // numer and denom are wrapped in OrdGroup
            if let ParseNode::OrdGroup { body, .. } = numer.as_ref() {
                assert_eq!(body.len(), 1);
                assert_eq!(body[0].symbol_text(), Some("a"));
            }
            if let ParseNode::OrdGroup { body, .. } = denom.as_ref() {
                assert_eq!(body.len(), 1);
                assert_eq!(body[0].symbol_text(), Some("b"));
            }
        }
    }

    #[test]
    fn frac_with_expressions() {
        let ast = parse("\\frac{a^2 + b}{c}").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::GenFrac { numer, .. } = &ast[0] {
            if let ParseNode::OrdGroup { body, .. } = numer.as_ref() {
                assert!(body.len() >= 3); // a^2, +, b (with supsub)
            }
        }
    }

    #[test]
    fn dfrac() {
        let ast = parse("\\dfrac{a}{b}").unwrap();
        // dfrac wraps in styling node
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "styling");
    }

    #[test]
    fn tfrac() {
        let ast = parse("\\tfrac{a}{b}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "styling");
    }

    #[test]
    fn binom() {
        let ast = parse("\\binom{n}{k}").unwrap();
        assert_eq!(ast.len(), 1);
        // binom has delimiters and no bar line
        let check = |node: &ParseNode| {
            if let ParseNode::GenFrac { has_bar_line, left_delim, right_delim, .. } = node {
                assert!(!has_bar_line);
                assert_eq!(left_delim.as_deref(), Some("("));
                assert_eq!(right_delim.as_deref(), Some(")"));
            }
        };
        // Might be wrapped in styling
        if ast[0].type_name() == "genfrac" {
            check(&ast[0]);
        } else if let ParseNode::Styling { body, .. } = &ast[0] {
            check(&body[0]);
        }
    }

    #[test]
    fn sqrt_simple() {
        let ast = parse("\\sqrt{x}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "sqrt");
        if let ParseNode::Sqrt { index, .. } = &ast[0] {
            assert!(index.is_none());
        }
    }

    #[test]
    fn sqrt_with_index() {
        let ast = parse("\\sqrt[3]{x}").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::Sqrt { index, body, .. } = &ast[0] {
            assert!(index.is_some());
            // body is an OrdGroup wrapping x
            assert_eq!(body.type_name(), "ordgroup");
        }
    }

    #[test]
    fn nested_frac_sqrt() {
        let ast = parse("\\frac{\\sqrt{a^2+b^2}}{c}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "genfrac");
        if let ParseNode::GenFrac { numer, .. } = &ast[0] {
            if let ParseNode::OrdGroup { body, .. } = numer.as_ref() {
                assert_eq!(body[0].type_name(), "sqrt");
            }
        }
    }
}

#[cfg(test)]
mod operators {
    use crate::parser::parse;
    use crate::parse_node::ParseNode;

    #[test]
    fn sum_symbol() {
        let ast = parse("\\sum").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "op");
        if let ParseNode::Op { symbol, limits, name, .. } = &ast[0] {
            assert!(symbol);
            assert!(limits);
            assert_eq!(name.as_deref(), Some("\\sum"));
        }
    }

    #[test]
    fn int_symbol() {
        let ast = parse("\\int").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::Op { symbol, limits, .. } = &ast[0] {
            assert!(symbol);
            assert!(!limits); // integrals don't use limits by default
        }
    }

    #[test]
    fn lim_text_op() {
        let ast = parse("\\lim").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::Op { symbol, limits, name, .. } = &ast[0] {
            assert!(!symbol);
            assert!(limits);
            assert_eq!(name.as_deref(), Some("\\lim"));
        }
    }

    #[test]
    fn sin_text_op() {
        let ast = parse("\\sin").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::Op { symbol, limits, .. } = &ast[0] {
            assert!(!symbol);
            assert!(!limits);
        }
    }

    #[test]
    fn sum_with_limits() {
        let ast = parse("\\sum_{i=0}^{n}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "supsub");
        if let ParseNode::SupSub { base, sup, sub, .. } = &ast[0] {
            assert_eq!(base.as_ref().unwrap().type_name(), "op");
            assert!(sup.is_some());
            assert!(sub.is_some());
        }
    }
}

#[cfg(test)]
mod accents_and_fonts {
    use crate::parser::parse;
    use crate::parse_node::ParseNode;

    #[test]
    fn hat_accent() {
        let ast = parse("\\hat{x}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "accent");
        if let ParseNode::Accent { label, is_stretchy, .. } = &ast[0] {
            assert_eq!(label, "\\hat");
            assert_eq!(*is_stretchy, Some(false));
        }
    }

    #[test]
    fn widehat_accent() {
        let ast = parse("\\widehat{ABC}").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::Accent { label, is_stretchy, .. } = &ast[0] {
            assert_eq!(label, "\\widehat");
            assert_eq!(*is_stretchy, Some(true));
        }
    }

    #[test]
    fn mathbf() {
        let ast = parse("\\mathbf{A}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "font");
        if let ParseNode::Font { font, .. } = &ast[0] {
            assert_eq!(font, "mathbf");
        }
    }

    #[test]
    fn mathit() {
        let ast = parse("\\mathit{x}").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::Font { font, .. } = &ast[0] {
            assert_eq!(font, "mathit");
        }
    }

    #[test]
    fn text_function() {
        let ast = parse("\\text{hello}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "text");
        if let ParseNode::Text { body, .. } = &ast[0] {
            assert!(!body.is_empty());
        }
    }
}

#[cfg(test)]
mod delimiters {
    use crate::parser::parse;
    use crate::parse_node::ParseNode;

    #[test]
    fn left_right_parens() {
        let ast = parse("\\left( x \\right)").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "leftright");
        if let ParseNode::LeftRight { left, right, body, .. } = &ast[0] {
            assert_eq!(left, "(");
            assert_eq!(right, ")");
            assert!(!body.is_empty());
        }
    }

    #[test]
    fn left_right_with_frac() {
        let ast = parse("\\left( \\frac{a}{b} \\right)").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::LeftRight { body, .. } = &ast[0] {
            assert_eq!(body[0].type_name(), "genfrac");
        }
    }

    #[test]
    fn right_without_left_error() {
        assert!(parse("\\right)").is_err());
    }
}

#[cfg(test)]
mod colors_and_sizing {
    use crate::parser::parse;
    use crate::parse_node::ParseNode;

    #[test]
    fn textcolor() {
        let ast = parse("\\textcolor{red}{x}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "color");
        if let ParseNode::Color { color, body, .. } = &ast[0] {
            assert_eq!(color, "red");
            assert!(!body.is_empty());
        }
    }

    #[test]
    fn overline() {
        let ast = parse("\\overline{x}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "overline");
    }

    #[test]
    fn underline() {
        let ast = parse("\\underline{x}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "underline");
    }
}

#[cfg(test)]
mod complex_expressions {
    use crate::parser::parse;

    #[test]
    fn quadratic_formula() {
        let ast = parse("\\frac{-b \\pm \\sqrt{b^2-4ac}}{2a}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "genfrac");
    }

    #[test]
    fn euler_identity() {
        let ast = parse("e^{i\\pi} + 1 = 0").unwrap();
        assert!(ast.len() >= 4); // supsub, +, 1, =, 0
    }

    #[test]
    fn sin_squared() {
        let ast = parse("\\sin^2(x) + \\cos^2(x) = 1").unwrap();
        assert!(ast.len() >= 5);
    }

    #[test]
    fn nested_fractions() {
        let ast = parse("\\frac{1}{1+\\frac{1}{x}}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "genfrac");
    }

    #[test]
    fn multiple_superscripts_in_expr() {
        let ast = parse("a^2 + b^2 = c^2").unwrap();
        let supsub_count = ast.iter().filter(|n| n.type_name() == "supsub").count();
        assert_eq!(supsub_count, 3);
    }
}

#[cfg(test)]
mod error_handling {
    use crate::parser::parse;

    #[test]
    fn unclosed_brace() {
        assert!(parse("{x").is_err());
    }

    #[test]
    fn extra_close_brace() {
        assert!(parse("x}").is_err());
    }

    #[test]
    fn missing_frac_arg() {
        assert!(parse("\\frac{a}").is_err());
    }

    #[test]
    fn double_superscript() {
        assert!(parse("x^1^2").is_err());
    }

    #[test]
    fn double_subscript() {
        assert!(parse("x_1_2").is_err());
    }

    #[test]
    fn undefined_command() {
        assert!(parse("\\nonexistentcommand").is_err());
    }
}

#[cfg(test)]
mod json_serialization {
    use crate::parser::parse;

    #[test]
    fn basic_json_output() {
        let ast = parse("x").unwrap();
        let json = serde_json::to_value(&ast).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr[0]["type"], "mathord");
        assert_eq!(arr[0]["mode"], "math");
        assert_eq!(arr[0]["text"], "x");
    }

    #[test]
    fn supsub_json_structure() {
        let ast = parse("x^2").unwrap();
        let json = serde_json::to_value(&ast).unwrap();
        let node = &json[0];
        assert_eq!(node["type"], "supsub");
        assert_eq!(node["base"]["type"], "mathord");
        assert_eq!(node["sup"]["type"], "textord");
        assert_eq!(node["sup"]["text"], "2");
    }

    #[test]
    fn frac_json_structure() {
        let ast = parse("\\frac{a}{b}").unwrap();
        let json = serde_json::to_value(&ast).unwrap();
        let node = &json[0];
        assert_eq!(node["type"], "genfrac");
        assert_eq!(node["hasBarLine"], true);
        assert_eq!(node["numer"]["type"], "ordgroup");
        assert_eq!(node["denom"]["type"], "ordgroup");
    }

    #[test]
    fn atom_json_structure() {
        let ast = parse("+").unwrap();
        let json = serde_json::to_value(&ast).unwrap();
        let node = &json[0];
        assert_eq!(node["type"], "atom");
        assert_eq!(node["family"], "bin");
        assert_eq!(node["text"], "+");
    }
}

// ── Environment tests ────────────────────────────────────────────────────

#[cfg(test)]
mod environments {
    use crate::parse_node::ParseNode;
    use crate::parser::parse;

    #[test]
    fn simple_matrix() {
        let ast = parse("\\begin{matrix} a & b \\\\ c & d \\end{matrix}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "array");
        if let ParseNode::Array { body, .. } = &ast[0] {
            assert_eq!(body.len(), 2);
            assert_eq!(body[0].len(), 2);
            assert_eq!(body[1].len(), 2);
        } else {
            panic!("Expected Array node");
        }
    }

    #[test]
    fn pmatrix_wraps_in_leftright() {
        let ast =
            parse("\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "leftright");
        if let ParseNode::LeftRight {
            left, right, body, ..
        } = &ast[0]
        {
            assert_eq!(left, "(");
            assert_eq!(right, ")");
            assert_eq!(body.len(), 1);
            assert_eq!(body[0].type_name(), "array");
        } else {
            panic!("Expected LeftRight node");
        }
    }

    #[test]
    fn bmatrix_wraps_in_leftright() {
        let ast =
            parse("\\begin{bmatrix} 1 & 2 \\\\ 3 & 4 \\end{bmatrix}").unwrap();
        assert_eq!(ast.len(), 1);
        if let ParseNode::LeftRight {
            left, right, ..
        } = &ast[0]
        {
            assert_eq!(left, "[");
            assert_eq!(right, "]");
        } else {
            panic!("Expected LeftRight node");
        }
    }

    #[test]
    fn vmatrix_wraps_in_leftright() {
        let ast =
            parse("\\begin{vmatrix} a & b \\\\ c & d \\end{vmatrix}").unwrap();
        if let ParseNode::LeftRight {
            left, right, ..
        } = &ast[0]
        {
            assert_eq!(left, "|");
            assert_eq!(right, "|");
        } else {
            panic!("Expected LeftRight node");
        }
    }

    #[test]
    fn big_bmatrix_wraps_in_leftright() {
        let ast =
            parse("\\begin{Bmatrix} a \\\\ b \\end{Bmatrix}").unwrap();
        if let ParseNode::LeftRight {
            left, right, ..
        } = &ast[0]
        {
            assert_eq!(left, "\\{");
            assert_eq!(right, "\\}");
        } else {
            panic!("Expected LeftRight node");
        }
    }

    #[test]
    fn cases_environment() {
        let ast = parse(
            "\\begin{cases} x & \\text{if } x > 0 \\\\ -x & \\text{otherwise} \\end{cases}",
        )
        .unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "leftright");
        if let ParseNode::LeftRight {
            left, right, body, ..
        } = &ast[0]
        {
            assert_eq!(left, "\\{");
            assert_eq!(right, ".");
            assert_eq!(body.len(), 1);
            if let ParseNode::Array { body: rows, .. } = &body[0] {
                assert_eq!(rows.len(), 2);
                assert_eq!(rows[0].len(), 2);
                assert_eq!(rows[1].len(), 2);
            } else {
                panic!("Expected Array inside LeftRight");
            }
        } else {
            panic!("Expected LeftRight node for cases");
        }
    }

    #[test]
    fn align_environment() {
        let ast = parse("\\begin{aligned} x &= 1 \\\\ y &= 2 \\end{aligned}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "array");
        if let ParseNode::Array {
            body,
            col_separation_type,
            add_jot,
            ..
        } = &ast[0]
        {
            assert_eq!(body.len(), 2);
            assert!(add_jot.unwrap_or(false));
            assert_eq!(
                col_separation_type.as_deref(),
                Some("align")
            );
        } else {
            panic!("Expected Array node for aligned");
        }
    }

    #[test]
    fn tag_primitive_parses_argument() {
        let ast = parse("\\tag{1}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "tag");
    }

    #[test]
    fn align_with_tag_strips_tag_and_sets_array_tags() {
        use crate::parse_node::ArrayTag;
        let ast =
            parse("\\begin{aligned} x &= 1 \\tag{1} \\\\ y &= 2 \\end{aligned}").unwrap();
        if let ParseNode::Array { body, tags, .. } = &ast[0] {
            assert_eq!(body.len(), 2);
            let row0_last = &body[0][body[0].len() - 1];
            let inner = match row0_last {
                ParseNode::Styling { body: sb, .. } => &sb[0],
                n => n,
            };
            let ob = match inner {
                ParseNode::OrdGroup { body, .. } => body,
                _ => panic!("expected ordgroup"),
            };
            assert!(!ob.iter().any(|n| n.type_name() == "tag"));
            let tags = tags.as_ref().expect("tags");
            assert_eq!(tags.len(), 2);
            assert!(matches!(&tags[0], ArrayTag::Explicit(v) if !v.is_empty()));
            assert!(matches!(&tags[1], ArrayTag::Auto(false)));
        } else {
            panic!("Expected Array node");
        }
    }

    #[test]
    fn matrix_single_row() {
        let ast = parse("\\begin{matrix} a & b & c \\end{matrix}").unwrap();
        if let ParseNode::Array { body, .. } = &ast[0] {
            assert_eq!(body.len(), 1);
            assert_eq!(body[0].len(), 3);
        } else {
            panic!("Expected Array node");
        }
    }

    #[test]
    fn matrix_3x3() {
        let ast = parse(
            "\\begin{matrix} 1 & 2 & 3 \\\\ 4 & 5 & 6 \\\\ 7 & 8 & 9 \\end{matrix}",
        )
        .unwrap();
        if let ParseNode::Array { body, cols, .. } = &ast[0] {
            assert_eq!(body.len(), 3);
            for row in body {
                assert_eq!(row.len(), 3);
            }
            let cols = cols.as_ref().unwrap();
            assert_eq!(cols.len(), 3);
        } else {
            panic!("Expected Array node");
        }
    }

    #[test]
    fn env_name_mismatch_error() {
        let result = parse("\\begin{matrix} a \\end{pmatrix}");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Mismatch"));
    }

    #[test]
    fn unknown_environment_error() {
        let result = parse("\\begin{foobar} a \\end{foobar}");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No such environment"));
    }

    #[test]
    fn gathered_environment() {
        let ast = parse("\\begin{gathered} a \\\\ b \\\\ c \\end{gathered}").unwrap();
        assert_eq!(ast[0].type_name(), "array");
        if let ParseNode::Array { body, .. } = &ast[0] {
            assert_eq!(body.len(), 3);
            for row in body {
                assert_eq!(row.len(), 1);
            }
        } else {
            panic!("Expected Array node");
        }
    }

    #[test]
    fn smallmatrix_environment() {
        let ast = parse("\\begin{smallmatrix} a & b \\\\ c & d \\end{smallmatrix}").unwrap();
        assert_eq!(ast[0].type_name(), "array");
        if let ParseNode::Array {
            arraystretch,
            col_separation_type,
            body,
            ..
        } = &ast[0]
        {
            assert_eq!(*arraystretch, 0.5);
            assert_eq!(col_separation_type.as_deref(), Some("small"));
            assert_eq!(body.len(), 2);
        } else {
            panic!("Expected Array node");
        }
    }

    #[test]
    fn matrix_json_structure() {
        let ast = parse("\\begin{matrix} a & b \\\\ c & d \\end{matrix}").unwrap();
        let json = serde_json::to_value(&ast).unwrap();
        let node = &json[0];
        assert_eq!(node["type"], "array");
        let body = node["body"].as_array().unwrap();
        assert_eq!(body.len(), 2);
        assert_eq!(body[0].as_array().unwrap().len(), 2);
    }

    #[test]
    fn pmatrix_json_structure() {
        let ast =
            parse("\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}").unwrap();
        let json = serde_json::to_value(&ast).unwrap();
        let node = &json[0];
        assert_eq!(node["type"], "leftright");
        assert_eq!(node["left"], "(");
        assert_eq!(node["right"], ")");
        let inner = &node["body"][0];
        assert_eq!(inner["type"], "array");
    }

    #[test]
    fn cases_json_structure() {
        let ast = parse("\\begin{cases} a \\\\ b \\end{cases}").unwrap();
        let json = serde_json::to_value(&ast).unwrap();
        let node = &json[0];
        assert_eq!(node["type"], "leftright");
        assert_eq!(node["left"], "\\{");
        assert_eq!(node["right"], ".");
    }

    #[test]
    fn nested_frac_in_matrix() {
        let ast = parse(
            "\\begin{pmatrix} \\frac{1}{2} & 0 \\\\ 0 & \\frac{3}{4} \\end{pmatrix}",
        )
        .unwrap();
        assert_eq!(ast[0].type_name(), "leftright");
    }

    #[test]
    fn matrix_with_expressions() {
        let ast = parse(
            "\\begin{bmatrix} a+b & c^2 \\\\ \\sqrt{d} & e_i \\end{bmatrix}",
        )
        .unwrap();
        assert_eq!(ast[0].type_name(), "leftright");
    }

    #[test]
    fn rcases_environment() {
        let ast = parse("\\begin{rcases} a \\\\ b \\end{rcases}").unwrap();
        if let ParseNode::LeftRight {
            left, right, ..
        } = &ast[0]
        {
            assert_eq!(left, ".");
            assert_eq!(right, "\\}");
        } else {
            panic!("Expected LeftRight node for rcases");
        }
    }

    #[test]
    fn vmatrix_double_wraps() {
        let ast =
            parse("\\begin{Vmatrix} a & b \\\\ c & d \\end{Vmatrix}").unwrap();
        if let ParseNode::LeftRight {
            left, right, ..
        } = &ast[0]
        {
            assert_eq!(left, "\\Vert");
            assert_eq!(right, "\\Vert");
        } else {
            panic!("Expected LeftRight node");
        }
    }

    // ── Auto-numbering tests ────────────────────────────────

    #[test]
    fn equation_environment_sets_auto_tag() {
        use crate::parse_node::ArrayTag;
        let ast = parse("\\begin{equation} x=1 \\end{equation}").unwrap();
        assert_eq!(ast[0].type_name(), "array");
        if let ParseNode::Array { tags, .. } = &ast[0] {
            let tags = tags.as_ref().expect("expected auto tags");
            assert_eq!(tags.len(), 1);
            assert!(matches!(&tags[0], ArrayTag::Auto(true)));
        } else {
            panic!("Expected Array node");
        }
    }

    #[test]
    fn equation_star_has_no_auto_tag() {
        let ast = parse("\\begin{equation*} x=1 \\end{equation*}").unwrap();
        if let ParseNode::Array { tags, .. } = &ast[0] {
            assert!(tags.is_none(), "starred equation should not have tags");
        } else {
            panic!("Expected Array node");
        }
    }

    #[test]
    fn align_environment_sets_auto_tags() {
        use crate::parse_node::ArrayTag;
        let ast = parse("\\begin{align} x &= 1 \\\\ y &= 2 \\end{align}").unwrap();
        if let ParseNode::Array { tags, .. } = &ast[0] {
            let tags = tags.as_ref().expect("expected auto tags");
            assert_eq!(tags.len(), 2);
            assert!(matches!(&tags[0], ArrayTag::Auto(true)));
            assert!(matches!(&tags[1], ArrayTag::Auto(true)));
        } else {
            panic!("Expected Array node");
        }
    }

    #[test]
    fn align_star_has_no_auto_tags() {
        let ast = parse("\\begin{align*} x &= 1 \\\\ y &= 2 \\end{align*}").unwrap();
        if let ParseNode::Array { tags, .. } = &ast[0] {
            assert!(tags.is_none(), "starred align should not have tags");
        } else {
            panic!("Expected Array node");
        }
    }

    #[test]
    fn gather_environment_sets_auto_tags() {
        use crate::parse_node::ArrayTag;
        let ast = parse("\\begin{gather} x \\\\ y \\end{gather}").unwrap();
        if let ParseNode::Array { tags, .. } = &ast[0] {
            let tags = tags.as_ref().expect("expected auto tags");
            assert_eq!(tags.len(), 2);
            assert!(matches!(&tags[0], ArrayTag::Auto(true)));
            assert!(matches!(&tags[1], ArrayTag::Auto(true)));
        } else {
            panic!("Expected Array node");
        }
    }

    #[test]
    fn aligned_inner_has_no_auto_tags() {
        let ast = parse("\\begin{aligned} x &= 1 \\end{aligned}").unwrap();
        if let ParseNode::Array { tags, .. } = &ast[0] {
            assert!(tags.is_none(), "aligned should not have auto tags");
        } else {
            panic!("Expected Array node");
        }
    }

    #[test]
    fn equation_with_label_sets_labels_field() {
        let ast =
            parse("\\begin{equation} x=1 \\label{eq:one} \\end{equation}").unwrap();
        if let ParseNode::Array { labels, .. } = &ast[0] {
            let labels = labels.as_ref().expect("expected labels");
            assert_eq!(labels.len(), 1);
            assert_eq!(labels[0], Some("eq:one".to_string()));
        } else {
            panic!("Expected Array node");
        }
    }

    #[test]
    fn notag_suppresses_numbering() {
        use crate::parse_node::ArrayTag;
        let ast =
            parse("\\begin{align} x &= 1 \\notag \\\\ y &= 2 \\end{align}").unwrap();
        if let ParseNode::Array { tags, .. } = &ast[0] {
            let tags = tags.as_ref().expect("expected tags");
            assert_eq!(tags.len(), 2);
            assert!(matches!(&tags[0], ArrayTag::Suppressed));
            assert!(matches!(&tags[1], ArrayTag::Auto(true)));
        } else {
            panic!("Expected Array node");
        }
    }

    #[test]
    fn ref_parses_to_ref_node() {
        let ast = parse("\\ref{eq:one}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "ref");
        if let ParseNode::Ref { label, .. } = &ast[0] {
            assert_eq!(label, "eq:one");
        } else {
            panic!("Expected Ref node");
        }
    }

    #[test]
    fn eqref_parses_to_eqref_node() {
        let ast = parse("\\eqref{eq:one}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "eqref");
        if let ParseNode::EqRef { label, .. } = &ast[0] {
            assert_eq!(label, "eq:one");
        } else {
            panic!("Expected EqRef node");
        }
    }

    #[test]
    fn label_parses_to_label_node() {
        let ast = parse("\\label{eq:one}").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "label");
        if let ParseNode::Label { label, .. } = &ast[0] {
            assert_eq!(label, "eq:one");
        } else {
            panic!("Expected Label node");
        }
    }

    #[test]
    fn notag_parses_to_notag_node() {
        let ast = parse("\\notag").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "notag");
    }

    #[test]
    fn nonumber_parses_to_notag_node() {
        let ast = parse("\\nonumber").unwrap();
        assert_eq!(ast.len(), 1);
        assert_eq!(ast[0].type_name(), "notag");
    }
}
