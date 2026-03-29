use ratex_font::{get_char_metrics, get_global_metrics, FontId};
use ratex_parser::parse_node::{AtomFamily, Mode, ParseNode};
use ratex_types::color::Color;
use ratex_types::math_style::MathStyle;
use ratex_types::path_command::PathCommand;

use crate::hbox::make_hbox;
use crate::layout_box::{BoxContent, LayoutBox};
use crate::layout_options::LayoutOptions;
use crate::spacing::{atom_spacing, mu_to_em, MathClass};
use crate::stacked_delim::make_stacked_delim_if_needed;

/// Main entry point: lay out a list of ParseNodes into a LayoutBox.
pub fn layout(nodes: &[ParseNode], options: &LayoutOptions) -> LayoutBox {
    layout_expression(nodes, options, true)
}

/// KaTeX `binLeftCanceller` / `binRightCanceller` (TeXbook p.442–446, Rules 5–6).
/// Binary operators become ordinary in certain contexts so spacing matches TeX/KaTeX.
fn apply_bin_cancellation(raw: &[Option<MathClass>]) -> Vec<Option<MathClass>> {
    let n = raw.len();
    let mut eff = raw.to_vec();
    for i in 0..n {
        if raw[i] != Some(MathClass::Bin) {
            continue;
        }
        let prev = if i == 0 { None } else { raw[i - 1] };
        let left_cancel = matches!(
            prev,
            None
                | Some(MathClass::Bin)
                | Some(MathClass::Open)
                | Some(MathClass::Rel)
                | Some(MathClass::Op)
                | Some(MathClass::Punct)
        );
        if left_cancel {
            eff[i] = Some(MathClass::Ord);
        }
    }
    for i in 0..n {
        if raw[i] != Some(MathClass::Bin) {
            continue;
        }
        let next = if i + 1 < n { raw[i + 1] } else { None };
        let right_cancel = matches!(
            next,
            None | Some(MathClass::Rel) | Some(MathClass::Close) | Some(MathClass::Punct)
        );
        if right_cancel {
            eff[i] = Some(MathClass::Ord);
        }
    }
    eff
}

/// Lay out an expression (list of nodes) as a horizontal sequence with spacing.
fn layout_expression(
    nodes: &[ParseNode],
    options: &LayoutOptions,
    is_real_group: bool,
) -> LayoutBox {
    if nodes.is_empty() {
        return LayoutBox::new_empty();
    }

    // Check for line breaks (\\, \newline) — split into rows stacked in a VBox
    let has_cr = nodes.iter().any(|n| matches!(n, ParseNode::Cr { .. }));
    if has_cr {
        return layout_multiline(nodes, options, is_real_group);
    }

    let raw_classes: Vec<Option<MathClass>> =
        nodes.iter().map(node_math_class).collect();
    let eff_classes = apply_bin_cancellation(&raw_classes);

    let mut children = Vec::new();
    let mut prev_class: Option<MathClass> = None;

    for (i, node) in nodes.iter().enumerate() {
        let lbox = layout_node(node, options);
        let cur_class = eff_classes.get(i).copied().flatten();

        if is_real_group {
            if let (Some(prev), Some(cur)) = (prev_class, cur_class) {
                let mu = atom_spacing(prev, cur, options.style.is_tight());
                let mu = options
                    .align_relation_spacing
                    .map_or(mu, |cap| mu.min(cap));
                if mu > 0.0 {
                    let em = mu_to_em(mu, options.metrics().quad);
                    children.push(LayoutBox::new_kern(em));
                }
            }
        }

        if cur_class.is_some() {
            prev_class = cur_class;
        }

        children.push(lbox);
    }

    make_hbox(children)
}

/// Layout an expression containing line-break nodes (\\, \newline) as a VBox.
fn layout_multiline(
    nodes: &[ParseNode],
    options: &LayoutOptions,
    is_real_group: bool,
) -> LayoutBox {
    use crate::layout_box::{BoxContent, VBoxChild, VBoxChildKind};
    let metrics = options.metrics();
    let pt = 1.0 / metrics.pt_per_em;
    let baselineskip = 12.0 * pt; // standard TeX baselineskip
    let lineskip = 1.0 * pt; // minimum gap between lines

    // Split nodes at Cr boundaries
    let mut rows: Vec<&[ParseNode]> = Vec::new();
    let mut start = 0;
    for (i, node) in nodes.iter().enumerate() {
        if matches!(node, ParseNode::Cr { .. }) {
            rows.push(&nodes[start..i]);
            start = i + 1;
        }
    }
    rows.push(&nodes[start..]);

    let row_boxes: Vec<LayoutBox> = rows
        .iter()
        .map(|row| layout_expression(row, options, is_real_group))
        .collect();

    let total_width = row_boxes.iter().map(|b| b.width).fold(0.0_f64, f64::max);

    let mut vchildren: Vec<VBoxChild> = Vec::new();
    let mut h = row_boxes.first().map(|b| b.height).unwrap_or(0.0);
    let d = row_boxes.last().map(|b| b.depth).unwrap_or(0.0);
    for (i, row) in row_boxes.iter().enumerate() {
        if i > 0 {
            // TeX baselineskip: gap = baselineskip - prev_depth - cur_height
            let prev_depth = row_boxes[i - 1].depth;
            let gap = (baselineskip - prev_depth - row.height).max(lineskip);
            vchildren.push(VBoxChild { kind: VBoxChildKind::Kern(gap), shift: 0.0 });
            h += gap + row.height + prev_depth;
        }
        vchildren.push(VBoxChild {
            kind: VBoxChildKind::Box(Box::new(row.clone())),
            shift: 0.0,
        });
    }

    LayoutBox {
        width: total_width,
        height: h,
        depth: d,
        content: BoxContent::VBox(vchildren),
        color: options.color,
    }
}


/// Lay out a single ParseNode.
fn layout_node(node: &ParseNode, options: &LayoutOptions) -> LayoutBox {
    match node {
        ParseNode::MathOrd { text, mode, .. } => layout_symbol(text, *mode, options),
        ParseNode::TextOrd { text, mode, .. } => layout_symbol(text, *mode, options),
        ParseNode::Atom { text, mode, .. } => layout_symbol(text, *mode, options),
        ParseNode::OpToken { text, mode, .. } => layout_symbol(text, *mode, options),

        ParseNode::OrdGroup { body, .. } => layout_expression(body, options, true),

        ParseNode::SupSub {
            base, sup, sub, ..
        } => {
            if let Some(base_node) = base.as_deref() {
                if should_use_op_limits(base_node, options) {
                    return layout_op_with_limits(base_node, sup.as_deref(), sub.as_deref(), options);
                }
            }
            layout_supsub(base.as_deref(), sup.as_deref(), sub.as_deref(), options, None)
        }

        ParseNode::GenFrac {
            numer,
            denom,
            has_bar_line,
            bar_size,
            left_delim,
            right_delim,
            ..
        } => {
            let bar_thickness = if *has_bar_line {
                bar_size
                    .as_ref()
                    .map(|m| measurement_to_em(m, options))
                    .unwrap_or(options.metrics().default_rule_thickness)
            } else {
                0.0
            };
            let frac = layout_fraction(numer, denom, bar_thickness, options);

            let has_left = left_delim.as_ref().is_some_and(|d| !d.is_empty() && d != ".");
            let has_right = right_delim.as_ref().is_some_and(|d| !d.is_empty() && d != ".");

            if has_left || has_right {
                let total_h = genfrac_delim_target_height(options);
                let left_d = left_delim.as_deref().unwrap_or(".");
                let right_d = right_delim.as_deref().unwrap_or(".");
                let left_box = make_stretchy_delim(left_d, total_h, options);
                let right_box = make_stretchy_delim(right_d, total_h, options);

                let width = left_box.width + frac.width + right_box.width;
                let height = frac.height.max(left_box.height).max(right_box.height);
                let depth = frac.depth.max(left_box.depth).max(right_box.depth);

                LayoutBox {
                    width,
                    height,
                    depth,
                    content: BoxContent::LeftRight {
                        left: Box::new(left_box),
                        right: Box::new(right_box),
                        inner: Box::new(frac),
                    },
                    color: options.color,
                }
            } else {
                frac
            }
        }

        ParseNode::Sqrt { body, index, .. } => {
            layout_radical(body, index.as_deref(), options)
        }

        ParseNode::Op {
            name,
            symbol,
            body,
            limits,
            suppress_base_shift,
            ..
        } => layout_op(
            name.as_deref(),
            *symbol,
            body.as_deref(),
            *limits,
            suppress_base_shift.unwrap_or(false),
            options,
        ),

        ParseNode::OperatorName { body, .. } => layout_operatorname(body, options),

        ParseNode::SpacingNode { text, .. } => layout_spacing_command(text, options),

        ParseNode::Kern { dimension, .. } => {
            let em = measurement_to_em(dimension, options);
            LayoutBox::new_kern(em)
        }

        ParseNode::Color { color, body, .. } => {
            let new_color = Color::parse(color).unwrap_or(options.color);
            let new_opts = options.with_color(new_color);
            let mut lbox = layout_expression(body, &new_opts, true);
            lbox.color = new_color;
            lbox
        }

        ParseNode::Styling { style, body, .. } => {
            let new_style = match style {
                ratex_parser::parse_node::StyleStr::Display => MathStyle::Display,
                ratex_parser::parse_node::StyleStr::Text => MathStyle::Text,
                ratex_parser::parse_node::StyleStr::Script => MathStyle::Script,
                ratex_parser::parse_node::StyleStr::Scriptscript => MathStyle::ScriptScript,
            };
            let ratio = new_style.size_multiplier() / options.style.size_multiplier();
            let new_opts = options.with_style(new_style);
            let inner = layout_expression(body, &new_opts, true);
            if (ratio - 1.0).abs() < 0.001 {
                inner
            } else {
                LayoutBox {
                    width: inner.width * ratio,
                    height: inner.height * ratio,
                    depth: inner.depth * ratio,
                    content: BoxContent::Scaled {
                        body: Box::new(inner),
                        child_scale: ratio,
                    },
                    color: options.color,
                }
            }
        }

        ParseNode::Accent {
            label, base, is_stretchy, is_shifty, ..
        } => {
            // Some text accents (e.g. \c cedilla) place the mark below
            let is_below = matches!(label.as_str(), "\\c");
            layout_accent(label, base, is_stretchy.unwrap_or(false), is_shifty.unwrap_or(false), is_below, options)
        }

        ParseNode::AccentUnder {
            label, base, is_stretchy, ..
        } => layout_accent(label, base, is_stretchy.unwrap_or(false), false, true, options),

        ParseNode::LeftRight {
            body, left, right, ..
        } => layout_left_right(body, left, right, options),

        ParseNode::DelimSizing {
            size, delim, ..
        } => layout_delim_sizing(*size, delim, options),

        ParseNode::Array {
            body,
            cols,
            arraystretch,
            add_jot,
            row_gaps,
            hlines_before_row,
            col_separation_type,
            hskip_before_and_after,
            ..
        } => layout_array(
            body,
            cols.as_deref(),
            *arraystretch,
            add_jot.unwrap_or(false),
            row_gaps,
            hlines_before_row,
            col_separation_type.as_deref(),
            hskip_before_and_after.unwrap_or(false),
            options,
        ),

        ParseNode::Sizing { size, body, .. } => layout_sizing(*size, body, options),

        ParseNode::Text { body, font, mode, .. } => match font.as_deref() {
            Some(f) => {
                let group = ParseNode::OrdGroup {
                    mode: *mode,
                    body: body.clone(),
                    semisimple: None,
                    loc: None,
                };
                layout_font(f, &group, options)
            }
            None => layout_text(body, options),
        },

        ParseNode::Font { font, body, .. } => layout_font(font, body, options),

        ParseNode::Href { body, .. } => layout_href(body, options),

        ParseNode::Overline { body, .. } => layout_overline(body, options),
        ParseNode::Underline { body, .. } => layout_underline(body, options),

        ParseNode::Rule {
            width: w,
            height: h,
            shift,
            ..
        } => {
            let width = measurement_to_em(w, options);
            let ink_h = measurement_to_em(h, options);
            let raise = shift
                .as_ref()
                .map(|s| measurement_to_em(s, options))
                .unwrap_or(0.0);
            let box_height = (raise + ink_h).max(0.0);
            let box_depth = (-raise).max(0.0);
            LayoutBox::new_rule(width, box_height, box_depth, ink_h, raise)
        }

        ParseNode::Phantom { body, .. } => {
            let inner = layout_expression(body, options, true);
            LayoutBox {
                width: inner.width,
                height: inner.height,
                depth: inner.depth,
                content: BoxContent::Empty,
                color: Color::BLACK,
            }
        }

        ParseNode::VPhantom { body, .. } => {
            let inner = layout_node(body, options);
            LayoutBox {
                width: 0.0,
                height: inner.height,
                depth: inner.depth,
                content: BoxContent::Empty,
                color: Color::BLACK,
            }
        }

        ParseNode::Smash { body, smash_height, smash_depth, .. } => {
            let mut inner = layout_node(body, options);
            if *smash_height { inner.height = 0.0; }
            if *smash_depth { inner.depth = 0.0; }
            inner
        }

        ParseNode::Middle { delim, .. } => {
            match options.leftright_delim_height {
                Some(h) => make_stretchy_delim(delim, h, options),
                None => {
                    // First pass inside \left...\right: reserve width but don't affect inner height.
                    let placeholder = make_stretchy_delim(delim, 1.0, options);
                    LayoutBox {
                        width: placeholder.width,
                        height: 0.0,
                        depth: 0.0,
                        content: BoxContent::Empty,
                        color: options.color,
                    }
                }
            }
        }

        ParseNode::HtmlMathMl { html, .. } => {
            layout_expression(html, options, true)
        }

        ParseNode::MClass { body, .. } => layout_expression(body, options, true),

        ParseNode::MathChoice {
            display, text, script, scriptscript, ..
        } => {
            let branch = match options.style {
                MathStyle::Display | MathStyle::DisplayCramped => display,
                MathStyle::Text | MathStyle::TextCramped => text,
                MathStyle::Script | MathStyle::ScriptCramped => script,
                MathStyle::ScriptScript | MathStyle::ScriptScriptCramped => scriptscript,
            };
            layout_expression(branch, options, true)
        }

        ParseNode::Lap { alignment, body, .. } => {
            let inner = layout_node(body, options);
            let shift = match alignment.as_str() {
                "llap" => -inner.width,
                "clap" => -inner.width / 2.0,
                _ => 0.0, // rlap: no shift
            };
            let mut children = Vec::new();
            if shift != 0.0 {
                children.push(LayoutBox::new_kern(shift));
            }
            let h = inner.height;
            let d = inner.depth;
            children.push(inner);
            LayoutBox {
                width: 0.0,
                height: h,
                depth: d,
                content: BoxContent::HBox(children),
                color: options.color,
            }
        }

        ParseNode::HorizBrace {
            base, is_over, ..
        } => layout_horiz_brace(base, *is_over, options),

        ParseNode::XArrow {
            label, body, below, ..
        } => layout_xarrow(label, body, below.as_deref(), options),

        ParseNode::Pmb { body, .. } => layout_pmb(body, options),

        ParseNode::HBox { body, .. } => layout_text(body, options),

        ParseNode::Enclose { label, background_color, border_color, body, .. } => {
            layout_enclose(label, background_color.as_deref(), border_color.as_deref(), body, options)
        }

        ParseNode::RaiseBox { dy, body, .. } => {
            let shift = measurement_to_em(dy, options);
            layout_raisebox(shift, body, options)
        }

        ParseNode::VCenter { body, .. } => {
            // Vertically center on the math axis
            let inner = layout_node(body, options);
            let axis = options.metrics().axis_height;
            let total = inner.height + inner.depth;
            let height = total / 2.0 + axis;
            let depth = total - height;
            LayoutBox {
                width: inner.width,
                height,
                depth,
                content: inner.content,
                color: inner.color,
            }
        }

        ParseNode::Verb { body, star, .. } => layout_verb(body, *star, options),

        // Fallback for unhandled node types: produce empty box
        _ => LayoutBox::new_empty(),
    }
}

// ============================================================================
// Symbol layout
// ============================================================================

/// Advance width for glyphs missing from bundled KaTeX fonts (e.g. CJK in `\text{…}`).
///
/// The placeholder width must match what system font fallback draws at ~1em: using 0.5em
/// collapses.advance and Core Text / platform rasterizers still paint a full-width ideograph,
/// so neighbors overlap and the row looks "too large" / clipped.
fn missing_glyph_width_em(ch: char) -> f64 {
    match ch as u32 {
        // Hiragana / Katakana
        0x3040..=0x30FF | 0x31F0..=0x31FF => 1.0,
        // CJK Unified + extension / compatibility ideographs
        0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF => 1.0,
        // Hangul syllables
        0xAC00..=0xD7AF => 1.0,
        // Fullwidth ASCII, punctuation, currency
        0xFF01..=0xFF60 | 0xFFE0..=0xFFEE => 1.0,
        _ => 0.5,
    }
}

fn missing_glyph_metrics_fallback(ch: char, options: &LayoutOptions) -> (f64, f64, f64) {
    let m = get_global_metrics(options.style.size_index());
    let w = missing_glyph_width_em(ch);
    if w >= 0.99 {
        let h = (m.quad * 0.92).max(m.x_height);
        (w, h, 0.0)
    } else {
        (w, m.x_height, 0.0)
    }
}

fn layout_symbol(text: &str, mode: Mode, options: &LayoutOptions) -> LayoutBox {
    let ch = resolve_symbol_char(text, mode);

    // Synthetic symbols not present in any KaTeX font; built from SVG paths.
    match ch as u32 {
        0x22B7 => return layout_imageof_origof(true, options),  // \imageof  •—○
        0x22B6 => return layout_imageof_origof(false, options), // \origof   ○—•
        _ => {}
    }

    let mut font_id = select_font(text, ch, mode, options);
    let char_code = ch as u32;

    let mut metrics = get_char_metrics(font_id, char_code);

    if metrics.is_none() && mode == Mode::Math && font_id != FontId::MathItalic {
        if let Some(m) = get_char_metrics(FontId::MathItalic, char_code) {
            font_id = FontId::MathItalic;
            metrics = Some(m);
        }
    }

    let (width, height, depth) = match metrics {
        Some(m) => (m.width, m.height, m.depth),
        None => missing_glyph_metrics_fallback(ch, options),
    };

    LayoutBox {
        width,
        height,
        depth,
        content: BoxContent::Glyph {
            font_id,
            char_code,
        },
        color: options.color,
    }
}

/// Resolve a symbol name to its actual character.
fn resolve_symbol_char(text: &str, mode: Mode) -> char {
    let font_mode = match mode {
        Mode::Math => ratex_font::Mode::Math,
        Mode::Text => ratex_font::Mode::Text,
    };

    if let Some(info) = ratex_font::get_symbol(text, font_mode) {
        if let Some(cp) = info.codepoint {
            return cp;
        }
    }

    text.chars().next().unwrap_or('?')
}

/// Select the font for a math symbol.
/// Uses the symbol table's font field for AMS symbols, and character properties
/// to choose between MathItalic (for letters and Greek) and MainRegular.
fn select_font(text: &str, resolved_char: char, mode: Mode, _options: &LayoutOptions) -> FontId {
    let font_mode = match mode {
        Mode::Math => ratex_font::Mode::Math,
        Mode::Text => ratex_font::Mode::Text,
    };

    if let Some(info) = ratex_font::get_symbol(text, font_mode) {
        if info.font == ratex_font::SymbolFont::Ams {
            return FontId::AmsRegular;
        }
    }

    match mode {
        Mode::Math => {
            if resolved_char.is_ascii_lowercase()
                || resolved_char.is_ascii_uppercase()
                || is_greek_letter(resolved_char)
            {
                FontId::MathItalic
            } else {
                FontId::MainRegular
            }
        }
        Mode::Text => FontId::MainRegular,
    }
}

fn is_greek_letter(ch: char) -> bool {
    matches!(ch,
        '\u{0391}'..='\u{03C9}' |
        '\u{03D1}' | '\u{03D5}' | '\u{03D6}' |
        '\u{03F1}' | '\u{03F5}'
    )
}

fn is_arrow_accent(label: &str) -> bool {
    matches!(
        label,
        "\\overrightarrow"
            | "\\overleftarrow"
            | "\\Overrightarrow"
            | "\\overleftrightarrow"
            | "\\underrightarrow"
            | "\\underleftarrow"
            | "\\underleftrightarrow"
            | "\\overleftharpoon"
            | "\\overrightharpoon"
            | "\\overlinesegment"
            | "\\underlinesegment"
    )
}

// ============================================================================
// Fraction layout (TeX Rule 15d)
// ============================================================================

fn layout_fraction(
    numer: &ParseNode,
    denom: &ParseNode,
    bar_thickness: f64,
    options: &LayoutOptions,
) -> LayoutBox {
    let numer_s = options.style.numerator();
    let denom_s = options.style.denominator();
    let numer_style = options.with_style(numer_s);
    let denom_style = options.with_style(denom_s);

    let numer_box = layout_node(numer, &numer_style);
    let denom_box = layout_node(denom, &denom_style);

    // Size ratios for converting child em to parent em
    let numer_ratio = numer_s.size_multiplier() / options.style.size_multiplier();
    let denom_ratio = denom_s.size_multiplier() / options.style.size_multiplier();

    let numer_height = numer_box.height * numer_ratio;
    let numer_depth = numer_box.depth * numer_ratio;
    let denom_height = denom_box.height * denom_ratio;
    let denom_depth = denom_box.depth * denom_ratio;
    let numer_width = numer_box.width * numer_ratio;
    let denom_width = denom_box.width * denom_ratio;

    let metrics = options.metrics();
    let axis = metrics.axis_height;
    let rule = bar_thickness;

    // TeX Rule 15d: choose shift amounts based on display/text mode
    let (mut num_shift, mut den_shift) = if options.style.is_display() {
        (metrics.num1, metrics.denom1)
    } else if bar_thickness > 0.0 {
        (metrics.num2, metrics.denom2)
    } else {
        (metrics.num3, metrics.denom2)
    };

    if bar_thickness > 0.0 {
        let min_clearance = if options.style.is_display() {
            3.0 * rule
        } else {
            rule
        };

        let num_clearance = (num_shift - numer_depth) - (axis + rule / 2.0);
        if num_clearance < min_clearance {
            num_shift += min_clearance - num_clearance;
        }

        let den_clearance = (axis - rule / 2.0) + (den_shift - denom_height);
        if den_clearance < min_clearance {
            den_shift += min_clearance - den_clearance;
        }
    } else {
        let min_gap = if options.style.is_display() {
            7.0 * metrics.default_rule_thickness
        } else {
            3.0 * metrics.default_rule_thickness
        };

        let gap = (num_shift - numer_depth) - (denom_height - den_shift);
        if gap < min_gap {
            let adjust = (min_gap - gap) / 2.0;
            num_shift += adjust;
            den_shift += adjust;
        }
    }

    let total_width = numer_width.max(denom_width);
    let height = numer_height + num_shift;
    let depth = denom_depth + den_shift;

    LayoutBox {
        width: total_width,
        height,
        depth,
        content: BoxContent::Fraction {
            numer: Box::new(numer_box),
            denom: Box::new(denom_box),
            numer_shift: num_shift,
            denom_shift: den_shift,
            bar_thickness: rule,
            numer_scale: numer_ratio,
            denom_scale: denom_ratio,
        },
        color: options.color,
    }
}

// ============================================================================
// Superscript/Subscript layout
// ============================================================================

fn layout_supsub(
    base: Option<&ParseNode>,
    sup: Option<&ParseNode>,
    sub: Option<&ParseNode>,
    options: &LayoutOptions,
    inherited_font: Option<FontId>,
) -> LayoutBox {
    let layout_child = |n: &ParseNode, opts: &LayoutOptions| match inherited_font {
        Some(fid) => layout_with_font(n, fid, opts),
        None => layout_node(n, opts),
    };

    let horiz_brace_over = matches!(
        base,
        Some(ParseNode::HorizBrace {
            is_over: true,
            ..
        })
    );
    let horiz_brace_under = matches!(
        base,
        Some(ParseNode::HorizBrace {
            is_over: false,
            ..
        })
    );
    let center_scripts = horiz_brace_over || horiz_brace_under;

    let base_box = base
        .map(|b| layout_child(b, options))
        .unwrap_or_else(LayoutBox::new_empty);

    let is_char_box = base.is_some_and(is_character_box);
    let metrics = options.metrics();

    let sup_style = options.style.superscript();
    let sub_style = options.style.subscript();

    let sup_ratio = sup_style.size_multiplier() / options.style.size_multiplier();
    let sub_ratio = sub_style.size_multiplier() / options.style.size_multiplier();

    let sup_box = sup.map(|s| {
        let sup_opts = options.with_style(sup_style);
        layout_child(s, &sup_opts)
    });

    let sub_box = sub.map(|s| {
        let sub_opts = options.with_style(sub_style);
        layout_child(s, &sub_opts)
    });

    let sup_height_scaled = sup_box.as_ref().map(|b| b.height * sup_ratio).unwrap_or(0.0);
    let sup_depth_scaled = sup_box.as_ref().map(|b| b.depth * sup_ratio).unwrap_or(0.0);
    let sub_height_scaled = sub_box.as_ref().map(|b| b.height * sub_ratio).unwrap_or(0.0);
    let sub_depth_scaled = sub_box.as_ref().map(|b| b.depth * sub_ratio).unwrap_or(0.0);

    // KaTeX uses the CHILD style's metrics for supDrop/subDrop, not the parent's
    let sup_style_metrics = get_global_metrics(sup_style.size_index());
    let sub_style_metrics = get_global_metrics(sub_style.size_index());

    // Rule 18a: initial shift from base dimensions
    // For character boxes, supShift/subShift start at 0 (KaTeX behavior)
    let mut sup_shift = if !is_char_box && sup_box.is_some() {
        base_box.height - sup_style_metrics.sup_drop * sup_ratio
    } else {
        0.0
    };

    let mut sub_shift = if !is_char_box && sub_box.is_some() {
        base_box.depth + sub_style_metrics.sub_drop * sub_ratio
    } else {
        0.0
    };

    let min_sup_shift = if options.style.is_cramped() {
        metrics.sup3
    } else if options.style.is_display() {
        metrics.sup1
    } else {
        metrics.sup2
    };

    if sup_box.is_some() && sub_box.is_some() {
        // Rule 18c+e: both sup and sub
        sup_shift = sup_shift
            .max(min_sup_shift)
            .max(sup_depth_scaled + 0.25 * metrics.x_height);
        sub_shift = sub_shift.max(metrics.sub2); // sub2 when both present

        let rule_width = metrics.default_rule_thickness;
        let max_width = 4.0 * rule_width;
        let gap = (sup_shift - sup_depth_scaled) - (sub_height_scaled - sub_shift);
        if gap < max_width {
            sub_shift = max_width - (sup_shift - sup_depth_scaled) + sub_height_scaled;
            let psi = 0.8 * metrics.x_height - (sup_shift - sup_depth_scaled);
            if psi > 0.0 {
                sup_shift += psi;
                sub_shift -= psi;
            }
        }
    } else if sub_box.is_some() {
        // Rule 18b: sub only
        sub_shift = sub_shift
            .max(metrics.sub1)
            .max(sub_height_scaled - 0.8 * metrics.x_height);
    } else if sup_box.is_some() {
        // Rule 18c,d: sup only
        sup_shift = sup_shift
            .max(min_sup_shift)
            .max(sup_depth_scaled + 0.25 * metrics.x_height);
    }

    // `\overbrace{…}^{…}` / `\underbrace{…}_{…}`: default sup_shift = height - sup_drop places
    // the script baseline *inside* tall atoms (by design for single glyphs). For stretchy
    // horizontal braces the label must sit above/below the ink with limit-style clearance.
    if horiz_brace_over && sup_box.is_some() {
        sup_shift += sup_style_metrics.sup_drop * sup_ratio;
        // Same order of gap as `\xrightarrow` labels (`big_op_spacing1` ≈ 2mu); extra +0.3em
        // pushed the script too far above the brace vs KaTeX reference (golden 0603).
        sup_shift += metrics.big_op_spacing1;
    }
    if horiz_brace_under && sub_box.is_some() {
        sub_shift += sub_style_metrics.sub_drop * sub_ratio;
        sub_shift += metrics.big_op_spacing2 + 0.2;
    }

    // Italic correction: KaTeX adds margin-right = italic to italic math characters,
    // so superscripts start at advance_width + italic_correction.
    // Only apply to simple single-glyph bases (not complex sub-expressions).
    let italic_correction = if is_char_box && !center_scripts {
        glyph_italic(&base_box)
    } else {
        0.0
    };

    // Compute total dimensions (using scaled child dimensions)
    let mut height = base_box.height;
    let mut depth = base_box.depth;
    let mut total_width = base_box.width;

    if let Some(ref sup_b) = sup_box {
        height = height.max(sup_shift + sup_height_scaled);
        if center_scripts {
            total_width = total_width.max(sup_b.width * sup_ratio);
        } else {
            total_width = total_width.max(base_box.width + italic_correction + sup_b.width * sup_ratio);
        }
    }
    if let Some(ref sub_b) = sub_box {
        depth = depth.max(sub_shift + sub_depth_scaled);
        if center_scripts {
            total_width = total_width.max(sub_b.width * sub_ratio);
        } else {
            total_width = total_width.max(base_box.width + sub_b.width * sub_ratio);
        }
    }

    LayoutBox {
        width: total_width,
        height,
        depth,
        content: BoxContent::SupSub {
            base: Box::new(base_box),
            sup: sup_box.map(Box::new),
            sub: sub_box.map(Box::new),
            sup_shift,
            sub_shift,
            sup_scale: sup_ratio,
            sub_scale: sub_ratio,
            center_scripts,
            italic_correction,
        },
        color: options.color,
    }
}

// ============================================================================
// Radical (square root) layout
// ============================================================================

fn layout_radical(
    body: &ParseNode,
    index: Option<&ParseNode>,
    options: &LayoutOptions,
) -> LayoutBox {
    let cramped = options.style.cramped();
    let cramped_opts = options.with_style(cramped);
    let mut body_box = layout_node(body, &cramped_opts);

    // Cramped style has same size_multiplier as uncramped
    let body_ratio = cramped.size_multiplier() / options.style.size_multiplier();
    body_box.height *= body_ratio;
    body_box.depth *= body_ratio;
    body_box.width *= body_ratio;

    // Ensure non-zero inner height (KaTeX: if inner.height === 0, use xHeight)
    if body_box.height == 0.0 {
        body_box.height = options.metrics().x_height;
    }

    let metrics = options.metrics();
    let theta = metrics.default_rule_thickness; // 0.04 for textstyle

    // KaTeX sqrt.js: `let phi = theta; if (options.style.id < Style.TEXT.id) phi = xHeight`.
    // Style ids 0–1 are DISPLAY / DISPLAY_CRAMPED; TEXT is id 2. So only display styles use xHeight.
    let phi = if options.style.is_display() {
        metrics.x_height
    } else {
        theta
    };

    let mut line_clearance = theta + phi / 4.0;

    // Minimum delimiter height needed
    let min_delim_height = body_box.height + body_box.depth + line_clearance + theta;

    // Select surd glyph size (simplified: use known breakpoints)
    // KaTeX surd sizes: small=1.0, size1=1.2, size2=1.8, size3=2.4, size4=3.0
    let tex_height = select_surd_height(min_delim_height);
    let rule_width = theta;
    let surd_font = crate::surd::surd_font_for_inner_height(tex_height);
    let advance_width = ratex_font::get_char_metrics(surd_font, 0x221A)
        .map(|m| m.width)
        .unwrap_or(0.833);

    // Check if delimiter is taller than needed → center the extra space
    let delim_depth = tex_height - rule_width;
    if delim_depth > body_box.height + body_box.depth + line_clearance {
        line_clearance =
            (line_clearance + delim_depth - body_box.height - body_box.depth) / 2.0;
    }

    let img_shift = tex_height - body_box.height - line_clearance - rule_width;

    // Compute final box dimensions via vlist logic
    // height = inner.height + lineClearance + 2*ruleWidth when inner.depth=0
    let height = tex_height + rule_width - img_shift;
    let depth = if img_shift > body_box.depth {
        img_shift
    } else {
        body_box.depth
    };

    // Root index (e.g. \sqrt[3]{x}): KaTeX uses SCRIPTSCRIPT (TeX: superscript of superscript).
    const INDEX_KERN: f64 = 0.05;
    let (index_box, index_offset, index_scale) = if let Some(index_node) = index {
        let root_style = options.style.superscript().superscript();
        let root_opts = options.with_style(root_style);
        let idx = layout_node(index_node, &root_opts);
        let index_ratio = root_style.size_multiplier() / options.style.size_multiplier();
        let offset = idx.width * index_ratio + INDEX_KERN;
        (Some(Box::new(idx)), offset, index_ratio)
    } else {
        (None, 0.0, 1.0)
    };

    let width = index_offset + advance_width + body_box.width;

    LayoutBox {
        width,
        height,
        depth,
        content: BoxContent::Radical {
            body: Box::new(body_box),
            index: index_box,
            index_offset,
            index_scale,
            rule_thickness: rule_width,
            inner_height: tex_height,
        },
        color: options.color,
    }
}

/// Select the surd glyph height based on the required minimum delimiter height.
/// KaTeX uses: small(1.0), Size1(1.2), Size2(1.8), Size3(2.4), Size4(3.0).
fn select_surd_height(min_height: f64) -> f64 {
    const SURD_HEIGHTS: [f64; 5] = [1.0, 1.2, 1.8, 2.4, 3.0];
    for &h in &SURD_HEIGHTS {
        if h >= min_height {
            return h;
        }
    }
    // For very tall content, use the largest + stack
    SURD_HEIGHTS[4].max(min_height)
}

// ============================================================================
// Operator layout (TeX Rule 13)
// ============================================================================

const NO_SUCCESSOR: &[&str] = &["\\smallint"];

/// Check if a SupSub's base should use limits (above/below) positioning.
fn should_use_op_limits(base: &ParseNode, options: &LayoutOptions) -> bool {
    match base {
        ParseNode::Op {
            limits,
            always_handle_sup_sub,
            ..
        } => {
            *limits
                && (options.style.is_display()
                    || always_handle_sup_sub.unwrap_or(false))
        }
        ParseNode::OperatorName {
            always_handle_sup_sub,
            limits,
            ..
        } => {
            *always_handle_sup_sub
                && (options.style.is_display() || *limits)
        }
        _ => false,
    }
}

/// Lay out an Op node (without limits — standalone or nolimits mode).
///
/// In KaTeX, baseShift is applied via CSS `position:relative;top:` which
/// does NOT alter the box dimensions. So we return the original glyph
/// dimensions unchanged — the visual shift is handled at render time.
fn layout_op(
    name: Option<&str>,
    symbol: bool,
    body: Option<&[ParseNode]>,
    _limits: bool,
    suppress_base_shift: bool,
    options: &LayoutOptions,
) -> LayoutBox {
    let (mut base_box, _slant) = build_op_base(name, symbol, body, options);

    // Center symbol operators on the math axis (TeX Rule 13a)
    if symbol && !suppress_base_shift {
        let axis = options.metrics().axis_height;
        let _total = base_box.height + base_box.depth;
        let shift = (base_box.height - base_box.depth) / 2.0 - axis;
        if shift.abs() > 0.001 {
            base_box.height -= shift;
            base_box.depth += shift;
        }
    }

    base_box
}

/// Build the base glyph/text for an operator.
/// Returns (base_box, slant) where slant is the italic correction.
fn build_op_base(
    name: Option<&str>,
    symbol: bool,
    body: Option<&[ParseNode]>,
    options: &LayoutOptions,
) -> (LayoutBox, f64) {
    if symbol {
        let large = options.style.is_display()
            && !NO_SUCCESSOR.contains(&name.unwrap_or(""));
        let font_id = if large {
            FontId::Size2Regular
        } else {
            FontId::Size1Regular
        };

        let op_name = name.unwrap_or("");
        let ch = resolve_op_char(op_name);
        let char_code = ch as u32;

        let metrics = get_char_metrics(font_id, char_code);
        let (width, height, depth, italic) = match metrics {
            Some(m) => (m.width, m.height, m.depth, m.italic),
            None => (1.0, 0.75, 0.25, 0.0),
        };
        // Include italic correction in width so limits centered above/below don't overlap
        // the operator's right-side extension (e.g. integral ∫ has non-zero italic).
        let width_with_italic = width + italic;

        let base = LayoutBox {
            width: width_with_italic,
            height,
            depth,
            content: BoxContent::Glyph {
                font_id,
                char_code,
            },
            color: options.color,
        };

        // \oiint and \oiiint: overlay an ellipse on the integral (∬/∭) like \oint’s circle.
        // resolve_op_char already maps them to ∬/∭; add the circle overlay here.
        if op_name == "\\oiint" || op_name == "\\oiiint" {
            let w = base.width;
            let ellipse_commands = ellipse_overlay_path(w, base.height, base.depth);
            let overlay_box = LayoutBox {
                width: w,
                height: base.height,
                depth: base.depth,
                content: BoxContent::SvgPath {
                    commands: ellipse_commands,
                    fill: false,
                },
                color: options.color,
            };
            let with_overlay = make_hbox(vec![base, LayoutBox::new_kern(-w), overlay_box]);
            return (with_overlay, italic);
        }

        (base, italic)
    } else if let Some(body_nodes) = body {
        let base = layout_expression(body_nodes, options, true);
        (base, 0.0)
    } else {
        let base = layout_op_text(name.unwrap_or(""), options);
        (base, 0.0)
    }
}

/// Render a text operator name like \sin, \cos, \lim.
fn layout_op_text(name: &str, options: &LayoutOptions) -> LayoutBox {
    let text = name.strip_prefix('\\').unwrap_or(name);
    let mut children = Vec::new();
    for ch in text.chars() {
        let char_code = ch as u32;
        let metrics = get_char_metrics(FontId::MainRegular, char_code);
        let (width, height, depth) = match metrics {
            Some(m) => (m.width, m.height, m.depth),
            None => (0.5, 0.43, 0.0),
        };
        children.push(LayoutBox {
            width,
            height,
            depth,
            content: BoxContent::Glyph {
                font_id: FontId::MainRegular,
                char_code,
            },
            color: options.color,
        });
    }
    make_hbox(children)
}

/// Compute the vertical shift to center an op symbol on the math axis (Rule 13).
fn compute_op_base_shift(base: &LayoutBox, options: &LayoutOptions) -> f64 {
    let metrics = options.metrics();
    (base.height - base.depth) / 2.0 - metrics.axis_height
}

/// Resolve an op command name to its Unicode character.
fn resolve_op_char(name: &str) -> char {
    // \oiint and \oiiint: use ∬/∭ as base glyph; circle overlay is drawn in build_op_base
    // (same idea as \oint’s circle, but U+222F/U+2230 often missing in math fonts).
    match name {
        "\\oiint"  => return '\u{222C}', // ∬ (double integral)
        "\\oiiint" => return '\u{222D}', // ∭ (triple integral)
        _ => {}
    }
    let font_mode = ratex_font::Mode::Math;
    if let Some(info) = ratex_font::get_symbol(name, font_mode) {
        if let Some(cp) = info.codepoint {
            return cp;
        }
    }
    name.chars().next().unwrap_or('?')
}

/// Lay out an Op with limits above/below (called from SupSub delegation).
fn layout_op_with_limits(
    base_node: &ParseNode,
    sup_node: Option<&ParseNode>,
    sub_node: Option<&ParseNode>,
    options: &LayoutOptions,
) -> LayoutBox {
    let (name, symbol, body, suppress_base_shift) = match base_node {
        ParseNode::Op {
            name,
            symbol,
            body,
            suppress_base_shift,
            ..
        } => (
            name.as_deref(),
            *symbol,
            body.as_deref(),
            suppress_base_shift.unwrap_or(false),
        ),
        ParseNode::OperatorName { body, .. } => (None, false, Some(body.as_slice()), false),
        _ => return layout_supsub(Some(base_node), sup_node, sub_node, options, None),
    };

    let (base_box, slant) = build_op_base(name, symbol, body, options);
    // baseShift only applies to symbol operators (KaTeX: base instanceof SymbolNode)
    let base_shift = if symbol && !suppress_base_shift {
        compute_op_base_shift(&base_box, options)
    } else {
        0.0
    };

    layout_op_limits_inner(&base_box, sup_node, sub_node, slant, base_shift, options)
}

/// Assemble an operator with limits above/below (KaTeX's assembleSupSub).
fn layout_op_limits_inner(
    base: &LayoutBox,
    sup_node: Option<&ParseNode>,
    sub_node: Option<&ParseNode>,
    slant: f64,
    base_shift: f64,
    options: &LayoutOptions,
) -> LayoutBox {
    let metrics = options.metrics();
    let sup_style = options.style.superscript();
    let sub_style = options.style.subscript();

    let sup_ratio = sup_style.size_multiplier() / options.style.size_multiplier();
    let sub_ratio = sub_style.size_multiplier() / options.style.size_multiplier();

    // Extra vertical padding so limits don't sit too close to the operator (e.g. ∫_0^1).
    let extra_clearance = 0.08_f64;

    let sup_data = sup_node.map(|s| {
        let sup_opts = options.with_style(sup_style);
        let elem = layout_node(s, &sup_opts);
        let kern = (metrics.big_op_spacing1 + extra_clearance)
            .max(metrics.big_op_spacing3 - elem.depth * sup_ratio + extra_clearance);
        (elem, kern)
    });

    let sub_data = sub_node.map(|s| {
        let sub_opts = options.with_style(sub_style);
        let elem = layout_node(s, &sub_opts);
        let kern = (metrics.big_op_spacing2 + extra_clearance)
            .max(metrics.big_op_spacing4 - elem.height * sub_ratio + extra_clearance);
        (elem, kern)
    });

    let sp5 = metrics.big_op_spacing5;

    let (total_height, total_depth, total_width) = match (&sup_data, &sub_data) {
        (Some((sup_elem, sup_kern)), Some((sub_elem, sub_kern))) => {
            // Both sup and sub: VList from bottom
            // [sp5, sub, sub_kern, base, sup_kern, sup, sp5]
            let sup_h = sup_elem.height * sup_ratio;
            let sup_d = sup_elem.depth * sup_ratio;
            let sub_h = sub_elem.height * sub_ratio;
            let sub_d = sub_elem.depth * sub_ratio;

            let bottom = sp5 + sub_h + sub_d + sub_kern + base.depth + base_shift;

            let height = bottom
                + base.height - base_shift
                + sup_kern
                + sup_h + sup_d
                + sp5
                - (base.height + base.depth);

            let total_h = base.height - base_shift + sup_kern + sup_h + sup_d + sp5;
            let total_d = bottom;

            let w = base
                .width
                .max(sup_elem.width * sup_ratio)
                .max(sub_elem.width * sub_ratio);
            let _ = height; // suppress unused; we use total_h/total_d
            (total_h, total_d, w)
        }
        (None, Some((sub_elem, sub_kern))) => {
            // Sub only: VList from top
            // [sp5, sub, sub_kern, base]
            let sub_h = sub_elem.height * sub_ratio;
            let sub_d = sub_elem.depth * sub_ratio;

            let total_h = base.height - base_shift;
            let total_d = base.depth + base_shift + sub_kern + sub_h + sub_d + sp5;

            let w = base.width.max(sub_elem.width * sub_ratio);
            (total_h, total_d, w)
        }
        (Some((sup_elem, sup_kern)), None) => {
            // Sup only: VList from bottom
            // [base, sup_kern, sup, sp5]
            let sup_h = sup_elem.height * sup_ratio;
            let sup_d = sup_elem.depth * sup_ratio;

            let total_h =
                base.height - base_shift + sup_kern + sup_h + sup_d + sp5;
            let total_d = base.depth + base_shift;

            let w = base.width.max(sup_elem.width * sup_ratio);
            (total_h, total_d, w)
        }
        (None, None) => {
            return base.clone();
        }
    };

    let sup_kern_val = sup_data.as_ref().map(|(_, k)| *k).unwrap_or(0.0);
    let sub_kern_val = sub_data.as_ref().map(|(_, k)| *k).unwrap_or(0.0);

    LayoutBox {
        width: total_width,
        height: total_height,
        depth: total_depth,
        content: BoxContent::OpLimits {
            base: Box::new(base.clone()),
            sup: sup_data.map(|(elem, _)| Box::new(elem)),
            sub: sub_data.map(|(elem, _)| Box::new(elem)),
            base_shift,
            sup_kern: sup_kern_val,
            sub_kern: sub_kern_val,
            slant,
            sup_scale: sup_ratio,
            sub_scale: sub_ratio,
        },
        color: options.color,
    }
}

/// Lay out \operatorname body as roman text.
fn layout_operatorname(body: &[ParseNode], options: &LayoutOptions) -> LayoutBox {
    let mut children = Vec::new();
    for node in body {
        match node {
            ParseNode::MathOrd { text, .. } | ParseNode::TextOrd { text, .. } => {
                let ch = text.chars().next().unwrap_or('?');
                let char_code = ch as u32;
                let metrics = get_char_metrics(FontId::MainRegular, char_code);
                let (width, height, depth) = match metrics {
                    Some(m) => (m.width, m.height, m.depth),
                    None => (0.5, 0.43, 0.0),
                };
                children.push(LayoutBox {
                    width,
                    height,
                    depth,
                    content: BoxContent::Glyph {
                        font_id: FontId::MainRegular,
                        char_code,
                    },
                    color: options.color,
                });
            }
            _ => {
                children.push(layout_node(node, options));
            }
        }
    }
    make_hbox(children)
}

// ============================================================================
// Accent layout
// ============================================================================

/// `\vec` KaTeX SVG: nudge slightly right to match KaTeX reference.
const VEC_SKEW_EXTRA_RIGHT_EM: f64 = 0.018;

/// Extract the italic correction of the base glyph.
/// Used by superscripts: KaTeX adds margin-right = italic_correction to italic math characters,
/// so the superscript starts at advance_width + italic_correction (not just advance_width).
fn glyph_italic(lb: &LayoutBox) -> f64 {
    match &lb.content {
        BoxContent::Glyph { font_id, char_code } => {
            get_char_metrics(*font_id, *char_code)
                .map(|m| m.italic)
                .unwrap_or(0.0)
        }
        BoxContent::HBox(children) => {
            children.last().map(glyph_italic).unwrap_or(0.0)
        }
        _ => 0.0,
    }
}

/// Extract the skew (italic correction) of the innermost/last glyph in a box.
/// Used by shifty accents (\hat, \tilde…) to horizontally centre the mark
/// over italic math letters (e.g. M in MathItalic has skew ≈ 0.083em).
fn glyph_skew(lb: &LayoutBox) -> f64 {
    match &lb.content {
        BoxContent::Glyph { font_id, char_code } => {
            get_char_metrics(*font_id, *char_code)
                .map(|m| m.skew)
                .unwrap_or(0.0)
        }
        BoxContent::HBox(children) => {
            children.last().map(glyph_skew).unwrap_or(0.0)
        }
        _ => 0.0,
    }
}

fn layout_accent(
    label: &str,
    base: &ParseNode,
    is_stretchy: bool,
    is_shifty: bool,
    is_below: bool,
    options: &LayoutOptions,
) -> LayoutBox {
    let body_box = layout_node(base, options);
    let base_w = body_box.width.max(0.5);

    // Special handling for \textcircled: draw a circle around the content
    if label == "\\textcircled" {
        return layout_textcircled(body_box, options);
    }

    // Try KaTeX exact SVG paths first (widehat, widetilde, overgroup, etc.)
    if let Some((commands, w, h, fill)) =
        crate::katex_svg::katex_accent_path(label, base_w)
    {
        // KaTeX paths use SVG coords (y down): height=0, depth=h
        let accent_box = LayoutBox {
            width: w,
            height: 0.0,
            depth: h,
            content: BoxContent::SvgPath { commands, fill },
            color: options.color,
        };
        // KaTeX `accent.ts` uses `clearance = min(body.height, xHeight)` for ordinary accents.
        // That matches fixed-size `\vec` (svgData.vec); using it for *width-scaled* SVG accents
        // (\widehat, \widetilde, \overgroup, …) pulls the path down onto the base (golden 0604/0885/0886).
        let gap = 0.08;
        let clearance = if is_below {
            body_box.height + body_box.depth + gap
        } else if label == "\\vec" {
            // KaTeX: clearance = min(body.height, xHeight) is used as *overlap* (kern down).
            // Equivalent RaTeX position: vec bottom = body.height - overlap = max(0, body.height - xHeight).
            (body_box.height - options.metrics().x_height).max(0.0)
        } else {
            body_box.height + gap
        };
        let (height, depth) = if is_below {
            (body_box.height, body_box.depth + h + gap)
        } else if label == "\\vec" {
            // Box height = clearance + H_EM, matching KaTeX VList height.
            (clearance + h, body_box.depth)
        } else {
            (body_box.height + gap + h, body_box.depth)
        };
        let vec_skew = if label == "\\vec" {
            (if is_shifty {
                glyph_skew(&body_box)
            } else {
                0.0
            }) + VEC_SKEW_EXTRA_RIGHT_EM
        } else {
            0.0
        };
        return LayoutBox {
            width: body_box.width,
            height,
            depth,
            content: BoxContent::Accent {
                base: Box::new(body_box),
                accent: Box::new(accent_box),
                clearance,
                skew: vec_skew,
                is_below,
            },
            color: options.color,
        };
    }

    // Arrow-type stretchy accents (overrightarrow, etc.)
    let use_arrow_path = is_stretchy && is_arrow_accent(label);

    let accent_box = if use_arrow_path {
        let (commands, arrow_h, fill_arrow) =
            match crate::katex_svg::katex_stretchy_path(label, base_w) {
                Some((c, h)) => (c, h, true),
                None => {
                    let h = 0.3_f64;
                    let c = stretchy_accent_path(label, base_w, h);
                    let fill = label == "\\xtwoheadrightarrow" || label == "\\xtwoheadleftarrow";
                    (c, h, fill)
                }
            };
        LayoutBox {
            width: base_w,
            height: arrow_h / 2.0,
            depth: arrow_h / 2.0,
            content: BoxContent::SvgPath {
                commands,
                fill: fill_arrow,
            },
            color: options.color,
        }
    } else {
        // Try text mode first for text accents (\c, \', \`, etc.), fall back to math
        let accent_char = {
            let ch = resolve_symbol_char(label, Mode::Text);
            if ch == label.chars().next().unwrap_or('?') {
                // Text mode didn't resolve (returned first char of label, likely '\\')
                // so try math mode
                resolve_symbol_char(label, Mode::Math)
            } else {
                ch
            }
        };
        let accent_code = accent_char as u32;
        let accent_metrics = get_char_metrics(FontId::MainRegular, accent_code);
        let (accent_w, accent_h, accent_d) = match accent_metrics {
            Some(m) => (m.width, m.height, m.depth),
            None => (body_box.width, 0.25, 0.0),
        };
        LayoutBox {
            width: accent_w,
            height: accent_h,
            depth: accent_d,
            content: BoxContent::Glyph {
                font_id: FontId::MainRegular,
                char_code: accent_code,
            },
            color: options.color,
        }
    };

    let skew = if use_arrow_path {
        0.0
    } else if is_shifty {
        // For shifty accents (\hat, \tilde, etc.) shift by the BASE character's skew,
        // which encodes the italic correction in math-italic fonts (e.g. M → 0.083em).
        glyph_skew(&body_box)
    } else {
        0.0
    };

    // gap = clearance between body top and bottom of accent SVG.
    // For arrow accents, the SVG path is centered (height=h/2, depth=h/2).
    // The gap prevents the visible arrowhead / harpoon tip from overlapping the base top.
    //
    // KaTeX stretchy arrows with vb_height 522 have h/2 ≈ 0.261em; default gap=0.12 left
    // too little room for tall caps (`\overleftrightarrow{AB}`, `\overleftarrow{AB}`,
    // `\overleftharpoon{AB}`, …).  `\Overrightarrow` uses a taller glyph (vb 560) and keeps
    // the slightly smaller kern used in prior tuning.
    let gap = if use_arrow_path {
        if label == "\\Overrightarrow" {
            0.21
        } else {
            0.26
        }
    } else {
        0.0
    };

    let clearance = if is_below {
        body_box.height + body_box.depth + accent_box.depth + gap
    } else if use_arrow_path {
        body_box.height + gap
    } else {
        // Clearance = how high above baseline the accent is positioned.
        // - For simple letters (M, b, o): body_box.height is the letter top → use directly.
        // - For a body that is itself an above-accent (\r{a} = \aa, \bar{x}, …):
        //   body_box.height = inner_clearance + 0.35 (the 0.35 rendering correction is
        //   already baked in). Using it as outer clearance adds ANOTHER 0.35 on top
        //   (staircase effect), placing hat 0.35em above ring — too spaced.
        //   Instead, read the inner accent's clearance directly from BoxContent and add
        //   a small ε (0.07em ≈ 3px) so the marks don't pixel-overlap in the rasterizer.
        //   This is equivalent to KaTeX's min(body.height, xHeight) approach.
        let base_clearance = match &body_box.content {
            BoxContent::Accent { clearance: inner_cl, is_below, accent: inner_accent, .. }
                if !is_below =>
            {
                // For SVG accents (height≈0, e.g. \vec): body_box.height = clearance + H_EM,
                // which matches KaTeX's body.height. Use min(body.height, xHeight) exactly as
                // KaTeX does: clearance = min(body.height, xHeight).
                // For glyph accents the 0.35 rendering shift is baked into body_box.height,
                // so we use inner_cl + 0.3 to avoid double-counting that correction.
                if inner_accent.height <= 0.001 {
                    // For SVG accents like \vec: KaTeX places the outer glyph accent with
                    // its baseline at body.height - min(body.height, xHeight) above formula
                    // baseline, i.e. max(0, body.height - xHeight).
                    // to_display.rs shifts the glyph DOWN by (accent_h - 0.35.min(accent_h))
                    // so we pre-add that correction to land at the right position.
                    let katex_pos = (body_box.height - options.metrics().x_height).max(0.0);
                    let correction = (accent_box.height - 0.35_f64.min(accent_box.height)).max(0.0);
                    katex_pos + correction
                } else {
                    inner_cl + 0.3
                }
            }
            _ => body_box.height,
        };
        // \bar and \= (macron): add small extra gap so bar distance matches KaTeX reference
        if label == "\\bar" || label == "\\=" {
            base_clearance - 0.2
        } else {
            base_clearance
        }
    };

    let (height, depth) = if is_below {
        (body_box.height, body_box.depth + accent_box.height + accent_box.depth + gap)
    } else if use_arrow_path {
        (body_box.height + gap + accent_box.height, body_box.depth)
    } else {
        // to_display.rs shifts every glyph accent DOWN by max(0, accent.height - 0.35),
        // so the actual visual top of the accent mark = clearance + min(0.35, accent.height).
        // Use this for the layout height so nested accents (e.g. \hat{\r{a}}) see the
        // correct base height instead of the over-estimated clearance + accent.height.
        // For \hat, \bar, \dot, \ddot: also enforce KaTeX's 0.78056em strut so that
        // short bases (x_height ≈ 0.43) produce consistent line spacing.
        const ACCENT_ABOVE_STRUT_HEIGHT_EM: f64 = 0.78056;
        let accent_visual_top = clearance + 0.35_f64.min(accent_box.height);
        let h = if matches!(label, "\\hat" | "\\bar" | "\\=" | "\\dot" | "\\ddot") {
            accent_visual_top.max(ACCENT_ABOVE_STRUT_HEIGHT_EM)
        } else {
            body_box.height.max(accent_visual_top)
        };
        (h, body_box.depth)
    };

    LayoutBox {
        width: body_box.width,
        height,
        depth,
        content: BoxContent::Accent {
            base: Box::new(body_box),
            accent: Box::new(accent_box),
            clearance,
            skew,
            is_below,
        },
        color: options.color,
    }
}

// ============================================================================
// Left/Right stretchy delimiters
// ============================================================================

/// Returns true if the node (or any descendant) is a Middle node.
fn node_contains_middle(node: &ParseNode) -> bool {
    match node {
        ParseNode::Middle { .. } => true,
        ParseNode::OrdGroup { body, .. } | ParseNode::MClass { body, .. } => {
            body.iter().any(node_contains_middle)
        }
        ParseNode::SupSub { base, sup, sub, .. } => {
            base.as_deref().is_some_and(node_contains_middle)
                || sup.as_deref().is_some_and(node_contains_middle)
                || sub.as_deref().is_some_and(node_contains_middle)
        }
        ParseNode::GenFrac { numer, denom, .. } => {
            node_contains_middle(numer) || node_contains_middle(denom)
        }
        ParseNode::Sqrt { body, index, .. } => {
            node_contains_middle(body) || index.as_deref().is_some_and(node_contains_middle)
        }
        ParseNode::Accent { base, .. } | ParseNode::AccentUnder { base, .. } => {
            node_contains_middle(base)
        }
        ParseNode::Op { body, .. } => body
            .as_ref()
            .is_some_and(|b| b.iter().any(node_contains_middle)),
        ParseNode::LeftRight { body, .. } => body.iter().any(node_contains_middle),
        ParseNode::OperatorName { body, .. } => body.iter().any(node_contains_middle),
        ParseNode::Font { body, .. } => node_contains_middle(body),
        ParseNode::Text { body, .. }
        | ParseNode::Color { body, .. }
        | ParseNode::Styling { body, .. }
        | ParseNode::Sizing { body, .. } => body.iter().any(node_contains_middle),
        ParseNode::Overline { body, .. } | ParseNode::Underline { body, .. } => {
            node_contains_middle(body)
        }
        ParseNode::Phantom { body, .. } => body.iter().any(node_contains_middle),
        ParseNode::VPhantom { body, .. } | ParseNode::Smash { body, .. } => {
            node_contains_middle(body)
        }
        ParseNode::Array { body, .. } => body
            .iter()
            .any(|row| row.iter().any(node_contains_middle)),
        ParseNode::Enclose { body, .. }
        | ParseNode::Lap { body, .. }
        | ParseNode::RaiseBox { body, .. }
        | ParseNode::VCenter { body, .. } => node_contains_middle(body),
        ParseNode::Pmb { body, .. } => body.iter().any(node_contains_middle),
        ParseNode::XArrow { body, below, .. } => {
            node_contains_middle(body) || below.as_deref().is_some_and(node_contains_middle)
        }
        ParseNode::MathChoice {
            display,
            text,
            script,
            scriptscript,
            ..
        } => {
            display.iter().any(node_contains_middle)
                || text.iter().any(node_contains_middle)
                || script.iter().any(node_contains_middle)
                || scriptscript.iter().any(node_contains_middle)
        }
        ParseNode::HorizBrace { base, .. } => node_contains_middle(base),
        ParseNode::Href { body, .. } => body.iter().any(node_contains_middle),
        _ => false,
    }
}

/// Returns true if any node in the slice (recursing into all container nodes) is a Middle node.
fn body_contains_middle(nodes: &[ParseNode]) -> bool {
    nodes.iter().any(node_contains_middle)
}

/// KaTeX genfrac HTML Rule 15e: `\binom`, `\brace`, `\brack`, `\atop` use `delim1`/`delim2`
/// from font metrics, not the `\left`/`\right` height formula (`makeLeftRightDelim` vs genfrac).
fn genfrac_delim_target_height(options: &LayoutOptions) -> f64 {
    let m = options.metrics();
    if options.style.is_display() {
        m.delim1
    } else if matches!(
        options.style,
        MathStyle::ScriptScript | MathStyle::ScriptScriptCramped
    ) {
        options
            .with_style(MathStyle::Script)
            .metrics()
            .delim2
    } else {
        m.delim2
    }
}

/// Required total height for `\left`/`\right` stretchy delimiters (TeX `\sigma_4` rule).
fn left_right_delim_total_height(inner: &LayoutBox, options: &LayoutOptions) -> f64 {
    let metrics = options.metrics();
    let inner_height = inner.height;
    let inner_depth = inner.depth;
    let axis = metrics.axis_height;
    let max_dist = (inner_height - axis).max(inner_depth + axis);
    let delim_factor = 901.0;
    let delim_extend = 5.0 / metrics.pt_per_em;
    let from_formula = (max_dist / 500.0 * delim_factor).max(2.0 * max_dist - delim_extend);
    // Ensure delimiter is at least as tall as inner content
    from_formula.max(inner_height + inner_depth)
}

fn layout_left_right(
    body: &[ParseNode],
    left_delim: &str,
    right_delim: &str,
    options: &LayoutOptions,
) -> LayoutBox {
    let (inner, total_height) = if body_contains_middle(body) {
        // First pass: layout with no delim height so \middle doesn't inflate inner size.
        let opts_first = LayoutOptions {
            leftright_delim_height: None,
            ..options.clone()
        };
        let inner_first = layout_expression(body, &opts_first, true);
        let total_height = left_right_delim_total_height(&inner_first, options);
        // Second pass: layout with total_height so \middle stretches to match \left and \right.
        let opts_second = LayoutOptions {
            leftright_delim_height: Some(total_height),
            ..options.clone()
        };
        let inner_second = layout_expression(body, &opts_second, true);
        (inner_second, total_height)
    } else {
        let inner = layout_expression(body, options, true);
        let total_height = left_right_delim_total_height(&inner, options);
        (inner, total_height)
    };

    let inner_height = inner.height;
    let inner_depth = inner.depth;

    let left_box = make_stretchy_delim(left_delim, total_height, options);
    let right_box = make_stretchy_delim(right_delim, total_height, options);

    let width = left_box.width + inner.width + right_box.width;
    let height = left_box.height.max(right_box.height).max(inner_height);
    let depth = left_box.depth.max(right_box.depth).max(inner_depth);

    LayoutBox {
        width,
        height,
        depth,
        content: BoxContent::LeftRight {
            left: Box::new(left_box),
            right: Box::new(right_box),
            inner: Box::new(inner),
        },
        color: options.color,
    }
}

const DELIM_FONT_SEQUENCE: [FontId; 5] = [
    FontId::MainRegular,
    FontId::Size1Regular,
    FontId::Size2Regular,
    FontId::Size3Regular,
    FontId::Size4Regular,
];

/// Normalize angle-bracket delimiter aliases to \langle / \rangle.
fn normalize_delim(delim: &str) -> &str {
    match delim {
        "<" | "\\lt" | "\u{27E8}" => "\\langle",
        ">" | "\\gt" | "\u{27E9}" => "\\rangle",
        _ => delim,
    }
}

/// Return true if delimiter should be rendered as a single vertical bar SVG path.
fn is_vert_delim(delim: &str) -> bool {
    matches!(delim, "|" | "\\vert" | "\\lvert" | "\\rvert")
}

/// Return true if delimiter should be rendered as a double vertical bar SVG path.
fn is_double_vert_delim(delim: &str) -> bool {
    matches!(delim, "\\|" | "\\Vert" | "\\lVert" | "\\rVert")
}

/// Build a vertical-bar delimiter LayoutBox using an SVG filled path.
/// `total_height` is the full height (height+depth) in em.
/// For single vert: viewBoxWidth = 0.333em; for double: 0.556em.
fn make_vert_delim_box(total_height: f64, is_double: bool, options: &LayoutOptions) -> LayoutBox {
    let axis = options.metrics().axis_height;
    let depth = (total_height / 2.0 - axis).max(0.0);
    let height = total_height - depth;
    let width = if is_double { 0.556 } else { 0.333 };

    let commands = if is_double {
        double_vert_delim_path(height, depth)
    } else {
        vert_delim_path(height, depth)
    };

    LayoutBox {
        width,
        height,
        depth,
        content: BoxContent::SvgPath { commands, fill: true },
        color: options.color,
    }
}

/// SVG path for single vertical bar delimiter in RaTeX em coordinates.
/// Baseline at y=0, y-axis points down.
fn vert_delim_path(height: f64, depth: f64) -> Vec<PathCommand> {
    // Thin filled rectangle: x ∈ [0.145, 0.188] em (= 43/1000 em wide)
    let xl = 0.145_f64;
    let xr = 0.188_f64;
    vec![
        PathCommand::MoveTo { x: xl, y: -height },
        PathCommand::LineTo { x: xr, y: -height },
        PathCommand::LineTo { x: xr, y: depth },
        PathCommand::LineTo { x: xl, y: depth },
        PathCommand::Close,
    ]
}

/// SVG path for double vertical bar delimiter in RaTeX em coordinates.
fn double_vert_delim_path(height: f64, depth: f64) -> Vec<PathCommand> {
    let (xl1, xr1) = (0.145_f64, 0.188_f64);
    let (xl2, xr2) = (0.367_f64, 0.410_f64);
    vec![
        PathCommand::MoveTo { x: xl1, y: -height },
        PathCommand::LineTo { x: xr1, y: -height },
        PathCommand::LineTo { x: xr1, y: depth },
        PathCommand::LineTo { x: xl1, y: depth },
        PathCommand::Close,
        PathCommand::MoveTo { x: xl2, y: -height },
        PathCommand::LineTo { x: xr2, y: -height },
        PathCommand::LineTo { x: xr2, y: depth },
        PathCommand::LineTo { x: xl2, y: depth },
        PathCommand::Close,
    ]
}

/// Select a delimiter glyph large enough for the given total height.
fn make_stretchy_delim(delim: &str, total_height: f64, options: &LayoutOptions) -> LayoutBox {
    if delim == "." || delim.is_empty() {
        return LayoutBox::new_kern(0.0);
    }

    // stackAlwaysDelimiters: use SVG path only when the required height exceeds
    // the natural font-glyph height (1.0em for single vert, same for double).
    // When the content is small enough, fall through to the normal font glyph.
    const VERT_NATURAL_HEIGHT: f64 = 1.0; // MainRegular |: 0.75+0.25
    if is_vert_delim(delim) && total_height > VERT_NATURAL_HEIGHT {
        return make_vert_delim_box(total_height, false, options);
    }
    if is_double_vert_delim(delim) && total_height > VERT_NATURAL_HEIGHT {
        return make_vert_delim_box(total_height, true, options);
    }

    // Normalize < > to \langle \rangle for proper angle bracket glyphs
    let delim = normalize_delim(delim);

    let ch = resolve_symbol_char(delim, Mode::Math);
    let char_code = ch as u32;

    let mut best_font = FontId::MainRegular;
    let mut best_w = 0.4;
    let mut best_h = 0.7;
    let mut best_d = 0.2;

    for &font_id in &DELIM_FONT_SEQUENCE {
        if let Some(m) = get_char_metrics(font_id, char_code) {
            best_font = font_id;
            best_w = m.width;
            best_h = m.height;
            best_d = m.depth;
            if best_h + best_d >= total_height {
                break;
            }
        }
    }

    let best_total = best_h + best_d;
    if let Some(stacked) = make_stacked_delim_if_needed(delim, total_height, best_total, options) {
        return stacked;
    }

    LayoutBox {
        width: best_w,
        height: best_h,
        depth: best_d,
        content: BoxContent::Glyph {
            font_id: best_font,
            char_code,
        },
        color: options.color,
    }
}

/// Fixed total heights for \big/\Big/\bigg/\Bigg (sizeToMaxHeight from KaTeX).
const SIZE_TO_MAX_HEIGHT: [f64; 5] = [0.0, 1.2, 1.8, 2.4, 3.0];

/// Layout \big, \Big, \bigg, \Bigg delimiters.
fn layout_delim_sizing(size: u8, delim: &str, options: &LayoutOptions) -> LayoutBox {
    if delim == "." || delim.is_empty() {
        return LayoutBox::new_kern(0.0);
    }

    // stackAlwaysDelimiters: render as SVG path at the fixed size height
    if is_vert_delim(delim) {
        let total = SIZE_TO_MAX_HEIGHT[size.min(4) as usize];
        return make_vert_delim_box(total, false, options);
    }
    if is_double_vert_delim(delim) {
        let total = SIZE_TO_MAX_HEIGHT[size.min(4) as usize];
        return make_vert_delim_box(total, true, options);
    }

    // Normalize angle brackets to proper math angle bracket glyphs
    let delim = normalize_delim(delim);

    let ch = resolve_symbol_char(delim, Mode::Math);
    let char_code = ch as u32;

    let font_id = match size {
        1 => FontId::Size1Regular,
        2 => FontId::Size2Regular,
        3 => FontId::Size3Regular,
        4 => FontId::Size4Regular,
        _ => FontId::Size1Regular,
    };

    let metrics = get_char_metrics(font_id, char_code);
    let (width, height, depth, actual_font) = match metrics {
        Some(m) => (m.width, m.height, m.depth, font_id),
        None => {
            let m = get_char_metrics(FontId::MainRegular, char_code);
            match m {
                Some(m) => (m.width, m.height, m.depth, FontId::MainRegular),
                None => (0.4, 0.7, 0.2, FontId::MainRegular),
            }
        }
    };

    LayoutBox {
        width,
        height,
        depth,
        content: BoxContent::Glyph {
            font_id: actual_font,
            char_code,
        },
        color: options.color,
    }
}

// ============================================================================
// Array / Matrix layout
// ============================================================================

#[allow(clippy::too_many_arguments)]
fn layout_array(
    body: &[Vec<ParseNode>],
    cols: Option<&[ratex_parser::parse_node::AlignSpec]>,
    arraystretch: f64,
    add_jot: bool,
    row_gaps: &[Option<ratex_parser::parse_node::Measurement>],
    hlines: &[Vec<bool>],
    col_sep_type: Option<&str>,
    hskip: bool,
    options: &LayoutOptions,
) -> LayoutBox {
    let metrics = options.metrics();
    let pt = 1.0 / metrics.pt_per_em;
    let baselineskip = 12.0 * pt;
    let jot = 3.0 * pt;
    let arrayskip = arraystretch * baselineskip;
    let arstrut_h = 0.7 * arrayskip;
    let arstrut_d = 0.3 * arrayskip;
    // align/aligned/alignedat: use thin space (3mu) so "x" and "=" are closer,
    // and cap relation spacing in cells to 3mu so spacing before/after "=" is equal.
    const ALIGN_RELATION_MU: f64 = 3.0;
    let col_gap = match col_sep_type {
        Some("align") | Some("alignat") => mu_to_em(ALIGN_RELATION_MU, metrics.quad),
        Some("small") => {
            // smallmatrix: 2 × thickspace × (script_multiplier / current_multiplier)
            // KaTeX: arraycolsep = 0.2778em × (scriptMultiplier / sizeMultiplier)
            2.0 * mu_to_em(5.0, metrics.quad) * MathStyle::Script.size_multiplier()
                / options.size_multiplier()
        }
        _ => 2.0 * 5.0 * pt, // 2 × arraycolsep
    };
    let cell_options = match col_sep_type {
        Some("align") | Some("alignat") => LayoutOptions {
            align_relation_spacing: Some(ALIGN_RELATION_MU),
            ..options.clone()
        },
        _ => options.clone(),
    };

    let num_rows = body.len();
    if num_rows == 0 {
        return LayoutBox::new_empty();
    }

    let num_cols = body.iter().map(|r| r.len()).max().unwrap_or(0);

    // Extract per-column alignment and column separators from cols spec.
    use ratex_parser::parse_node::AlignType;
    let col_aligns: Vec<u8> = {
        let align_specs: Vec<&ratex_parser::parse_node::AlignSpec> = cols
            .map(|cs| {
                cs.iter()
                    .filter(|s| matches!(s.align_type, AlignType::Align))
                    .collect()
            })
            .unwrap_or_default();
        (0..num_cols)
            .map(|c| {
                align_specs
                    .get(c)
                    .and_then(|s| s.align.as_deref())
                    .and_then(|a| a.bytes().next())
                    .unwrap_or(b'c')
            })
            .collect()
    };

    // Detect vertical separator ('|') positions in the column spec.
    // col_separators[i] = true means there is a '|' at boundary i (0 = before col 0, num_cols = after last col).
    let col_separators: Vec<bool> = {
        let mut seps = vec![false; num_cols + 1];
        let mut align_count = 0usize;
        if let Some(cs) = cols {
            for spec in cs {
                match spec.align_type {
                    AlignType::Align => align_count += 1,
                    AlignType::Separator if spec.align.as_deref() == Some("|") => {
                        if align_count <= num_cols {
                            seps[align_count] = true;
                        }
                    }
                    _ => {}
                }
            }
        }
        seps
    };

    let rule_thickness = 0.4 * pt;
    let double_rule_sep = metrics.double_rule_sep;

    // Layout all cells
    let mut cell_boxes: Vec<Vec<LayoutBox>> = Vec::with_capacity(num_rows);
    let mut col_widths = vec![0.0_f64; num_cols];
    let mut row_heights = Vec::with_capacity(num_rows);
    let mut row_depths = Vec::with_capacity(num_rows);

    for row in body {
        let mut row_boxes = Vec::with_capacity(num_cols);
        let mut rh = arstrut_h;
        let mut rd = arstrut_d;

        for (c, cell) in row.iter().enumerate() {
            let cell_nodes = match cell {
                ParseNode::OrdGroup { body, .. } => body.as_slice(),
                other => std::slice::from_ref(other),
            };
            let cell_box = layout_expression(cell_nodes, &cell_options, true);
            rh = rh.max(cell_box.height);
            rd = rd.max(cell_box.depth);
            if c < num_cols {
                col_widths[c] = col_widths[c].max(cell_box.width);
            }
            row_boxes.push(cell_box);
        }

        // Pad missing columns
        while row_boxes.len() < num_cols {
            row_boxes.push(LayoutBox::new_empty());
        }

        if add_jot {
            rd += jot;
        }

        row_heights.push(rh);
        row_depths.push(rd);
        cell_boxes.push(row_boxes);
    }

    // Apply row gaps
    for (r, gap) in row_gaps.iter().enumerate() {
        if r < row_depths.len() {
            if let Some(m) = gap {
                let gap_em = measurement_to_em(m, options);
                if gap_em > 0.0 {
                    row_depths[r] = row_depths[r].max(gap_em + arstrut_d);
                }
            }
        }
    }

    // Ensure hlines_before_row has num_rows + 1 entries.
    let mut hlines_before_row: Vec<Vec<bool>> = hlines.to_vec();
    while hlines_before_row.len() < num_rows + 1 {
        hlines_before_row.push(vec![]);
    }

    // For n > 1 consecutive hlines before row r, add extra vertical space so the
    // lines don't overlap with content.  Each extra line needs (rule_thickness +
    // double_rule_sep) of room.
    //   - r == 0: extra hlines appear above the first row → add to row_heights[0].
    //   - r >= 1: extra hlines appear in the gap above row r → add to row_depths[r-1].
    for r in 0..=num_rows {
        let n = hlines_before_row[r].len();
        if n > 1 {
            let extra = (n - 1) as f64 * (rule_thickness + double_rule_sep);
            if r == 0 {
                if num_rows > 0 {
                    row_heights[0] += extra;
                }
            } else {
                row_depths[r - 1] += extra;
            }
        }
    }

    // Total height and offset (computed after extra hline spacing is applied).
    let mut total_height = 0.0;
    let mut row_positions = Vec::with_capacity(num_rows);
    for r in 0..num_rows {
        total_height += row_heights[r];
        row_positions.push(total_height);
        total_height += row_depths[r];
    }

    let offset = total_height / 2.0 + metrics.axis_height;

    // Extra x padding before col 0 and after last col (hskip_before_and_after).
    let content_x_offset = if hskip { col_gap / 2.0 } else { 0.0 };

    // Total width including outer padding.
    let total_width: f64 = col_widths.iter().sum::<f64>()
        + col_gap * (num_cols.saturating_sub(1)) as f64
        + 2.0 * content_x_offset;

    let height = offset;
    let depth = total_height - offset;

    LayoutBox {
        width: total_width,
        height,
        depth,
        content: BoxContent::Array {
            cells: cell_boxes,
            col_widths: col_widths.clone(),
            col_aligns,
            row_heights: row_heights.clone(),
            row_depths: row_depths.clone(),
            col_gap,
            offset,
            content_x_offset,
            col_separators,
            hlines_before_row,
            rule_thickness,
            double_rule_sep,
        },
        color: options.color,
    }
}

// ============================================================================
// Sizing / Text / Font
// ============================================================================

fn layout_sizing(size: u8, body: &[ParseNode], options: &LayoutOptions) -> LayoutBox {
    // KaTeX sizing: size 1-11, maps to multipliers
    let multiplier = match size {
        1 => 0.5,
        2 => 0.6,
        3 => 0.7,
        4 => 0.8,
        5 => 0.9,
        6 => 1.0,
        7 => 1.2,
        8 => 1.44,
        9 => 1.728,
        10 => 2.074,
        11 => 2.488,
        _ => 1.0,
    };

    let inner = layout_expression(body, options, true);
    let ratio = multiplier / options.size_multiplier();
    if (ratio - 1.0).abs() < 0.001 {
        inner
    } else {
        LayoutBox {
            width: inner.width * ratio,
            height: inner.height * ratio,
            depth: inner.depth * ratio,
            content: BoxContent::Scaled {
                body: Box::new(inner),
                child_scale: ratio,
            },
            color: options.color,
        }
    }
}

/// Layout \verb and \verb* — verbatim text in typewriter font.
/// \verb* shows spaces as a visible character (U+2423 OPEN BOX).
fn layout_verb(body: &str, star: bool, options: &LayoutOptions) -> LayoutBox {
    let metrics = options.metrics();
    let mut children = Vec::new();
    for c in body.chars() {
        let ch = if star && c == ' ' {
            '\u{2423}' // OPEN BOX, visible space
        } else {
            c
        };
        let code = ch as u32;
        let (font_id, w, h, d) = match get_char_metrics(FontId::TypewriterRegular, code) {
            Some(m) => (FontId::TypewriterRegular, m.width, m.height, m.depth),
            None => match get_char_metrics(FontId::MainRegular, code) {
                Some(m) => (FontId::MainRegular, m.width, m.height, m.depth),
                None => (
                    FontId::TypewriterRegular,
                    0.5,
                    metrics.x_height,
                    0.0,
                ),
            },
        };
        children.push(LayoutBox {
            width: w,
            height: h,
            depth: d,
            content: BoxContent::Glyph {
                font_id,
                char_code: code,
            },
            color: options.color,
        });
    }
    let mut hbox = make_hbox(children);
    hbox.color = options.color;
    hbox
}

fn layout_text(body: &[ParseNode], options: &LayoutOptions) -> LayoutBox {
    let mut children = Vec::new();
    for node in body {
        match node {
            ParseNode::TextOrd { text, .. } | ParseNode::MathOrd { text, .. } => {
                let ch = resolve_symbol_char(text, Mode::Text);
                let char_code = ch as u32;
                let m = get_char_metrics(FontId::MainRegular, char_code);
                let (w, h, d) = match m {
                    Some(m) => (m.width, m.height, m.depth),
                    None => missing_glyph_metrics_fallback(ch, options),
                };
                children.push(LayoutBox {
                    width: w,
                    height: h,
                    depth: d,
                    content: BoxContent::Glyph {
                        font_id: FontId::MainRegular,
                        char_code,
                    },
                    color: options.color,
                });
            }
            ParseNode::SpacingNode { text, .. } => {
                children.push(layout_spacing_command(text, options));
            }
            _ => {
                children.push(layout_node(node, options));
            }
        }
    }
    make_hbox(children)
}

/// Layout \pmb — poor man's bold via CSS-style text shadow.
/// Renders the body twice: once normally, once offset by (0.02em, 0.01em).
fn layout_pmb(body: &[ParseNode], options: &LayoutOptions) -> LayoutBox {
    let base = layout_expression(body, options, true);
    let w = base.width;
    let h = base.height;
    let d = base.depth;

    // Shadow copy shifted right 0.02em, down 0.01em — same content, same color
    let shadow = layout_expression(body, options, true);
    let shadow_shift_x = 0.02_f64;
    let _shadow_shift_y = 0.01_f64;

    // Combine: place shadow first (behind), then base on top
    // Shadow is placed at an HBox offset — we use a VBox/kern trick:
    // Instead, represent as HBox where shadow overlaps base via negative kern
    let kern_back = LayoutBox::new_kern(-w);
    let kern_x = LayoutBox::new_kern(shadow_shift_x);

    // We create: [shadow | kern(-w) | base] in an HBox
    // But shadow needs to be shifted down by shadow_shift_y.
    // Use a raised box trick: wrap shadow in a VBox with a small kern.
    // Simplest approximation: just render body once (the shadow is < 1px at normal size)
    // but with a tiny kern to hint at bold width.
    // Better: use a simple 2-layer HBox with overlap.
    let children = vec![
        kern_x,
        shadow,
        kern_back,
        base,
    ];
    // Width should be original base width, not doubled
    let hbox = make_hbox(children);
    // Return a box with original dimensions (shadow overflow is clipped)
    LayoutBox {
        width: w,
        height: h,
        depth: d,
        content: hbox.content,
        color: options.color,
    }
}

/// Layout \fbox, \colorbox, \fcolorbox — framed/colored box.
/// Also handles \phase, \cancel, \sout, \bcancel, \xcancel.
fn layout_enclose(
    label: &str,
    background_color: Option<&str>,
    border_color: Option<&str>,
    body: &ParseNode,
    options: &LayoutOptions,
) -> LayoutBox {
    use crate::layout_box::BoxContent;
    use ratex_types::color::Color;

    // \phase: angle mark (diagonal line) below the body with underline
    if label == "\\phase" {
        return layout_phase(body, options);
    }

    // \angl: actuarial angle — arc/roof above the body (KaTeX actuarialangle-style)
    if label == "\\angl" {
        return layout_angl(body, options);
    }

    // \cancel, \bcancel, \xcancel, \sout: strike-through overlays
    if matches!(label, "\\cancel" | "\\bcancel" | "\\xcancel" | "\\sout") {
        return layout_cancel(label, body, options);
    }

    // KaTeX defaults: fboxpad = 3pt, fboxrule = 0.4pt
    let metrics = options.metrics();
    let padding = 3.0 / metrics.pt_per_em;
    let border_thickness = 0.4 / metrics.pt_per_em;

    let has_border = matches!(label, "\\fbox" | "\\fcolorbox");

    let bg = background_color.and_then(|c| Color::from_name(c).or_else(|| Color::from_hex(c)));
    let border = border_color
        .and_then(|c| Color::from_name(c).or_else(|| Color::from_hex(c)))
        .unwrap_or(Color::BLACK);

    let inner = layout_node(body, options);
    let outer_pad = padding + if has_border { border_thickness } else { 0.0 };

    let width = inner.width + 2.0 * outer_pad;
    let height = inner.height + outer_pad;
    let depth = inner.depth + outer_pad;

    LayoutBox {
        width,
        height,
        depth,
        content: BoxContent::Framed {
            body: Box::new(inner),
            padding,
            border_thickness,
            has_border,
            bg_color: bg,
            border_color: border,
        },
        color: options.color,
    }
}

/// Layout \raisebox{dy}{body} — shift content vertically.
fn layout_raisebox(shift: f64, body: &ParseNode, options: &LayoutOptions) -> LayoutBox {
    use crate::layout_box::BoxContent;
    let inner = layout_node(body, options);
    // Positive shift moves content up → height increases, depth decreases
    let height = inner.height + shift;
    let depth = (inner.depth - shift).max(0.0);
    let width = inner.width;
    LayoutBox {
        width,
        height,
        depth,
        content: BoxContent::RaiseBox {
            body: Box::new(inner),
            shift,
        },
        color: options.color,
    }
}

/// Returns true if the parse node is a single character box (atom / mathord / textord),
/// mirroring KaTeX's `isCharacterBox` + `getBaseElem` logic.
fn is_single_char_body(node: &ParseNode) -> bool {
    use ratex_parser::parse_node::ParseNode as PN;
    match node {
        // Unwrap single-element ord-groups and styling nodes.
        PN::OrdGroup { body, .. } if body.len() == 1 => is_single_char_body(&body[0]),
        PN::Styling { body, .. } if body.len() == 1 => is_single_char_body(&body[0]),
        // Bare character nodes.
        PN::Atom { .. } | PN::MathOrd { .. } | PN::TextOrd { .. } => true,
        _ => false,
    }
}

/// Layout \cancel, \bcancel, \xcancel, \sout — body with strike-through line(s) overlay.
///
/// Matches KaTeX `enclose.ts` + `stretchy.ts` geometry:
///   • single char  → v_pad = 0.2em, h_pad = 0   (line corner-to-corner of w × (h+d+0.4) box)
///   • multi char   → v_pad = 0,     h_pad = 0.2em (cancel-pad: line extends 0.2em each side)
fn layout_cancel(
    label: &str,
    body: &ParseNode,
    options: &LayoutOptions,
) -> LayoutBox {
    use crate::layout_box::BoxContent;
    let inner = layout_node(body, options);
    let w = inner.width.max(0.01);
    let h = inner.height;
    let d = inner.depth;

    // KaTeX padding: single character gets vertical extension, multi-char gets horizontal.
    let single = is_single_char_body(body);
    let v_pad = if single { 0.2 } else { 0.0 };
    let h_pad = if single { 0.0 } else { 0.2 };

    // Path coordinates: y=0 at baseline, y<0 above (height), y>0 below (depth).
    // \cancel  = "/" diagonal: bottom-left → top-right
    // \bcancel = "\" diagonal: top-left → bottom-right
    let commands: Vec<PathCommand> = match label {
        "\\cancel" => vec![
            PathCommand::MoveTo { x: -h_pad,     y: d + v_pad  },  // bottom-left
            PathCommand::LineTo { x: w + h_pad,  y: -h - v_pad },  // top-right
        ],
        "\\bcancel" => vec![
            PathCommand::MoveTo { x: -h_pad,     y: -h - v_pad },  // top-left
            PathCommand::LineTo { x: w + h_pad,  y: d + v_pad  },  // bottom-right
        ],
        "\\xcancel" => vec![
            PathCommand::MoveTo { x: -h_pad,     y: d + v_pad  },
            PathCommand::LineTo { x: w + h_pad,  y: -h - v_pad },
            PathCommand::MoveTo { x: -h_pad,     y: -h - v_pad },
            PathCommand::LineTo { x: w + h_pad,  y: d + v_pad  },
        ],
        "\\sout" => {
            // Horizontal line at –0.5× x-height, extended to content edges.
            let mid_y = -0.5 * options.metrics().x_height;
            vec![
                PathCommand::MoveTo { x: 0.0, y: mid_y },
                PathCommand::LineTo { x: w,   y: mid_y },
            ]
        }
        _ => vec![],
    };

    let line_w = w + 2.0 * h_pad;
    let line_h = h + v_pad;
    let line_d = d + v_pad;
    let line_box = LayoutBox {
        width: line_w,
        height: line_h,
        depth: line_d,
        content: BoxContent::SvgPath { commands, fill: false },
        color: options.color,
    };

    // For multi-char the body is inset by h_pad from the line-box's left edge.
    let body_kern = -(line_w - h_pad);
    let body_shifted = make_hbox(vec![LayoutBox::new_kern(body_kern), inner]);
    LayoutBox {
        width: w,
        height: h,
        depth: d,
        content: BoxContent::HBox(vec![line_box, body_shifted]),
        color: options.color,
    }
}

/// Layout \phase{body} — angle notation: body with a diagonal angle mark + underline.
/// Matches KaTeX `enclose.ts` + `phasePath(y)` (steinmetz): dynamic viewBox height, `x = y/2` at the peak.
fn layout_phase(body: &ParseNode, options: &LayoutOptions) -> LayoutBox {
    use crate::layout_box::BoxContent;
    let metrics = options.metrics();
    let inner = layout_node(body, options);
    // KaTeX: lineWeight = 0.6pt, clearance = 0.35ex; angleHeight = inner.h + inner.d + both
    let line_weight = 0.6_f64 / metrics.pt_per_em;
    let clearance = 0.35_f64 * metrics.x_height;
    let angle_height = inner.height + inner.depth + line_weight + clearance;
    let left_pad = angle_height / 2.0 + line_weight;
    let width = inner.width + left_pad;

    // KaTeX: viewBoxHeight = floor(1000 * angleHeight * scale); base sizing uses scale → 1 here.
    let y_svg = (1000.0 * angle_height).floor().max(80.0);

    // Vertical: viewBox height y_svg → angle_height em (baseline mapping below).
    let sy = angle_height / y_svg;
    // Horizontal: KaTeX SVG uses preserveAspectRatio xMinYMin slice — scale follows viewBox height,
    // so x grows ~sy per SVG unit (not width/400000). That keeps the left angle visible; clip to `width`.
    let sx = sy;
    let right_x = (400_000.0_f64 * sx).min(width);

    // Baseline: peak at svg y=0 → -inner.height; bottom at y=y_svg → inner.depth + line_weight + clearance
    let bottom_y = inner.depth + line_weight + clearance;
    let vy = |y_sv: f64| -> f64 { bottom_y - (y_svg - y_sv) * sy };

    // phasePath(y): M400000 y H0 L y/2 0 l65 45 L145 y-80 H400000z
    let x_peak = y_svg / 2.0;
    let commands = vec![
        PathCommand::MoveTo { x: right_x, y: vy(y_svg) },
        PathCommand::LineTo { x: 0.0, y: vy(y_svg) },
        PathCommand::LineTo { x: x_peak * sx, y: vy(0.0) },
        PathCommand::LineTo { x: (x_peak + 65.0) * sx, y: vy(45.0) },
        PathCommand::LineTo {
            x: 145.0 * sx,
            y: vy(y_svg - 80.0),
        },
        PathCommand::LineTo {
            x: right_x,
            y: vy(y_svg - 80.0),
        },
        PathCommand::Close,
    ];

    let body_shifted = make_hbox(vec![
        LayoutBox::new_kern(left_pad),
        inner.clone(),
    ]);

    let path_height = inner.height;
    let path_depth = bottom_y;

    LayoutBox {
        width,
        height: path_height,
        depth: path_depth,
        content: BoxContent::HBox(vec![
            LayoutBox {
                width,
                height: path_height,
                depth: path_depth,
                content: BoxContent::SvgPath { commands, fill: true },
                color: options.color,
            },
            LayoutBox::new_kern(-width),
            body_shifted,
        ]),
        color: options.color,
    }
}

/// Layout \angl{body} — actuarial angle: horizontal roof line above body + vertical bar on the right (KaTeX/fixture style).
/// Path and body share the same baseline; vertical bar runs from roof down through baseline to bottom of body.
fn layout_angl(body: &ParseNode, options: &LayoutOptions) -> LayoutBox {
    use crate::layout_box::BoxContent;
    let inner = layout_node(body, options);
    let w = inner.width.max(0.3);
    // Roof line a bit higher: body_height + clearance
    let clearance = 0.1_f64;
    let arc_h = inner.height + clearance;

    // Path: horizontal roof (0,-arc_h) to (w,-arc_h), then vertical (w,-arc_h) down to (w, depth) so bar extends below baseline
    let path_commands = vec![
        PathCommand::MoveTo { x: 0.0, y: -arc_h },
        PathCommand::LineTo { x: w, y: -arc_h },
        PathCommand::LineTo { x: w, y: inner.depth + 0.3_f64},
    ];

    let height = arc_h;
    LayoutBox {
        width: w,
        height,
        depth: inner.depth,
        content: BoxContent::Angl {
            path_commands,
            body: Box::new(inner),
        },
        color: options.color,
    }
}

fn layout_font(font: &str, body: &ParseNode, options: &LayoutOptions) -> LayoutBox {
    let font_id = match font {
        "mathrm" | "\\mathrm" | "textrm" | "\\textrm" | "rm" | "\\rm" => Some(FontId::MainRegular),
        "mathbf" | "\\mathbf" | "textbf" | "\\textbf" | "bf" | "\\bf" => Some(FontId::MainBold),
        "mathit" | "\\mathit" | "textit" | "\\textit" => Some(FontId::MainItalic),
        "mathsf" | "\\mathsf" | "textsf" | "\\textsf" => Some(FontId::SansSerifRegular),
        "mathtt" | "\\mathtt" | "texttt" | "\\texttt" => Some(FontId::TypewriterRegular),
        "mathcal" | "\\mathcal" | "cal" | "\\cal" => Some(FontId::CaligraphicRegular),
        "mathfrak" | "\\mathfrak" | "frak" | "\\frak" => Some(FontId::FrakturRegular),
        "mathscr" | "\\mathscr" => Some(FontId::ScriptRegular),
        "mathbb" | "\\mathbb" => Some(FontId::AmsRegular),
        "boldsymbol" | "\\boldsymbol" | "bm" | "\\bm" => Some(FontId::MathBoldItalic),
        _ => None,
    };

    if let Some(fid) = font_id {
        layout_with_font(body, fid, options)
    } else {
        layout_node(body, options)
    }
}

fn layout_with_font(node: &ParseNode, font_id: FontId, options: &LayoutOptions) -> LayoutBox {
    match node {
        ParseNode::OrdGroup { body, .. } => {
            let children: Vec<LayoutBox> = body.iter().map(|n| layout_with_font(n, font_id, options)).collect();
            make_hbox(children)
        }
        ParseNode::SupSub {
            base, sup, sub, ..
        } => {
            if let Some(base_node) = base.as_deref() {
                if should_use_op_limits(base_node, options) {
                    return layout_op_with_limits(base_node, sup.as_deref(), sub.as_deref(), options);
                }
            }
            layout_supsub(base.as_deref(), sup.as_deref(), sub.as_deref(), options, Some(font_id))
        }
        ParseNode::MathOrd { text, .. }
        | ParseNode::TextOrd { text, .. }
        | ParseNode::Atom { text, .. } => {
            let ch = resolve_symbol_char(text, Mode::Math);
            let char_code = ch as u32;
            if let Some(m) = get_char_metrics(font_id, char_code) {
                LayoutBox {
                    width: m.width,
                    height: m.height,
                    depth: m.depth,
                    content: BoxContent::Glyph { font_id, char_code },
                    color: options.color,
                }
            } else {
                // Glyph not in requested font — fall back to default math rendering
                layout_node(node, options)
            }
        }
        _ => layout_node(node, options),
    }
}

// ============================================================================
// Overline / Underline
// ============================================================================

fn layout_overline(body: &ParseNode, options: &LayoutOptions) -> LayoutBox {
    let cramped = options.with_style(options.style.cramped());
    let body_box = layout_node(body, &cramped);
    let metrics = options.metrics();
    let rule = metrics.default_rule_thickness;

    // Total height: body height + 2*rule clearance + rule thickness = body.height + 3*rule
    let height = body_box.height + 3.0 * rule;
    LayoutBox {
        width: body_box.width,
        height,
        depth: body_box.depth,
        content: BoxContent::Overline {
            body: Box::new(body_box),
            rule_thickness: rule,
        },
        color: options.color,
    }
}

fn layout_underline(body: &ParseNode, options: &LayoutOptions) -> LayoutBox {
    let body_box = layout_node(body, options);
    let metrics = options.metrics();
    let rule = metrics.default_rule_thickness;

    // Total depth: body depth + 2*rule clearance + rule thickness = body.depth + 3*rule
    let depth = body_box.depth + 3.0 * rule;
    LayoutBox {
        width: body_box.width,
        height: body_box.height,
        depth,
        content: BoxContent::Underline {
            body: Box::new(body_box),
            rule_thickness: rule,
        },
        color: options.color,
    }
}

/// `\href` / `\url`: link color on the glyphs and an underline in the same color (KaTeX-style).
fn layout_href(body: &[ParseNode], options: &LayoutOptions) -> LayoutBox {
    let link_color = Color::from_name("blue").unwrap_or_else(|| Color::rgb(0.0, 0.0, 1.0));
    let body_opts = options.with_color(link_color);
    let body_box = layout_expression(body, &body_opts, true);
    layout_underline_laid_out(body_box, options, link_color)
}

/// Same geometry as [`layout_underline`], but for an already computed inner box.
fn layout_underline_laid_out(body_box: LayoutBox, options: &LayoutOptions, color: Color) -> LayoutBox {
    let metrics = options.metrics();
    let rule = metrics.default_rule_thickness;
    let depth = body_box.depth + 3.0 * rule;
    LayoutBox {
        width: body_box.width,
        height: body_box.height,
        depth,
        content: BoxContent::Underline {
            body: Box::new(body_box),
            rule_thickness: rule,
        },
        color,
    }
}

// ============================================================================
// Spacing commands
// ============================================================================

fn layout_spacing_command(text: &str, options: &LayoutOptions) -> LayoutBox {
    let metrics = options.metrics();
    let mu = metrics.css_em_per_mu();

    let width = match text {
        "\\," | "\\thinspace" => 3.0 * mu,
        "\\:" | "\\medspace" => 4.0 * mu,
        "\\;" | "\\thickspace" => 5.0 * mu,
        "\\!" | "\\negthinspace" => -3.0 * mu,
        "\\negmedspace" => -4.0 * mu,
        "\\negthickspace" => -5.0 * mu,
        " " | "~" | "\\nobreakspace" | "\\ " | "\\space" => {
            // KaTeX renders these by placing the U+00A0 glyph (char 160) via mathsym.
            // Look up its width from MainRegular; fall back to 0.25em (the font-defined value).
            // Literal space in `\text{ … }` becomes SpacingNode with text " ".
            get_char_metrics(FontId::MainRegular, 160)
                .map(|m| m.width)
                .unwrap_or(0.25)
        }
        "\\quad" => metrics.quad,
        "\\qquad" => 2.0 * metrics.quad,
        "\\enspace" => metrics.quad / 2.0,
        _ => 0.0,
    };

    LayoutBox::new_kern(width)
}

// ============================================================================
// Measurement conversion
// ============================================================================

fn measurement_to_em(m: &ratex_parser::parse_node::Measurement, options: &LayoutOptions) -> f64 {
    let metrics = options.metrics();
    match m.unit.as_str() {
        "em" => m.number,
        "ex" => m.number * metrics.x_height,
        "mu" => m.number * metrics.css_em_per_mu(),
        "pt" => m.number / metrics.pt_per_em,
        "mm" => m.number * 7227.0 / 2540.0 / metrics.pt_per_em,
        "cm" => m.number * 7227.0 / 254.0 / metrics.pt_per_em,
        "in" => m.number * 72.27 / metrics.pt_per_em,
        "bp" => m.number * 803.0 / 800.0 / metrics.pt_per_em,
        "pc" => m.number * 12.0 / metrics.pt_per_em,
        "dd" => m.number * 1238.0 / 1157.0 / metrics.pt_per_em,
        "cc" => m.number * 14856.0 / 1157.0 / metrics.pt_per_em,
        "nd" => m.number * 685.0 / 642.0 / metrics.pt_per_em,
        "nc" => m.number * 1370.0 / 107.0 / metrics.pt_per_em,
        "sp" => m.number / 65536.0 / metrics.pt_per_em,
        _ => m.number,
    }
}

// ============================================================================
// Math class determination
// ============================================================================

/// Determine the math class of a ParseNode for spacing purposes.
fn node_math_class(node: &ParseNode) -> Option<MathClass> {
    match node {
        ParseNode::MathOrd { .. } | ParseNode::TextOrd { .. } => Some(MathClass::Ord),
        ParseNode::Atom { family, .. } => Some(family_to_math_class(*family)),
        ParseNode::OpToken { .. } | ParseNode::Op { .. } | ParseNode::OperatorName { .. } => Some(MathClass::Op),
        ParseNode::OrdGroup { .. } => Some(MathClass::Ord),
        ParseNode::GenFrac { .. } => Some(MathClass::Inner),
        ParseNode::Sqrt { .. } => Some(MathClass::Ord),
        ParseNode::SupSub { base, .. } => {
            base.as_ref().and_then(|b| node_math_class(b))
        }
        ParseNode::MClass { mclass, .. } => Some(mclass_str_to_math_class(mclass)),
        ParseNode::SpacingNode { .. } => None,
        ParseNode::Kern { .. } => None,
        ParseNode::HtmlMathMl { html, .. } => {
            // Derive math class from the first meaningful child in the HTML branch
            for child in html {
                if let Some(cls) = node_math_class(child) {
                    return Some(cls);
                }
            }
            None
        }
        ParseNode::Lap { .. } => None,
        ParseNode::LeftRight { .. } => Some(MathClass::Inner),
        ParseNode::AccentToken { .. } => Some(MathClass::Ord),
        // \xrightarrow etc. are mathrel in TeX/KaTeX; without this they collapse to Ord–Ord (no kern).
        ParseNode::XArrow { .. } => Some(MathClass::Rel),
        _ => Some(MathClass::Ord),
    }
}

fn mclass_str_to_math_class(mclass: &str) -> MathClass {
    match mclass {
        "mord" => MathClass::Ord,
        "mop" => MathClass::Op,
        "mbin" => MathClass::Bin,
        "mrel" => MathClass::Rel,
        "mopen" => MathClass::Open,
        "mclose" => MathClass::Close,
        "mpunct" => MathClass::Punct,
        "minner" => MathClass::Inner,
        _ => MathClass::Ord,
    }
}

/// Check if a ParseNode is a single character box (affects sup/sub positioning).
fn is_character_box(node: &ParseNode) -> bool {
    matches!(
        node,
        ParseNode::MathOrd { .. }
            | ParseNode::TextOrd { .. }
            | ParseNode::Atom { .. }
            | ParseNode::AccentToken { .. }
    )
}

fn family_to_math_class(family: AtomFamily) -> MathClass {
    match family {
        AtomFamily::Bin => MathClass::Bin,
        AtomFamily::Rel => MathClass::Rel,
        AtomFamily::Open => MathClass::Open,
        AtomFamily::Close => MathClass::Close,
        AtomFamily::Punct => MathClass::Punct,
        AtomFamily::Inner => MathClass::Inner,
    }
}

// ============================================================================
// Horizontal brace layout (\overbrace, \underbrace)
// ============================================================================

fn layout_horiz_brace(
    base: &ParseNode,
    is_over: bool,
    options: &LayoutOptions,
) -> LayoutBox {
    let body_box = layout_node(base, options);
    let w = body_box.width.max(0.5);

    let label = if is_over { "overbrace" } else { "underbrace" };
    // KaTeXSize4 brace glyphs are closed contours meant for fill (like stretchy arrows).
    // fill=false strokes outlines → hollow “wireframe” and exaggerated bar ends.
    let (raw_commands, brace_h, brace_fill) =
        match crate::katex_svg::katex_stretchy_path(label, w) {
            Some((c, h)) => (c, h, true),
            None => {
                let h = 0.35_f64;
                (horiz_brace_path(w, h, is_over), h, false)
            }
        };

    // Shift y-coordinates: centered commands → positioned for over/under
    // For overbrace: foot at y=0 (bottom), peak goes up → shift by -brace_h/2
    // For underbrace: foot at y=0 (top), peak goes down → shift by +brace_h/2
    let y_shift = if is_over { -brace_h / 2.0 } else { brace_h / 2.0 };
    let commands = shift_path_y(raw_commands, y_shift);

    let brace_box = LayoutBox {
        width: w,
        height: if is_over { brace_h } else { 0.0 },
        depth: if is_over { 0.0 } else { brace_h },
        content: BoxContent::SvgPath {
            commands,
            fill: brace_fill,
        },
        color: options.color,
    };

    let gap = 0.1;
    let (height, depth) = if is_over {
        (body_box.height + brace_h + gap, body_box.depth)
    } else {
        (body_box.height, body_box.depth + brace_h + gap)
    };

    let clearance = if is_over {
        height - brace_h
    } else {
        body_box.height + body_box.depth + gap
    };
    let total_w = body_box.width;

    LayoutBox {
        width: total_w,
        height,
        depth,
        content: BoxContent::Accent {
            base: Box::new(body_box),
            accent: Box::new(brace_box),
            clearance,
            skew: 0.0,
            is_below: !is_over,
        },
        color: options.color,
    }
}

// ============================================================================
// XArrow layout (\xrightarrow, \xleftarrow, etc.)
// ============================================================================

fn layout_xarrow(
    label: &str,
    body: &ParseNode,
    below: Option<&ParseNode>,
    options: &LayoutOptions,
) -> LayoutBox {
    let sup_style = options.style.superscript();
    let sub_style = options.style.subscript();
    let sup_ratio = sup_style.size_multiplier() / options.style.size_multiplier();
    let sub_ratio = sub_style.size_multiplier() / options.style.size_multiplier();

    let sup_opts = options.with_style(sup_style);
    let body_box = layout_node(body, &sup_opts);
    let body_w = body_box.width * sup_ratio;

    let below_box = below.map(|b| {
        let sub_opts = options.with_style(sub_style);
        layout_node(b, &sub_opts)
    });
    let below_w = below_box
        .as_ref()
        .map(|b| b.width * sub_ratio)
        .unwrap_or(0.0);

    // KaTeX `katexImagesData` minWidth on the stretchy SVG, plus `.x-arrow-pad { padding: 0 0.5em }`
    // on each label row (em = that row's font). In parent em: +0.5·sup_ratio + 0.5·sup_ratio, etc.
    let min_w = crate::katex_svg::katex_stretchy_min_width_em(label).unwrap_or(1.0);
    let upper_w = body_w + sup_ratio;
    let lower_w = if below_box.is_some() {
        below_w + sub_ratio
    } else {
        0.0
    };
    let arrow_w = upper_w.max(lower_w).max(min_w);
    let arrow_h = 0.3;

    let (commands, actual_arrow_h, fill_arrow) =
        match crate::katex_svg::katex_stretchy_path(label, arrow_w) {
            Some((c, h)) => (c, h, true),
            None => (
                stretchy_accent_path(label, arrow_w, arrow_h),
                arrow_h,
                label == "\\xtwoheadrightarrow" || label == "\\xtwoheadleftarrow",
            ),
        };
    let arrow_box = LayoutBox {
        width: arrow_w,
        height: actual_arrow_h / 2.0,
        depth: actual_arrow_h / 2.0,
        content: BoxContent::SvgPath {
            commands,
            fill: fill_arrow,
        },
        color: options.color,
    };

    // KaTeX positions xarrows centered on the math axis, with a 0.111em (2mu) gap
    // between the arrow and the text above/below (see amsmath.dtx reference).
    let metrics = options.metrics();
    let axis = metrics.axis_height;        // 0.25em
    let arrow_half = actual_arrow_h / 2.0;
    let gap = 0.111;                       // 2mu gap (KaTeX constant)

    // Center the arrow on the math axis by shifting it up.
    let base_shift = -axis;

    // sup_kern: gap between arrow top and text bottom.
    // In the OpLimits renderer:
    //   sup_y = y - (arrow_half - base_shift) - sup_kern - sup_box.depth * ratio
    //         = y - (arrow_half + axis) - sup_kern - sup_box.depth * ratio
    // KaTeX: text_baseline = -(axis + arrow_half + gap)
    //   (with extra -= depth when depth > 0.25, but that's rare for typical text)
    // Matching: sup_kern = gap
    let sup_kern = gap;
    let sub_kern = gap;

    let sup_h = body_box.height * sup_ratio;
    let sup_d = body_box.depth * sup_ratio;

    // Height: from baseline to top of upper text
    let height = axis + arrow_half + gap + sup_h + sup_d;
    // Depth: arrow bottom below baseline = arrow_half - axis
    let mut depth = (arrow_half - axis).max(0.0);

    if let Some(ref bel) = below_box {
        let sub_h = bel.height * sub_ratio;
        let sub_d = bel.depth * sub_ratio;
        // Lower text positioned symmetrically below the arrow
        depth = (arrow_half - axis) + gap + sub_h + sub_d;
    }

    LayoutBox {
        width: arrow_w,
        height,
        depth,
        content: BoxContent::OpLimits {
            base: Box::new(arrow_box),
            sup: Some(Box::new(body_box)),
            sub: below_box.map(Box::new),
            base_shift,
            sup_kern,
            sub_kern,
            slant: 0.0,
            sup_scale: sup_ratio,
            sub_scale: sub_ratio,
        },
        color: options.color,
    }
}

// ============================================================================
// \textcircled layout
// ============================================================================

fn layout_textcircled(body_box: LayoutBox, options: &LayoutOptions) -> LayoutBox {
    // Draw a circle around the content, similar to KaTeX's CSS-based approach
    let pad = 0.1_f64; // padding around the content
    let total_h = body_box.height + body_box.depth;
    let radius = (body_box.width.max(total_h) / 2.0 + pad).max(0.35);
    let diameter = radius * 2.0;

    // Build a circle path using cubic Bezier approximation
    let cx = radius;
    let cy = -(body_box.height - total_h / 2.0); // center at vertical center of content
    let k = 0.5523; // cubic Bezier approximation of circle: 4*(sqrt(2)-1)/3
    let r = radius;

    let circle_commands = vec![
        PathCommand::MoveTo { x: cx + r, y: cy },
        PathCommand::CubicTo {
            x1: cx + r, y1: cy - k * r,
            x2: cx + k * r, y2: cy - r,
            x: cx, y: cy - r,
        },
        PathCommand::CubicTo {
            x1: cx - k * r, y1: cy - r,
            x2: cx - r, y2: cy - k * r,
            x: cx - r, y: cy,
        },
        PathCommand::CubicTo {
            x1: cx - r, y1: cy + k * r,
            x2: cx - k * r, y2: cy + r,
            x: cx, y: cy + r,
        },
        PathCommand::CubicTo {
            x1: cx + k * r, y1: cy + r,
            x2: cx + r, y2: cy + k * r,
            x: cx + r, y: cy,
        },
        PathCommand::Close,
    ];

    let circle_box = LayoutBox {
        width: diameter,
        height: r - cy.min(0.0),
        depth: (r + cy).max(0.0),
        content: BoxContent::SvgPath { commands: circle_commands, fill: false },
        color: options.color,
    };

    // Center the content inside the circle
    let content_shift = (diameter - body_box.width) / 2.0;
    // Shift content to the right to center it
    let children = vec![
        circle_box,
        LayoutBox::new_kern(-(diameter) + content_shift),
        body_box.clone(),
    ];

    let height = r - cy.min(0.0);
    let depth = (r + cy).max(0.0);

    LayoutBox {
        width: diameter,
        height,
        depth,
        content: BoxContent::HBox(children),
        color: options.color,
    }
}

// ============================================================================
// Path generation helpers
// ============================================================================

// ============================================================================
// \imageof / \origof  (U+22B7 / U+22B6)
// ============================================================================

/// Synthesise \imageof (•—○) or \origof (○—•).
///
/// Neither glyph exists in any KaTeX font.  We build each symbol as an HBox
/// of three pieces:
///   disk  : filled circle SVG path
///   bar   : Rule (horizontal segment at circle-centre height)
///   ring  : stroked circle SVG path
///
/// The ordering is reversed for \origof.
///
/// Dimensions are calibrated against the KaTeX reference PNG (DPR=2, 20px font):
///   ink bbox ≈ 0.700w × 0.225h em, centre ≈ 0.263em above baseline.
///
/// Coordinate convention in path commands:
///   origin = baseline-left of the box, x right, y positive → below baseline.
fn layout_imageof_origof(imageof: bool, options: &LayoutOptions) -> LayoutBox {
    // Disk radius: filled circle ink height = 2·r = 0.225em  →  r = 0.1125em
    let r: f64 = 0.1125;
    // Circle centre above baseline (negative = above in path coords).
    // Calibrated to the math axis (≈0.25em) so both symbols sit at the same height
    // as the reference KaTeX rendering.
    let cy: f64 = -0.2625;
    // Cubic-Bezier circle approximation constant (4*(√2−1)/3)
    let k: f64 = 0.5523;
    // Each circle sub-box is 2r wide; the circle centre sits at x = r within it.
    let cx: f64 = r;

    // Box height/depth: symbol sits entirely above baseline.
    let h: f64 = r + cy.abs(); // 0.1125 + 0.2625 = 0.375
    let d: f64 = 0.0;

    // The renderer strokes rings with width = 1.5 × DPR pixels.
    // At the golden-test resolution (font=40px, DPR=1) that is 1.5 px = 0.0375em.
    // To keep the ring's outer ink edge coincident with the disk's outer edge,
    // draw the ring path at r_ring = r − stroke_half so the outer ink = r − stroke_half + stroke_half = r.
    let stroke_half: f64 = 0.01875; // 0.75px / 40px·em⁻¹
    let r_ring: f64 = r - stroke_half; // 0.09375em

    // Closed circle path (counter-clockwise) centred at (ox, cy) with radius rad.
    let circle_commands = |ox: f64, rad: f64| -> Vec<PathCommand> {
        vec![
            PathCommand::MoveTo { x: ox + rad, y: cy },
            PathCommand::CubicTo {
                x1: ox + rad,     y1: cy - k * rad,
                x2: ox + k * rad, y2: cy - rad,
                x:  ox,           y:  cy - rad,
            },
            PathCommand::CubicTo {
                x1: ox - k * rad, y1: cy - rad,
                x2: ox - rad,     y2: cy - k * rad,
                x:  ox - rad,     y:  cy,
            },
            PathCommand::CubicTo {
                x1: ox - rad,     y1: cy + k * rad,
                x2: ox - k * rad, y2: cy + rad,
                x:  ox,           y:  cy + rad,
            },
            PathCommand::CubicTo {
                x1: ox + k * rad, y1: cy + rad,
                x2: ox + rad,     y2: cy + k * rad,
                x:  ox + rad,     y:  cy,
            },
            PathCommand::Close,
        ]
    };

    let disk = LayoutBox {
        width: 2.0 * r,
        height: h,
        depth: d,
        content: BoxContent::SvgPath { commands: circle_commands(cx, r), fill: true },
        color: options.color,
    };

    let ring = LayoutBox {
        width: 2.0 * r,
        height: h,
        depth: d,
        content: BoxContent::SvgPath { commands: circle_commands(cx, r_ring), fill: false },
        color: options.color,
    };

    // Connecting bar centred on the same axis as the circles.
    // Rule.raise = distance from baseline to the bottom edge of the rule.
    // bar centre at |cy| = 0.2625em  →  raise = 0.2625 − bar_th/2
    let bar_len: f64 = 0.25;
    let bar_th: f64 = 0.04;
    let bar_raise: f64 = cy.abs() - bar_th / 2.0; // 0.2625 − 0.02 = 0.2425

    let bar = LayoutBox::new_rule(bar_len, h, d, bar_th, bar_raise);

    let children = if imageof {
        vec![disk, bar, ring]
    } else {
        vec![ring, bar, disk]
    };

    // Total width = 2r (disk) + bar_len + 2r (ring) = 0.225 + 0.25 + 0.225 = 0.700em
    let total_width = 4.0 * r + bar_len;
    LayoutBox {
        width: total_width,
        height: h,
        depth: d,
        content: BoxContent::HBox(children),
        color: options.color,
    }
}

/// Build path commands for a horizontal ellipse (circle overlay for \oiint, \oiiint).
/// Box-local coords: origin at baseline-left, x right, y down (positive = below baseline).
/// Ellipse is centered in the box and spans most of the integral width.
fn ellipse_overlay_path(width: f64, height: f64, depth: f64) -> Vec<PathCommand> {
    let cx = width / 2.0;
    let cy = (depth - height) / 2.0; // vertical center
    let a = width * 0.402_f64; // horizontal semi-axis (0.36 * 1.2)
    let b = 0.3_f64;          // vertical semi-axis (0.1 * 2)
    let k = 0.62_f64;          // Bezier factor: larger = fuller ellipse (0.5523 ≈ exact circle)
    vec![
        PathCommand::MoveTo { x: cx + a, y: cy },
        PathCommand::CubicTo {
            x1: cx + a,
            y1: cy - k * b,
            x2: cx + k * a,
            y2: cy - b,
            x: cx,
            y: cy - b,
        },
        PathCommand::CubicTo {
            x1: cx - k * a,
            y1: cy - b,
            x2: cx - a,
            y2: cy - k * b,
            x: cx - a,
            y: cy,
        },
        PathCommand::CubicTo {
            x1: cx - a,
            y1: cy + k * b,
            x2: cx - k * a,
            y2: cy + b,
            x: cx,
            y: cy + b,
        },
        PathCommand::CubicTo {
            x1: cx + k * a,
            y1: cy + b,
            x2: cx + a,
            y2: cy + k * b,
            x: cx + a,
            y: cy,
        },
        PathCommand::Close,
    ]
}

fn shift_path_y(cmds: Vec<PathCommand>, dy: f64) -> Vec<PathCommand> {
    cmds.into_iter().map(|c| match c {
        PathCommand::MoveTo { x, y } => PathCommand::MoveTo { x, y: y + dy },
        PathCommand::LineTo { x, y } => PathCommand::LineTo { x, y: y + dy },
        PathCommand::CubicTo { x1, y1, x2, y2, x, y } => PathCommand::CubicTo {
            x1, y1: y1 + dy, x2, y2: y2 + dy, x, y: y + dy,
        },
        PathCommand::QuadTo { x1, y1, x, y } => PathCommand::QuadTo {
            x1, y1: y1 + dy, x, y: y + dy,
        },
        PathCommand::Close => PathCommand::Close,
    }).collect()
}

fn stretchy_accent_path(label: &str, width: f64, height: f64) -> Vec<PathCommand> {
    if let Some(commands) = crate::katex_svg::katex_stretchy_arrow_path(label, width, height) {
        return commands;
    }
    let ah = height * 0.35; // arrowhead size
    let mid_y = -height / 2.0;

    match label {
        "\\overleftarrow" | "\\underleftarrow" | "\\xleftarrow" | "\\xLeftarrow" => {
            vec![
                PathCommand::MoveTo { x: ah, y: mid_y - ah },
                PathCommand::LineTo { x: 0.0, y: mid_y },
                PathCommand::LineTo { x: ah, y: mid_y + ah },
                PathCommand::MoveTo { x: 0.0, y: mid_y },
                PathCommand::LineTo { x: width, y: mid_y },
            ]
        }
        "\\overleftrightarrow" | "\\underleftrightarrow"
        | "\\xleftrightarrow" | "\\xLeftrightarrow" => {
            vec![
                PathCommand::MoveTo { x: ah, y: mid_y - ah },
                PathCommand::LineTo { x: 0.0, y: mid_y },
                PathCommand::LineTo { x: ah, y: mid_y + ah },
                PathCommand::MoveTo { x: 0.0, y: mid_y },
                PathCommand::LineTo { x: width, y: mid_y },
                PathCommand::MoveTo { x: width - ah, y: mid_y - ah },
                PathCommand::LineTo { x: width, y: mid_y },
                PathCommand::LineTo { x: width - ah, y: mid_y + ah },
            ]
        }
        "\\xlongequal" => {
            let gap = 0.04;
            vec![
                PathCommand::MoveTo { x: 0.0, y: mid_y - gap },
                PathCommand::LineTo { x: width, y: mid_y - gap },
                PathCommand::MoveTo { x: 0.0, y: mid_y + gap },
                PathCommand::LineTo { x: width, y: mid_y + gap },
            ]
        }
        "\\xhookleftarrow" => {
            vec![
                PathCommand::MoveTo { x: ah, y: mid_y - ah },
                PathCommand::LineTo { x: 0.0, y: mid_y },
                PathCommand::LineTo { x: ah, y: mid_y + ah },
                PathCommand::MoveTo { x: 0.0, y: mid_y },
                PathCommand::LineTo { x: width, y: mid_y },
                PathCommand::QuadTo { x1: width + ah, y1: mid_y, x: width + ah, y: mid_y + ah },
            ]
        }
        "\\xhookrightarrow" => {
            vec![
                PathCommand::MoveTo { x: 0.0 - ah, y: mid_y - ah },
                PathCommand::QuadTo { x1: 0.0 - ah, y1: mid_y, x: 0.0, y: mid_y },
                PathCommand::LineTo { x: width, y: mid_y },
                PathCommand::MoveTo { x: width - ah, y: mid_y - ah },
                PathCommand::LineTo { x: width, y: mid_y },
                PathCommand::LineTo { x: width - ah, y: mid_y + ah },
            ]
        }
        "\\xrightharpoonup" | "\\xleftharpoonup" => {
            let right = label.contains("right");
            if right {
                vec![
                    PathCommand::MoveTo { x: 0.0, y: mid_y },
                    PathCommand::LineTo { x: width, y: mid_y },
                    PathCommand::MoveTo { x: width - ah, y: mid_y - ah },
                    PathCommand::LineTo { x: width, y: mid_y },
                ]
            } else {
                vec![
                    PathCommand::MoveTo { x: ah, y: mid_y - ah },
                    PathCommand::LineTo { x: 0.0, y: mid_y },
                    PathCommand::LineTo { x: width, y: mid_y },
                ]
            }
        }
        "\\xrightharpoondown" | "\\xleftharpoondown" => {
            let right = label.contains("right");
            if right {
                vec![
                    PathCommand::MoveTo { x: 0.0, y: mid_y },
                    PathCommand::LineTo { x: width, y: mid_y },
                    PathCommand::MoveTo { x: width - ah, y: mid_y + ah },
                    PathCommand::LineTo { x: width, y: mid_y },
                ]
            } else {
                vec![
                    PathCommand::MoveTo { x: ah, y: mid_y + ah },
                    PathCommand::LineTo { x: 0.0, y: mid_y },
                    PathCommand::LineTo { x: width, y: mid_y },
                ]
            }
        }
        "\\xrightleftharpoons" | "\\xleftrightharpoons" => {
            let gap = 0.06;
            vec![
                PathCommand::MoveTo { x: 0.0, y: mid_y - gap },
                PathCommand::LineTo { x: width, y: mid_y - gap },
                PathCommand::MoveTo { x: width - ah, y: mid_y - gap - ah },
                PathCommand::LineTo { x: width, y: mid_y - gap },
                PathCommand::MoveTo { x: width, y: mid_y + gap },
                PathCommand::LineTo { x: 0.0, y: mid_y + gap },
                PathCommand::MoveTo { x: ah, y: mid_y + gap + ah },
                PathCommand::LineTo { x: 0.0, y: mid_y + gap },
            ]
        }
        "\\xtofrom" | "\\xrightleftarrows" => {
            let gap = 0.06;
            vec![
                PathCommand::MoveTo { x: 0.0, y: mid_y - gap },
                PathCommand::LineTo { x: width, y: mid_y - gap },
                PathCommand::MoveTo { x: width - ah, y: mid_y - gap - ah },
                PathCommand::LineTo { x: width, y: mid_y - gap },
                PathCommand::LineTo { x: width - ah, y: mid_y - gap + ah },
                PathCommand::MoveTo { x: width, y: mid_y + gap },
                PathCommand::LineTo { x: 0.0, y: mid_y + gap },
                PathCommand::MoveTo { x: ah, y: mid_y + gap - ah },
                PathCommand::LineTo { x: 0.0, y: mid_y + gap },
                PathCommand::LineTo { x: ah, y: mid_y + gap + ah },
            ]
        }
        "\\overlinesegment" | "\\underlinesegment" => {
            vec![
                PathCommand::MoveTo { x: 0.0, y: mid_y },
                PathCommand::LineTo { x: width, y: mid_y },
            ]
        }
        _ => {
            vec![
                PathCommand::MoveTo { x: 0.0, y: mid_y },
                PathCommand::LineTo { x: width, y: mid_y },
                PathCommand::MoveTo { x: width - ah, y: mid_y - ah },
                PathCommand::LineTo { x: width, y: mid_y },
                PathCommand::LineTo { x: width - ah, y: mid_y + ah },
            ]
        }
    }
}

fn horiz_brace_path(width: f64, height: f64, is_over: bool) -> Vec<PathCommand> {
    let mid = width / 2.0;
    let q = height * 0.6;
    if is_over {
        vec![
            PathCommand::MoveTo { x: 0.0, y: 0.0 },
            PathCommand::QuadTo { x1: 0.0, y1: -q, x: mid * 0.4, y: -q },
            PathCommand::LineTo { x: mid - 0.05, y: -q },
            PathCommand::LineTo { x: mid, y: -height },
            PathCommand::LineTo { x: mid + 0.05, y: -q },
            PathCommand::LineTo { x: width - mid * 0.4, y: -q },
            PathCommand::QuadTo { x1: width, y1: -q, x: width, y: 0.0 },
        ]
    } else {
        vec![
            PathCommand::MoveTo { x: 0.0, y: 0.0 },
            PathCommand::QuadTo { x1: 0.0, y1: q, x: mid * 0.4, y: q },
            PathCommand::LineTo { x: mid - 0.05, y: q },
            PathCommand::LineTo { x: mid, y: height },
            PathCommand::LineTo { x: mid + 0.05, y: q },
            PathCommand::LineTo { x: width - mid * 0.4, y: q },
            PathCommand::QuadTo { x1: width, y1: q, x: width, y: 0.0 },
        ]
    }
}
