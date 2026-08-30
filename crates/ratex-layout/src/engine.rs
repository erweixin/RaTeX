use std::collections::HashMap;

use ratex_font::{get_char_metrics, get_global_metrics, FontId};
use ratex_parser::parse_node::{
    ArrayTag, AtomFamily, Mode, ParseNode, ProofBranch, ProofLineStyle, StyleStr,
};
use ratex_types::color::Color;
use ratex_types::math_style::MathStyle;
use ratex_types::path_command::PathCommand;

use crate::hbox::make_hbox;
use crate::layout_box::{
    BoxContent, LayoutBox, PlacedBox, ProofRule, RunGlyph, VBoxChild, VBoxChildKind,
};
use crate::layout_options::LayoutOptions;

use crate::katex_svg::parse_svg_path_data;
use crate::spacing::{atom_spacing, mu_to_em, MathClass};
use crate::stacked_delim::make_stacked_delim_if_needed;

/// TeX `\nulldelimiterspace` = 1.2pt = 0.12em (at 10pt design size).
/// KaTeX wraps every `\frac` / `\atop` in mopen+mclose nulldelimiter spans of this width.
const NULL_DELIMITER_SPACE: f64 = 0.12;

fn style_str_to_math_style(style: &StyleStr) -> MathStyle {
    match style {
        StyleStr::Display => MathStyle::Display,
        StyleStr::Text => MathStyle::Text,
        StyleStr::Script => MathStyle::Script,
        StyleStr::Scriptscript => MathStyle::ScriptScript,
    }
}

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
            None | Some(MathClass::Bin)
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

/// KaTeX HTML: `\middle` delimiters are built with class `delimsizing`, which
/// `getTypeOfDomTree` does not map to a math atom type, so **no** implicit
/// table glue is inserted next to them (buildHTML.js). RaTeX must match that or
/// `\frac` (Inner) gains spurious 3mu on each side of every `\middle\vert`.
fn node_is_middle_fence(node: &ParseNode) -> bool {
    matches!(node, ParseNode::Middle { .. })
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

    let raw_classes: Vec<Option<MathClass>> = nodes.iter().map(node_math_class).collect();
    let eff_classes = apply_bin_cancellation(&raw_classes);

    let mut children = Vec::new();
    let mut prev_class: Option<MathClass> = None;
    // Index of the last node that contributed `prev_class` (for `\middle` glue suppression).
    let mut prev_class_node_idx: Option<usize> = None;

    for (i, node) in nodes.iter().enumerate() {
        let lbox = layout_node(node, options);
        let cur_class = eff_classes.get(i).copied().flatten();

        if is_real_group {
            if let (Some(prev), Some(cur)) = (prev_class, cur_class) {
                let prev_middle =
                    prev_class_node_idx.is_some_and(|j| node_is_middle_fence(&nodes[j]));
                let cur_middle = node_is_middle_fence(node);
                let mu = if prev_middle || cur_middle {
                    0.0
                } else {
                    atom_spacing(prev, cur, options.style.is_tight())
                };
                let mu = if let Some(cap) = options.align_relation_spacing {
                    if prev == MathClass::Rel || cur == MathClass::Rel {
                        mu.min(cap)
                    } else {
                        mu
                    }
                } else {
                    mu
                };
                if mu > 0.0 {
                    let em = mu_to_em(mu, options.metrics().quad);
                    children.push(LayoutBox::new_kern(em));
                }
            }
        }

        if cur_class.is_some() {
            prev_class = cur_class;
            prev_class_node_idx = Some(i);
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
            vchildren.push(VBoxChild {
                kind: VBoxChildKind::Kern(gap),
                shift: 0.0,
            });
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

        ParseNode::SupSub { base, sup, sub, .. } => {
            if let Some(base_node) = base.as_deref() {
                if should_use_op_limits(base_node, options) {
                    return layout_op_with_limits(
                        base_node,
                        sup.as_deref(),
                        sub.as_deref(),
                        options,
                    );
                }
            }
            layout_supsub(
                base.as_deref(),
                sup.as_deref(),
                sub.as_deref(),
                options,
                None,
            )
        }

        ParseNode::GenFrac {
            numer,
            denom,
            has_bar_line,
            bar_size,
            left_delim,
            right_delim,
            continued,
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
            let frac = layout_fraction(numer, denom, bar_thickness, *continued, options);

            let has_left = left_delim
                .as_ref()
                .is_some_and(|d| !d.is_empty() && d != ".");
            let has_right = right_delim
                .as_ref()
                .is_some_and(|d| !d.is_empty() && d != ".");

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
                let right_nds = if *continued {
                    0.0
                } else {
                    NULL_DELIMITER_SPACE
                };
                make_hbox(vec![
                    LayoutBox::new_kern(NULL_DELIMITER_SPACE),
                    frac,
                    LayoutBox::new_kern(right_nds),
                ])
            }
        }

        ParseNode::Sqrt { body, index, .. } => layout_radical(body, index.as_deref(), options),

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
            let new_style = style_str_to_math_style(style);
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
            label,
            base,
            is_stretchy,
            is_shifty,
            ..
        } => {
            // Some text accents (e.g. \c cedilla) place the mark below
            let is_below = matches!(label.as_str(), "\\c");
            layout_accent(
                label,
                base,
                is_stretchy.unwrap_or(false),
                is_shifty.unwrap_or(false),
                is_below,
                options,
            )
        }

        ParseNode::AccentUnder {
            label,
            base,
            is_stretchy,
            ..
        } => layout_accent(
            label,
            base,
            is_stretchy.unwrap_or(false),
            false,
            true,
            options,
        ),

        ParseNode::LeftRight {
            body, left, right, ..
        } => layout_left_right(body, left, right, options),

        ParseNode::DelimSizing { size, delim, .. } => layout_delim_sizing(*size, delim, options),

        ParseNode::Array {
            body,
            cols,
            arraystretch,
            add_jot,
            row_gaps,
            hlines_before_row,
            col_separation_type,
            hskip_before_and_after,
            is_cd,
            tags,
            leqno,
            ..
        } => {
            if is_cd.unwrap_or(false) {
                layout_cd(body, options)
            } else {
                layout_array(
                    body,
                    cols.as_deref(),
                    *arraystretch,
                    add_jot.unwrap_or(false),
                    row_gaps,
                    hlines_before_row,
                    col_separation_type.as_deref(),
                    hskip_before_and_after.unwrap_or(false),
                    tags.as_deref(),
                    leqno.unwrap_or(false),
                    options,
                )
            }
        }

        ParseNode::CdArrow {
            direction,
            label_above,
            label_below,
            ..
        } => layout_cd_arrow(
            direction,
            label_above.as_deref(),
            label_below.as_deref(),
            0.0,
            0.0,
            true,
            options,
        ),

        ParseNode::ProofTree { tree, .. } => layout_proof_tree(tree, options),

        ParseNode::Sizing { size, body, .. } => layout_sizing(*size, body, options),

        ParseNode::Text {
            body, font, mode, ..
        } => match font.as_deref() {
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
                .with_color(options.color)
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

        ParseNode::Smash {
            body,
            smash_height,
            smash_depth,
            ..
        } => {
            let mut inner = layout_node(body, options);
            if *smash_height {
                inner.height = 0.0;
            }
            if *smash_depth {
                inner.depth = 0.0;
            }
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

        ParseNode::HtmlMathMl { html, .. } => layout_expression(html, options, true),

        ParseNode::Html {
            attributes, body, ..
        } => layout_html(attributes, body, options),

        ParseNode::MClass { body, .. } => layout_expression(body, options, true),

        ParseNode::MathChoice {
            display,
            text,
            script,
            scriptscript,
            ..
        } => {
            let branch = match options.style {
                MathStyle::Display | MathStyle::DisplayCramped => display,
                MathStyle::Text | MathStyle::TextCramped => text,
                MathStyle::Script | MathStyle::ScriptCramped => script,
                MathStyle::ScriptScript | MathStyle::ScriptScriptCramped => scriptscript,
            };
            layout_expression(branch, options, true)
        }

        ParseNode::Lap {
            alignment, body, ..
        } => {
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
            base,
            is_over,
            label,
            ..
        } => layout_horiz_brace(base, *is_over, label, options),

        ParseNode::XArrow {
            label, body, below, ..
        } => layout_xarrow(label, body, below.as_deref(), options),

        ParseNode::Pmb { body, .. } => layout_pmb(body, options),

        ParseNode::HBox { body, .. } => layout_text(body, options),

        ParseNode::Enclose {
            label,
            background_color,
            border_color,
            body,
            ..
        } => layout_enclose(
            label,
            background_color.as_deref(),
            border_color.as_deref(),
            body,
            options,
        ),

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

        ParseNode::Tag { tag, .. } => {
            let text_opts = options.with_style(options.style.text());
            layout_expression(tag, &text_opts, true)
        }

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
        // Emoji / pictographs in supplementary planes (e.g. U+1F60A 😊) and related blocks:
        // system fallback draws ~one em, same rationale as CJK above (issue #49).
        0x1F000..=0x1FAFF => 1.0,
        // Dingbats (many BMP emoji / ornaments lack bundled TeX metrics)
        0x2700..=0x27BF => 1.0,
        // Miscellaneous Symbols (★ ☆ ☎ ☑ ☒ etc.)
        0x2600..=0x26FF => 1.0,
        // Miscellaneous Symbols and Arrows (⭐ ⬛ ⬜ etc.)
        0x2B00..=0x2BFF => 1.0,
        _ => 0.5,
    }
}

fn missing_glyph_height_em(ch: char, m: &ratex_font::MathConstants) -> f64 {
    let ru = ch as u32;
    if (0x1F000..=0x1FAFF).contains(&ru) {
        // Supplementary-plane emoji: `missing_glyph_width_em` uses ~1em width for raster
        // parity (#49), but a 0.92·quad tall box inflates `\sqrt` `min_delim_height` past KaTeX’s
        // threshold so we pick Size1 surd (1em advance) instead of the small surd (0.833em),
        // visibly widening the gap before the radicand (golden 0955).
        (m.quad * 0.74).max(m.x_height)
    } else {
        (m.quad * 0.92).max(m.x_height)
    }
}

fn missing_glyph_metrics_fallback(ch: char, options: &LayoutOptions) -> (f64, f64, f64) {
    let m = get_global_metrics(options.style.size_index());
    let w = missing_glyph_width_em(ch);
    if w >= 0.99 {
        let h = missing_glyph_height_em(ch, m);
        (w, h, 0.0)
    } else {
        (w, m.x_height, 0.0)
    }
}

/// KaTeX `SymbolNode.toNode`: math symbols use `margin-right: italic` (advance = width + italic).
#[inline]
fn math_glyph_advance_em(m: &ratex_font::CharMetrics, mode: Mode) -> f64 {
    if mode == Mode::Math {
        m.width + m.italic
    } else {
        m.width
    }
}

fn layout_symbol(text: &str, mode: Mode, options: &LayoutOptions) -> LayoutBox {
    let ch = resolve_symbol_char(text, mode);

    if text == "\\shortmid" {
        return make_shortmid_box(options);
    }
    if text == "\\textunderscore" {
        return make_textunderscore_box(options);
    }
    if let Some(fitted) = layout_symbol_from_render_spec(text, mode, options, None) {
        return fitted;
    }

    // Synthetic symbols not present in any KaTeX font; built from SVG paths.
    match ch as u32 {
        0x2258 | 0xE258 => return make_corresponds_symbol(options),
        0x225E | 0xE25E => return make_measured_by_symbol(options),
        0x22B7 => return layout_imageof_origof(true, options), // \imageof  •—○
        0x22B6 => return layout_imageof_origof(false, options), // \origof   ○—•
        _ => {}
    }

    let char_code = ch as u32;

    if let Some((font_id, metric_cp)) =
        ratex_font::font_and_metric_for_mathematical_alphanumeric(char_code)
    {
        let m = get_char_metrics(font_id, metric_cp);
        let (width, height, depth) = match m {
            Some(m) => (math_glyph_advance_em(&m, mode), m.height, m.depth),
            None => missing_glyph_metrics_fallback(ch, options),
        };
        return LayoutBox {
            width,
            height,
            depth,
            content: BoxContent::Glyph { font_id, char_code },
            color: options.color,
        };
    }

    let mut font_id = select_font(text, ch, mode, options);
    let mut metrics = get_char_metrics(font_id, char_code);

    if metrics.is_none() && mode == Mode::Math && font_id != FontId::MathItalic {
        if let Some(m) = get_char_metrics(FontId::MathItalic, char_code) {
            font_id = FontId::MathItalic;
            metrics = Some(m);
        }
    }

    // KaTeX `Main-Regular` has no metrics/cmap for some codepoints (e.g. U+2211) that only exist
    // in `Size1`/`Size2`. `\@char` yields `textord`, so we still rasterize via the normal lookup
    // chain (unicode fallback when Main has no glyph). Using `missing_glyph_metrics_fallback`
    // (0.5em wide) then clips the real fallback outline in PNG/SVG — borrow Size-font TeX metrics
    // for the box only, without switching `font_id`.
    let (width, height, depth) = if let Some(m) = metrics {
        (math_glyph_advance_em(&m, mode), m.height, m.depth)
    } else if mode == Mode::Math {
        let size_font = if options.style.is_display() {
            FontId::Size2Regular
        } else {
            FontId::Size1Regular
        };
        match get_char_metrics(size_font, char_code)
            .or_else(|| get_char_metrics(FontId::Size1Regular, char_code))
        {
            Some(m) => (math_glyph_advance_em(&m, mode), m.height, m.depth),
            None => missing_glyph_metrics_fallback(ch, options),
        }
    } else {
        missing_glyph_metrics_fallback(ch, options)
    };

    // If the glyph has no KaTeX metrics and is a wide fallback character (CJK, emoji, etc.),
    // switch font_id to CjkRegular so renderers can load it from a system Unicode font.
    if metrics.is_none() && missing_glyph_width_em(ch) >= 0.99 {
        font_id = FontId::CjkRegular;
    }

    LayoutBox {
        width,
        height,
        depth,
        content: BoxContent::Glyph { font_id, char_code },
        color: options.color,
    }
}

/// Fit a bundled replacement glyph into another symbol's metric box.
///
/// This keeps the input symbol's TeX atom class independent from presentation:
/// U+25CB WHITE CIRCLE remains an ordinary atom, while its bundled U+25EF glyph
/// is uniformly scaled and centered in U+25A1 WHITE SQUARE's metric box.
fn layout_symbol_from_render_spec(
    text: &str,
    mode: Mode,
    options: &LayoutOptions,
    preferred_source_font: Option<FontId>,
) -> Option<LayoutBox> {
    let symbol_mode = match mode {
        Mode::Math => ratex_font::Mode::Math,
        Mode::Text => ratex_font::Mode::Text,
    };
    let spec = ratex_font::get_symbol_render_spec(text, symbol_mode)?;
    let symbol = ratex_font::get_symbol(text, symbol_mode)?;
    let source_codepoint = symbol.codepoint? as u32;
    let default_source_font = symbol_font_id(symbol.font);
    let target_font = symbol_font_id(spec.target_font);

    let (source_font, source) = preferred_source_font
        .and_then(|font_id| {
            get_char_metrics(font_id, source_codepoint).map(|metrics| (font_id, metrics))
        })
        .or_else(|| {
            get_char_metrics(default_source_font, source_codepoint)
                .map(|metrics| (default_source_font, metrics))
        })?;
    let target = get_char_metrics(target_font, spec.target_codepoint as u32)?;

    let source_width = math_glyph_advance_em(&source, mode);
    let source_total = source.height + source.depth;
    let target_width = math_glyph_advance_em(&target, mode);
    let target_total = target.height + target.depth;
    if source_width <= 0.0 || source_total <= 0.0 || target_width <= 0.0 || target_total <= 0.0 {
        return None;
    }

    // Preserve the replacement glyph's aspect ratio and contain it in the
    // target box. Any spare space is divided equally on the corresponding axis.
    let glyph_scale = (target_width / source_width).min(target_total / source_total);
    let scaled_width = source_width * glyph_scale;
    let side_padding = ((target_width - scaled_width) / 2.0).max(0.0);
    let source_center = (source.height - source.depth) * glyph_scale / 2.0;
    let target_center = (target.height - target.depth) / 2.0;
    let raise = target_center - source_center;

    let glyph = LayoutBox {
        width: source_width,
        height: source.height,
        depth: source.depth,
        content: BoxContent::Glyph {
            font_id: source_font,
            char_code: source_codepoint,
        },
        color: options.color,
    };
    let scaled = LayoutBox {
        width: scaled_width,
        height: source.height * glyph_scale,
        depth: source.depth * glyph_scale,
        content: BoxContent::Scaled {
            body: Box::new(glyph),
            child_scale: glyph_scale,
        },
        color: options.color,
    };
    let centered = LayoutBox {
        width: scaled_width,
        height: target.height,
        depth: target.depth,
        content: BoxContent::RaiseBox {
            body: Box::new(scaled),
            shift: raise,
        },
        color: options.color,
    };

    let mut result = make_hbox(vec![
        LayoutBox::new_kern(side_padding),
        centered,
        LayoutBox::new_kern(side_padding),
    ]);
    result.color = options.color;
    Some(result)
}

fn symbol_font_id(font: ratex_font::SymbolFont) -> FontId {
    match font {
        ratex_font::SymbolFont::Main => FontId::MainRegular,
        ratex_font::SymbolFont::Ams => FontId::AmsRegular,
    }
}

/// Resolve a symbol name to its actual character.
fn resolve_symbol_char(text: &str, mode: Mode) -> char {
    let font_mode = match mode {
        Mode::Math => ratex_font::Mode::Math,
        Mode::Text => ratex_font::Mode::Text,
    };

    if let Some(raw) = text.chars().next() {
        let ru = raw as u32;
        if (0x1D400..=0x1D7FF).contains(&ru) {
            return raw;
        }
    }

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
                || is_math_italic_greek(resolved_char)
            {
                FontId::MathItalic
            } else {
                FontId::MainRegular
            }
        }
        Mode::Text => FontId::MainRegular,
    }
}

/// Lowercase Greek letters and variant forms use Math-Italic in math mode.
/// Uppercase Greek (U+0391–U+03A9) stays upright in Main-Regular per TeX convention.
fn is_math_italic_greek(ch: char) -> bool {
    matches!(
        ch,
        '\u{03B1}'..='\u{03C9}' | '\u{03D1}' | '\u{03D5}' | '\u{03D6}' | '\u{03F1}' | '\u{03F5}'
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
    continued: bool,
    options: &LayoutOptions,
) -> LayoutBox {
    let numer_s = options.style.numerator();
    let denom_s = options.style.denominator();
    let numer_style = options.with_style(numer_s);
    let denom_style = options.with_style(denom_s);

    let mut numer_box = layout_node(numer, &numer_style);
    // KaTeX genfrac.js: `\cfrac` pads the numerator with a \strut (TeXbook p.353): 8.5pt × 3.5pt.
    if continued {
        let pt = options.metrics().pt_per_em;
        let h_min = 8.5 / pt;
        let d_min = 3.5 / pt;
        if numer_box.height < h_min {
            numer_box.height = h_min;
        }
        if numer_box.depth < d_min {
            numer_box.depth = d_min;
        }
    }
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

    let horiz_brace_over = matches!(base, Some(ParseNode::HorizBrace { is_over: true, .. }));
    let horiz_brace_under = matches!(base, Some(ParseNode::HorizBrace { is_over: false, .. }));
    let center_scripts = horiz_brace_over || horiz_brace_under;

    let base_box = base
        .map(|b| layout_child(b, options))
        .unwrap_or_else(LayoutBox::new_empty);

    let is_char_box = base.is_some_and(is_character_box);
    let metrics = options.metrics();
    // KaTeX `supsub.js`: each script span gets `marginRight: (0.5pt/ptPerEm)/sizeMultiplier`
    // (TeX `\scriptspace`). Without this, sub/sup boxes are too narrow vs KaTeX (e.g. `pmatrix`
    // column widths and inter-column alignment in golden tests).
    let script_space = 0.5 / metrics.pt_per_em / options.size_multiplier();

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

    let sup_height_scaled = sup_box
        .as_ref()
        .map(|b| b.height * sup_ratio)
        .unwrap_or(0.0);
    let sup_depth_scaled = sup_box.as_ref().map(|b| b.depth * sup_ratio).unwrap_or(0.0);
    let sub_height_scaled = sub_box
        .as_ref()
        .map(|b| b.height * sub_ratio)
        .unwrap_or(0.0);
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

    // KaTeX `horizBrace.js` htmlBuilder: the script is placed using a VList with a fixed 0.2em
    // kern between the brace result and the script, plus the script's own (scaled) dimensions.
    // This overrides the default TeX Rule 18 sub_shift / sup_shift with the exact KaTeX layout.
    if horiz_brace_over && sup_box.is_some() {
        sup_shift = base_box.height + 0.2 + sup_depth_scaled;
    }
    if horiz_brace_under && sub_box.is_some() {
        sub_shift = base_box.depth + 0.2 + sub_height_scaled;
    }

    // Superscript horizontal offset: `layout_symbol` already uses advance width + italic
    // (KaTeX `margin-right: italic`), so we must not add `glyph_italic` again here.
    let italic_correction = 0.0;

    // KaTeX `supsub.js`: only a direct `SymbolNode` base (plus the synthetic `\oiint` /
    // `\oiiint` operators) gets `margin-left: -base.italic`.  In particular, spans such as
    // `\text{\textit{CPI}}` and `{x}` must not inherit the italic correction of their last glyph.
    let sub_h_kern = if sub_box.is_some() && !center_scripts {
        let direct_italic = direct_glyph_italic(&base_box);
        let is_oiint = matches!(
            base,
            Some(ParseNode::Op {
                name: Some(name),
                ..
            }) if name == "\\oiint" || name == "\\oiiint"
        );
        let base_italic = direct_italic
            .or_else(|| is_oiint.then(|| first_glyph_italic(&base_box)).flatten())
            .unwrap_or(0.0);
        -base_italic
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
            total_width = total_width.max(sup_b.width * sup_ratio + script_space);
        } else {
            total_width = total_width
                .max(base_box.width + italic_correction + sup_b.width * sup_ratio + script_space);
        }
    }
    if let Some(ref sub_b) = sub_box {
        depth = depth.max(sub_shift + sub_depth_scaled);
        if center_scripts {
            total_width = total_width.max(sub_b.width * sub_ratio + script_space);
        } else {
            total_width = total_width
                .max(base_box.width + sub_h_kern + sub_b.width * sub_ratio + script_space);
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
            sub_h_kern,
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
        line_clearance = (line_clearance + delim_depth - body_box.height - body_box.depth) / 2.0;
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
        } => *limits && (options.style.is_display() || always_handle_sup_sub.unwrap_or(false)),
        ParseNode::OperatorName {
            always_handle_sup_sub,
            limits,
            ..
        } => *always_handle_sup_sub && (options.style.is_display() || *limits),
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
        let shift = (base_box.height - base_box.depth) / 2.0 - axis;
        if shift.abs() > 0.001 {
            base_box.height -= shift;
            base_box.depth += shift;
        }
    }

    // For user-defined \mathop{content} (e.g. \vcentcolon), center the content
    // on the math axis via a RaiseBox so the glyph physically moves up/down.
    // The HBox emit pass keeps all children at the same baseline, so adjusting
    // height/depth alone doesn't move the glyph.
    if !suppress_base_shift && !symbol && body.is_some() {
        let axis = options.metrics().axis_height;
        let delta = (base_box.height - base_box.depth) / 2.0 - axis;
        if delta.abs() > 0.001 {
            let w = base_box.width;
            // delta < 0 → center is below axis → raise (positive RaiseBox shift)
            let raise = -delta;
            base_box = LayoutBox {
                width: w,
                height: (base_box.height + raise).max(0.0),
                depth: (base_box.depth - raise).max(0.0),
                content: BoxContent::RaiseBox {
                    body: Box::new(base_box),
                    shift: raise,
                },
                color: options.color,
            };
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
        let large = options.style.is_display() && !NO_SUCCESSOR.contains(&name.unwrap_or(""));
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
        let base = LayoutBox {
            // `LayoutBox::width` is the horizontal ink-safe advance consumed by
            // the native rasterizer. Preserve the font's right italic overhang
            // here while carrying `italic` separately as the limits slant.
            width: width + italic,
            height,
            depth,
            content: BoxContent::Glyph { font_id, char_code },
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
        "\\oiint" => return '\u{222C}',  // ∬ (double integral)
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
    let (name, symbol, body, suppress_base_shift, is_operator_name) = match base_node {
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
            false,
        ),
        ParseNode::OperatorName { body, .. } => (None, false, Some(body.as_slice()), false, true),
        _ => return layout_supsub(Some(base_node), sup_node, sub_node, options, None),
    };

    let (base_box, slant) = if is_operator_name {
        (
            layout_operatorname(body.expect("operatorname body"), options),
            0.0,
        )
    } else {
        build_op_base(name, symbol, body, options)
    };
    // baseShift only applies to symbol operators (KaTeX: base instanceof SymbolNode)
    let base_shift = if symbol && !suppress_base_shift {
        compute_op_base_shift(&base_box, options)
    } else {
        0.0
    };

    layout_op_limits_inner(
        &base_box,
        sup_node,
        sub_node,
        slant,
        base_shift,
        !suppress_base_shift,
        options,
    )
}

/// Assemble an operator with limits above/below (KaTeX's `assembleSupSub`).
fn layout_op_limits_inner(
    base: &LayoutBox,
    sup_node: Option<&ParseNode>,
    sub_node: Option<&ParseNode>,
    slant: f64,
    base_shift: f64,
    include_dom_strut: bool,
    options: &LayoutOptions,
) -> LayoutBox {
    let metrics = options.metrics();
    let sup_style = options.style.superscript();
    let sub_style = options.style.subscript();

    let sup_ratio = sup_style.size_multiplier() / options.style.size_multiplier();
    let sub_ratio = sub_style.size_multiplier() / options.style.size_multiplier();

    let sup_data = sup_node.map(|s| {
        let sup_opts = options.with_style(sup_style);
        let elem = layout_node(s, &sup_opts);
        let kern = metrics
            .big_op_spacing1
            .max(metrics.big_op_spacing3 - elem.depth * sup_ratio);
        (elem, kern)
    });

    let sub_data = sub_node.map(|s| {
        let sub_opts = options.with_style(sub_style);
        let elem = layout_node(s, &sub_opts);
        let kern = metrics
            .big_op_spacing2
            .max(metrics.big_op_spacing4 - elem.height * sub_ratio);
        (elem, kern)
    });

    let sp5 = metrics.big_op_spacing5;
    // KaTeX's ordinary op-limits CSS vlist contributes a two-rule strut
    // outside the visible limit gap. Synthetic `\overset`/`\underset`
    // operators suppress that base/strut path. Keep the box extent separate
    // from `sup_kern`/`sub_kern`: native ink uses the font-derived kern above,
    // while enclosing layout sees the same ascent/depth as the KaTeX DOM.
    let limit_strut = if include_dom_strut {
        2.0 * metrics.default_rule_thickness
    } else {
        0.0
    };

    let (total_height, total_depth, total_width) = match (&sup_data, &sub_data) {
        (Some((sup_elem, sup_kern)), Some((sub_elem, sub_kern))) => {
            // Both sup and sub: VList from bottom
            // [sp5, sub, sub_kern, base, sup_kern, sup, sp5]
            let sup_h = sup_elem.height * sup_ratio;
            let sup_d = sup_elem.depth * sup_ratio;
            let sub_h = sub_elem.height * sub_ratio;
            let sub_d = sub_elem.depth * sub_ratio;

            let bottom = sp5 + sub_h + sub_d + sub_kern + base.depth + base_shift;

            let height = bottom + base.height - base_shift + sup_kern + sup_h + sup_d + sp5
                - (base.height + base.depth);

            let total_h = base.height - base_shift + sup_kern + sup_h + sup_d + sp5 + limit_strut;
            let total_d = bottom + limit_strut;

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
            let total_d = base.depth + base_shift + sub_kern + sub_h + sub_d + sp5 + limit_strut;

            let w = base.width.max(sub_elem.width * sub_ratio);
            (total_h, total_d, w)
        }
        (Some((sup_elem, sup_kern)), None) => {
            // Sup only: VList from bottom
            // [base, sup_kern, sup, sp5]
            let sup_h = sup_elem.height * sup_ratio;
            let sup_d = sup_elem.depth * sup_ratio;

            let total_h = base.height - base_shift + sup_kern + sup_h + sup_d + sp5 + limit_strut;
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

/// Extract the italic correction only when this box itself is a glyph.
///
/// This distinction mirrors KaTeX's `base instanceof SymbolNode` check.  Recursing through an
/// `HBox` would incorrectly treat text/group spans as symbols and pull their subscripts left.
fn direct_glyph_italic(lb: &LayoutBox) -> Option<f64> {
    match &lb.content {
        BoxContent::Glyph { font_id, char_code } => {
            get_char_metrics(*font_id, *char_code).map(|m| m.italic)
        }
        _ => None,
    }
}

/// Find the first glyph inside a box.
///
/// KaTeX special-cases `\oiint` and `\oiiint`: their overlay makes the base a span rather than a
/// `SymbolNode`, but the underlying integral glyph's italic correction still applies.
fn first_glyph_italic(lb: &LayoutBox) -> Option<f64> {
    match &lb.content {
        BoxContent::Glyph { .. } => direct_glyph_italic(lb),
        BoxContent::HBox(children) => children.iter().find_map(first_glyph_italic),
        _ => None,
    }
}

/// Extract the skew (italic correction) of the innermost/last glyph in a box.
/// Used by shifty accents (\hat, \tilde…) to horizontally centre the mark
/// over italic math letters (e.g. M in MathItalic has skew ≈ 0.083em).
/// KaTeX `groupLength` for wide SVG accents: `ordgroup.body.length`, else 1.
fn accent_ordgroup_len(base: &ParseNode) -> usize {
    match base {
        ParseNode::OrdGroup { body, .. } => body.len().max(1),
        _ => 1,
    }
}

fn glyph_skew(lb: &LayoutBox) -> f64 {
    let mut current = lb;
    loop {
        match &current.content {
            BoxContent::Glyph { font_id, char_code } => {
                return get_char_metrics(*font_id, *char_code)
                    .map(|m| m.skew)
                    .unwrap_or(0.0);
            }
            BoxContent::HBox(children) => match children.last() {
                Some(last) => current = last,
                None => return 0.0,
            },
            BoxContent::GlyphRun { glyphs } => {
                return glyphs
                    .last()
                    .and_then(|glyph| get_char_metrics(glyph.font_id, glyph.char_code))
                    .map(|metrics| metrics.skew)
                    .unwrap_or(0.0);
            }
            _ => return 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StretchyPlacement {
    Above,
    Below,
}

#[derive(Debug, Clone, Copy)]
struct StretchySpec {
    placement: StretchyPlacement,
    gap: f64,
}

impl StretchySpec {
    fn accent(label: &str, is_below: bool, options: &LayoutOptions) -> Self {
        Self {
            placement: if is_below {
                StretchyPlacement::Below
            } else {
                StretchyPlacement::Above
            },
            // KaTeX accentunder.ts assigns only \utilde a 0.12em kern.
            gap: if label == "\\utilde" {
                3.0 * options.metrics().default_rule_thickness
            } else {
                0.0
            },
        }
    }

    fn horizontal_brace(is_over: bool, options: &LayoutOptions) -> Self {
        Self {
            placement: if is_over {
                StretchyPlacement::Above
            } else {
                StretchyPlacement::Below
            },
            // KaTeX's 0.1em brace kern is 2.5 default rule units.
            gap: 2.5 * options.metrics().default_rule_thickness,
        }
    }
}

fn padded_to_width(body: LayoutBox, width: f64) -> LayoutBox {
    if width <= body.width + f64::EPSILON {
        return body;
    }
    let pad = (width - body.width) / 2.0;
    make_hbox(vec![
        LayoutBox::new_kern(pad),
        body,
        LayoutBox::new_kern(pad),
    ])
}

/// Dedicated geometry for AMSMath's triple/quadruple dot accents.
///
/// KaTeX exposes these commands through an `\overset` macro whose payload is a
/// run of text periods. Keeping that expansion in the AST makes the result
/// depend on text shaping, sizing resets, and operator-limit layout. Build the
/// actual dot disks here instead: their advance and clearance are stable math
/// geometry and no longer inherit text-run behavior.
fn layout_multidot_accent(body: LayoutBox, dot_count: usize, options: &LayoutOptions) -> LayoutBox {
    // The KaTeX macro wraps its dot payload in \normalsize. Math style and
    // explicit \tiny...\Huge sizing both scale this entire box later, so
    // compensate for both to keep only the mark at normal text size.
    let normal_size_scale = 1.0 / (options.size_multiplier() * options.explicit_size_multiplier);
    let advance = get_char_metrics(FontId::MainRegular, '.' as u32)
        .map(|metrics| metrics.width)
        .unwrap_or(0.27778)
        * normal_size_scale;
    let radius = 0.07_f64 * normal_size_scale;
    let mark_width = advance * dot_count as f64;
    let mark_height = radius * 2.0;
    let bezier = 0.552_284_749_8_f64;
    let mut commands = Vec::with_capacity(dot_count * 6);

    for index in 0..dot_count {
        let cx = advance * (index as f64 + 0.5);
        let cy = -radius;
        commands.extend([
            PathCommand::MoveTo {
                x: cx + radius,
                y: cy,
            },
            PathCommand::CubicTo {
                x1: cx + radius,
                y1: cy - bezier * radius,
                x2: cx + bezier * radius,
                y2: cy - radius,
                x: cx,
                y: cy - radius,
            },
            PathCommand::CubicTo {
                x1: cx - bezier * radius,
                y1: cy - radius,
                x2: cx - radius,
                y2: cy - bezier * radius,
                x: cx - radius,
                y: cy,
            },
            PathCommand::CubicTo {
                x1: cx - radius,
                y1: cy + bezier * radius,
                x2: cx - bezier * radius,
                y2: cy + radius,
                x: cx,
                y: cy + radius,
            },
            PathCommand::CubicTo {
                x1: cx + bezier * radius,
                y1: cy + radius,
                x2: cx + radius,
                y2: cy + bezier * radius,
                x: cx + radius,
                y: cy,
            },
            PathCommand::Close,
        ]);
    }

    let body = padded_to_width(body, mark_width);
    let mark = LayoutBox {
        width: mark_width,
        height: mark_height,
        depth: 0.0,
        content: BoxContent::SvgPath {
            commands,
            fill: true,
        },
        color: options.color,
    };
    let clearance = body.height + 3.0 * options.metrics().default_rule_thickness;

    LayoutBox {
        width: body.width,
        height: clearance + mark_height,
        depth: body.depth,
        content: BoxContent::Accent {
            base: Box::new(body),
            accent: Box::new(mark),
            clearance,
            skew: 0.0,
            is_below: false,
            under_gap_em: 0.0,
        },
        color: options.color,
    }
}

/// Place a top-origin stretchy path with a shared clearance model.
fn compose_stretchy(
    base: LayoutBox,
    accent: LayoutBox,
    spec: StretchySpec,
    skew: f64,
    color: Color,
) -> LayoutBox {
    debug_assert!(accent.height.abs() <= 0.001);
    debug_assert!((base.width - accent.width).abs() <= 0.001);

    let is_below = spec.placement == StretchyPlacement::Below;
    let clearance = if is_below {
        base.height + base.depth + spec.gap
    } else {
        base.height + spec.gap
    };
    let (height, depth) = if is_below {
        (base.height, base.depth + spec.gap + accent.depth)
    } else {
        (base.height + spec.gap + accent.depth, base.depth)
    };

    LayoutBox {
        width: base.width,
        height,
        depth,
        content: BoxContent::Accent {
            base: Box::new(base),
            accent: Box::new(accent),
            clearance,
            skew,
            is_below,
            under_gap_em: if is_below { spec.gap } else { 0.0 },
        },
        color,
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

    if matches!(label, "\\dddot" | "\\ddddot") {
        return layout_multidot_accent(body_box, if label == "\\dddot" { 3 } else { 4 }, options);
    }

    // Try KaTeX exact SVG paths first (widehat, widetilde, overgroup, etc.).
    // The path width must stay inside the accent box; renderers size their canvas from
    // DisplayList.width, so drawing wider than the box clips in an enclosing HBox.
    let accent_target_w =
        base_w.max(crate::katex_svg::katex_stretchy_min_width_em(label).unwrap_or(0.0));
    if let Some((commands, w, h, fill)) =
        crate::katex_svg::katex_accent_path(label, accent_target_w, accent_ordgroup_len(base))
    {
        // KaTeX paths use SVG coords (y down): height=0, depth=h
        let accent_box = LayoutBox {
            width: w,
            height: 0.0,
            depth: h,
            content: BoxContent::SvgPath { commands, fill },
            color: options.color,
        };
        if label != "\\vec" {
            let body_box = padded_to_width(body_box, w);
            return compose_stretchy(
                body_box,
                accent_box,
                StretchySpec::accent(label, is_below, options),
                0.0,
                options.color,
            );
        }

        // KaTeX `accent.ts` uses `clearance = min(body.height, xHeight)` for ordinary accents.
        // KaTeX: clearance = min(body.height, xHeight) is used as *overlap* (kern down).
        // Equivalent RaTeX position: vec bottom = body.height - overlap.
        let clearance = (body_box.height - options.metrics().x_height).max(0.0);
        let (height, depth) = (clearance + h, body_box.depth);
        let vec_skew = (if is_shifty {
            glyph_skew(&body_box)
        } else {
            0.0
        }) + VEC_SKEW_EXTRA_RIGHT_EM;
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
                under_gap_em: 0.0,
            },
            color: options.color,
        };
    }

    // Arrow-type stretchy accents (overrightarrow, etc.)
    let use_arrow_path = is_stretchy && is_arrow_accent(label);

    let accent_box = if use_arrow_path {
        let (commands, arrow_h, fill_arrow) =
            match crate::katex_svg::katex_stretchy_path(label, accent_target_w) {
                Some((c, h)) => (c, h, true),
                None => {
                    let h = 0.3_f64;
                    let c = stretchy_accent_path(label, accent_target_w, h);
                    let fill = label == "\\xtwoheadrightarrow" || label == "\\xtwoheadleftarrow";
                    (c, h, fill)
                }
            };
        LayoutBox {
            width: accent_target_w,
            height: 0.0,
            depth: arrow_h,
            content: BoxContent::SvgPath {
                commands: shift_path_y(commands, arrow_h / 2.0),
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

    if use_arrow_path {
        let body_box = padded_to_width(body_box, accent_box.width);
        return compose_stretchy(
            body_box,
            accent_box,
            StretchySpec::accent(label, is_below, options),
            0.0,
            options.color,
        );
    }

    let skew = if is_shifty {
        // For shifty accents (\hat, \tilde, etc.) shift by the BASE character's skew,
        // which encodes the italic correction in math-italic fonts (e.g. M → 0.083em).
        glyph_skew(&body_box)
    } else {
        0.0
    };

    let clearance = if is_below {
        body_box.height + body_box.depth + accent_box.depth
    } else {
        // Clearance = how high above baseline the accent is positioned.
        // - For simple letters (M, b, o): body_box.height is the letter top → use directly.
        // - For a body that is itself an above-accent (\r{a}, `\tilde{\tilde{x}}`, …):
        //   use the same kern basis as a plain base (`max(0, body.height - xHeight) +
        //   correction`, with `\bar`/`\=` exceptions) instead of `inner_clearance + ε`, which
        //   double-counted stacked accent depths and inflated nested spacing vs KaTeX.
        let base_clearance = match &body_box.content {
            BoxContent::Accent {
                clearance: inner_cl,
                is_below,
                accent: inner_accent,
                ..
            } if !is_below => {
                // For SVG accents (height≈0, e.g. \vec): body_box.height = clearance + H_EM,
                // which matches KaTeX's body.height. Use min(body.height, xHeight) exactly as
                // KaTeX does: clearance = min(body.height, xHeight).
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
                    // `inner_cl` already includes the inner accent glyph's depth. Using
                    // `inner_cl + ε` stacked another full kern on top (e.g. `\tilde{\tilde{x}}`
                    // blew up to ~1.32em vs KaTeX ~0.90em). KaTeX recomputes clearance from the
                    // built inner span via `min(body.height, xHeight)`; matching the non-nested
                    // glyph path (`max(0, body.height - xHeight) + correction`) tracks that.
                    if label == "\\bar" || label == "\\=" {
                        body_box.height
                    } else {
                        // `\hat{x}` / `\dot{x}` / … enforce a 0.78056em strut so `body_box.height`
                        // exceeds the ink top, while KaTeX's nested accent still uses the inner
                        // span height (~0.6944em) for clearance. `\tilde{x}` keeps body ≈ visual
                        // top, so we keep `body_box.height` when it is not strut-inflated.
                        let inner_visual_top = inner_cl + 0.35_f64.min(inner_accent.height);
                        let h_for_kern = if body_box.height > inner_visual_top + 0.002 {
                            inner_visual_top
                        } else {
                            body_box.height
                        };
                        let katex_pos = (h_for_kern - options.metrics().x_height).max(0.0);
                        let correction =
                            (accent_box.height - 0.35_f64.min(accent_box.height)).max(0.0);
                        katex_pos + correction
                    }
                }
            }
            _ => {
                // KaTeX positions glyph accents by kerning DOWN by
                // min(body.height, xHeight), so the accent baseline sits at
                //   max(0, body.height - xHeight)
                // above the formula baseline.  This keeps the accent within the
                // body's height bounds for normal-height bases and produces a
                // formula height == body_height (accent adds no extra height),
                // matching KaTeX's VList.
                //
                // \bar / \= (macron) are an exception: for x-height bases (a, e, o, …)
                // body.height ≈ xHeight so katex_pos ≈ 0 and the bar sits on the letter
                // (golden \text{\={a}}).  Tie macron clearance to full body height like
                // the pre-62f7ba53 engine, then apply the same small kern as before.
                if label == "\\bar" || label == "\\=" {
                    body_box.height
                } else {
                    let katex_pos = (body_box.height - options.metrics().x_height).max(0.0);
                    let correction = (accent_box.height - 0.35_f64.min(accent_box.height)).max(0.0);
                    katex_pos + correction
                }
            }
        };
        // KaTeX VList places the accent so its depth-bottom edge sits at the kern
        // position.  The accent baseline is therefore depth higher than that edge.
        // Without this term, glyphs with non-zero depth (notably \tilde, depth=0.35)
        // are positioned too low, overlapping the base character.
        let base_clearance = base_clearance + accent_box.depth;
        if label == "\\bar" || label == "\\=" {
            (base_clearance - 0.12).max(0.0)
        } else {
            base_clearance
        }
    };

    let (height, depth) = if is_below {
        (
            body_box.height,
            body_box.depth + accent_box.height + accent_box.depth,
        )
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
            under_gap_em: 0.0,
        },
        color: options.color,
    }
}

// ============================================================================
// Left/Right stretchy delimiters
// ============================================================================

/// Returns true if the node (or any descendant in the current `\left...\right`
/// scope) is a Middle node.  A nested LeftRight starts its own scope, so its
/// `\middle`s are handled by that nested layout pass.
fn node_contains_middle(node: &ParseNode, style: MathStyle) -> bool {
    let mut pending = vec![(node, style)];
    while let Some((current, current_style)) = pending.pop() {
        match current {
            ParseNode::Middle { .. } => return true,
            ParseNode::OrdGroup { body, .. } | ParseNode::MClass { body, .. } => {
                pending.extend(body.iter().map(|node| (node, current_style)));
            }
            ParseNode::SupSub { base, sup, sub, .. } => {
                if let Some(base) = base {
                    pending.push((base, current_style));
                }
                if let Some(sup) = sup {
                    pending.push((sup, current_style.superscript()));
                }
                if let Some(sub) = sub {
                    pending.push((sub, current_style.subscript()));
                }
            }
            ParseNode::GenFrac { numer, denom, .. } => {
                pending.push((numer, current_style.numerator()));
                pending.push((denom, current_style.denominator()));
            }
            ParseNode::Sqrt { body, index, .. } => {
                pending.push((body, current_style.cramped()));
                if let Some(index) = index {
                    pending.push((index, current_style.superscript().superscript()));
                }
            }
            ParseNode::Accent { base, .. } | ParseNode::AccentUnder { base, .. } => {
                pending.push((base, current_style));
            }
            ParseNode::Op {
                body: Some(body), ..
            }
            | ParseNode::OperatorName { body, .. }
            | ParseNode::Color { body, .. }
            | ParseNode::Phantom { body, .. }
            | ParseNode::Pmb { body, .. }
            | ParseNode::Href { body, .. }
            | ParseNode::Html { body, .. } => {
                pending.extend(body.iter().map(|node| (node, current_style)));
            }
            // A nested LeftRight starts a new \middle scope.
            ParseNode::LeftRight { .. } => {}
            ParseNode::Font { body, .. }
            | ParseNode::Overline { body, .. }
            | ParseNode::Underline { body, .. }
            | ParseNode::VPhantom { body, .. }
            | ParseNode::Smash { body, .. }
            | ParseNode::Enclose { body, .. }
            | ParseNode::Lap { body, .. }
            | ParseNode::RaiseBox { body, .. }
            | ParseNode::VCenter { body, .. }
            | ParseNode::HorizBrace { base: body, .. } => {
                pending.push((body, current_style));
            }
            ParseNode::Text { body, .. } | ParseNode::Sizing { body, .. } => {
                pending.extend(body.iter().map(|node| (node, current_style.text())));
            }
            ParseNode::Styling {
                style: node_style,
                body,
                ..
            } => {
                let new_style = style_str_to_math_style(node_style);
                pending.extend(body.iter().map(|node| (node, new_style)));
            }
            ParseNode::Array { body, .. } => {
                pending.extend(
                    body.iter()
                        .flat_map(|row| row.iter())
                        .map(|node| (node, current_style)),
                );
            }
            ParseNode::XArrow { body, below, .. } => {
                pending.push((body, current_style.superscript()));
                if let Some(below) = below {
                    pending.push((below, current_style.subscript()));
                }
            }
            ParseNode::CdArrow {
                label_above,
                label_below,
                ..
            } => {
                if let Some(above) = label_above {
                    pending.push((above, current_style.superscript()));
                }
                if let Some(below) = label_below {
                    pending.push((below, current_style.subscript()));
                }
            }
            ParseNode::MathChoice {
                display,
                text,
                script,
                scriptscript,
                ..
            } => {
                let branch = match current_style {
                    MathStyle::Display | MathStyle::DisplayCramped => display,
                    MathStyle::Text | MathStyle::TextCramped => text,
                    MathStyle::Script | MathStyle::ScriptCramped => script,
                    MathStyle::ScriptScript | MathStyle::ScriptScriptCramped => scriptscript,
                };
                pending.extend(branch.iter().map(|node| (node, current_style)));
            }
            ParseNode::HtmlMathMl { html, .. } => {
                pending.extend(html.iter().map(|node| (node, current_style)));
            }
            _ => {}
        }
    }
    false
}

/// Returns true if any node in the slice contains a Middle node governed by the
/// current `\left...\right` pass.
fn body_contains_middle(nodes: &[ParseNode], style: MathStyle) -> bool {
    nodes.iter().any(|n| node_contains_middle(n, style))
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
        options.with_style(MathStyle::Script).metrics().delim2
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
    let (inner, total_height) = if body_contains_middle(body, options.style) {
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

#[derive(Debug, Clone, Copy)]
struct VertDelimGeometry {
    width: f64,
    height: f64,
    depth: f64,
    view_box_height: i64,
    middle_height: i64,
}

#[derive(Debug, Clone, Copy)]
enum VertDelimKind {
    Stretchy,
    Sized,
}

/// Fixed em scale of the locked KaTeX reference wrapper (`.katex` is 1.21em).
///
/// The golden renderer uses a 40px base em while the browser SVG is rasterized
/// through that wrapper. Apply one suite-wide scale to sized paths instead of
/// the previous height-dependent per-command multipliers.
const KATEX_SIZED_DELIM_SCALE: f64 = 1.2;

/// KaTeX `delimiter.makeStackedDelim`: total span of one repeat piece (U+2223 / U+2225) in Size1-Regular.
fn vert_repeat_piece_height(is_double: bool) -> f64 {
    let code = if is_double { 8741_u32 } else { 8739 };
    get_char_metrics(FontId::Size1Regular, code)
        .map(|m| m.height + m.depth)
        .unwrap_or(0.5)
}

/// Match KaTeX `makeStackedDelim` for stack-always `|` / `\Vert` delimiters.
///
/// This object is the single source of truth for the SVG viewBox, TeX box
/// bounds, and axis split. Keeping those values together avoids scaling the
/// path independently from the box that contains it.
fn stacked_vert_geometry(
    requested_total: f64,
    is_double: bool,
    kind: VertDelimKind,
    options: &LayoutOptions,
) -> VertDelimGeometry {
    let piece = vert_repeat_piece_height(is_double);
    let minimum_total = 2.0 * piece;
    let repeat_count = ((requested_total - minimum_total) / piece).ceil().max(0.0);
    let logical_total = minimum_total + repeat_count * piece;
    let axis = options.metrics().axis_height;
    let logical_depth = logical_total / 2.0 - axis;
    let logical_height = logical_total - logical_depth;
    let scale = match kind {
        VertDelimKind::Stretchy => 1.0,
        VertDelimKind::Sized => KATEX_SIZED_DELIM_SCALE,
    };

    VertDelimGeometry {
        width: if is_double { 0.556 } else { 0.333 },
        height: logical_height * scale,
        depth: logical_depth * scale,
        view_box_height: (logical_total * 1000.0).round() as i64,
        middle_height: ((logical_total - minimum_total) * 1000.0).round() as i64,
    }
}

/// KaTeX `svgGeometry.tallDelim` paths for `"vert"` / `"doublevert"` (viewBox units per em width).
fn tall_vert_svg_path_data(mid_th: i64, is_double: bool) -> String {
    let neg = -mid_th;
    if !is_double {
        format!(
            "M145 15 v585 v{mid_th} v585 c2.667,10,9.667,15,21,15 c10,0,16.667,-5,20,-15 v-585 v{neg} v-585 c-2.667,-10,-9.667,-15,-21,-15 c-10,0,-16.667,5,-20,15z M188 15 H145 v585 v{mid_th} v585 h43z"
        )
    } else {
        format!(
            "M145 15 v585 v{mid_th} v585 c2.667,10,9.667,15,21,15 c10,0,16.667,-5,20,-15 v-585 v{neg} v-585 c-2.667,-10,-9.667,-15,-21,-15 c-10,0,-16.667,5,-20,15z M188 15 H145 v585 v{mid_th} v585 h43z M367 15 v585 v{mid_th} v585 c2.667,10,9.667,15,21,15 c10,0,16.667,-5,20,-15 v-585 v{neg} v-585 c-2.667,-10,-9.667,-15,-21,-15 c-10,0,-16.667,5,-20,15z M410 15 H367 v585 v{mid_th} v585 h43z"
        )
    }
}

fn scale_svg_path_to_em(cmds: &[PathCommand]) -> Vec<PathCommand> {
    let s = 0.001_f64;
    cmds.iter()
        .map(|c| match *c {
            PathCommand::MoveTo { x, y } => PathCommand::MoveTo { x: x * s, y: y * s },
            PathCommand::LineTo { x, y } => PathCommand::LineTo { x: x * s, y: y * s },
            PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => PathCommand::CubicTo {
                x1: x1 * s,
                y1: y1 * s,
                x2: x2 * s,
                y2: y2 * s,
                x: x * s,
                y: y * s,
            },
            PathCommand::QuadTo { x1, y1, x, y } => PathCommand::QuadTo {
                x1: x1 * s,
                y1: y1 * s,
                x: x * s,
                y: y * s,
            },
            PathCommand::Close => PathCommand::Close,
        })
        .collect()
}

/// Map KaTeX top-origin SVG y (after ×0.001) to RaTeX baseline coords (top −height, bottom +depth).
fn map_vert_path_y_to_baseline(
    cmds: Vec<PathCommand>,
    height: f64,
    depth: f64,
    view_box_height: i64,
) -> Vec<PathCommand> {
    let span_em = view_box_height as f64 / 1000.0;
    let total = height + depth;
    let scale_y = if span_em > 0.0 { total / span_em } else { 1.0 };
    cmds.into_iter()
        .map(|c| match c {
            PathCommand::MoveTo { x, y } => PathCommand::MoveTo {
                x,
                y: -height + y * scale_y,
            },
            PathCommand::LineTo { x, y } => PathCommand::LineTo {
                x,
                y: -height + y * scale_y,
            },
            PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => PathCommand::CubicTo {
                x1,
                y1: -height + y1 * scale_y,
                x2,
                y2: -height + y2 * scale_y,
                x,
                y: -height + y * scale_y,
            },
            PathCommand::QuadTo { x1, y1, x, y } => PathCommand::QuadTo {
                x1,
                y1: -height + y1 * scale_y,
                x,
                y: -height + y * scale_y,
            },
            PathCommand::Close => PathCommand::Close,
        })
        .collect()
}

fn make_shortmid_box(options: &LayoutOptions) -> LayoutBox {
    let total = 0.675_f64;
    let axis = options.metrics().axis_height;
    let depth = (total / 2.0 - axis).max(0.0);
    let height = total - depth;
    let raw = parse_svg_path_data(&tall_vert_svg_path_data(0, false));
    let scaled = scale_svg_path_to_em(&raw);
    let commands = map_vert_path_y_to_baseline(scaled, height, depth, 1170);
    LayoutBox {
        width: 0.22222,
        height,
        depth,
        content: BoxContent::SvgPath {
            commands,
            fill: true,
        },
        color: options.color,
    }
}

/// Natural Main-Regular U+2223/U+2225 delimiter.
///
/// KaTeX traverses the small-glyph sequence before choosing a stacked SVG.
/// Keeping the natural form as a glyph preserves its font hinting; only the
/// stacked form needs path geometry.
fn make_natural_vert_delim_box(is_double: bool, options: &LayoutOptions) -> LayoutBox {
    let char_code = if is_double { 8741 } else { 8739 };
    let metrics = get_char_metrics(FontId::MainRegular, char_code)
        .expect("KaTeX Main-Regular vertical delimiter metrics");

    LayoutBox {
        width: metrics.width,
        height: metrics.height,
        depth: metrics.depth,
        content: BoxContent::Glyph {
            font_id: FontId::MainRegular,
            char_code,
        },
        color: options.color,
    }
}

fn make_textunderscore_box(options: &LayoutOptions) -> LayoutBox {
    LayoutBox {
        width: 0.65,
        height: 0.0,
        depth: 0.31,
        content: BoxContent::Rule {
            thickness: 0.08,
            raise: -0.31,
        },
        color: options.color,
    }
}

fn make_corresponds_symbol(options: &LayoutOptions) -> LayoutBox {
    let width = 1.16;
    let height = 0.55;
    let depth = 0.24;
    let rule_thickness = 0.055;
    let arc = LayoutBox {
        width,
        height: 0.36,
        depth: 0.0,
        content: BoxContent::SvgPath {
            commands: vec![
                PathCommand::MoveTo { x: 0.02, y: 0.0 },
                PathCommand::CubicTo {
                    x1: 0.18,
                    y1: -0.34,
                    x2: 0.64,
                    y2: -0.34,
                    x: 0.80,
                    y: 0.0,
                },
            ],
            fill: false,
        },
        color: options.color,
    };

    LayoutBox {
        width,
        height,
        depth,
        content: BoxContent::ProofTree {
            children: vec![PlacedBox {
                box_: arc,
                x: 0.0,
                baseline_y: 0.36,
            }],
            rules: vec![
                ProofRule {
                    x: 0.28,
                    y: 0.48,
                    width: 0.86,
                    thickness: rule_thickness,
                    dashed: false,
                },
                ProofRule {
                    x: 0.28,
                    y: 0.68,
                    width: 0.86,
                    thickness: rule_thickness,
                    dashed: false,
                },
            ],
        },
        color: options.color,
    }
}

fn make_measured_by_symbol(options: &LayoutOptions) -> LayoutBox {
    let width = 0.98;
    let height = 0.56;
    let depth = 0.30;
    let rule_thickness = 0.055;
    let m_metrics = get_char_metrics(FontId::MainRegular, 'm' as u32);
    let (m_width, m_height, m_depth) = m_metrics
        .map(|m| (m.width, m.height, m.depth))
        .unwrap_or((0.78, 0.43, 0.0));
    let m_scale = 0.61;
    let m_glyph = LayoutBox {
        width: m_width,
        height: m_height,
        depth: m_depth,
        content: BoxContent::Glyph {
            font_id: FontId::MainRegular,
            char_code: 'm' as u32,
        },
        color: options.color,
    };
    let m_box = LayoutBox {
        width: m_width * m_scale,
        height: m_height * m_scale,
        depth: m_depth * m_scale,
        content: BoxContent::Scaled {
            body: Box::new(m_glyph),
            child_scale: m_scale,
        },
        color: options.color,
    };

    LayoutBox {
        width,
        height,
        depth,
        content: BoxContent::ProofTree {
            children: vec![PlacedBox {
                box_: m_box,
                x: (width - m_width * m_scale) / 2.0,
                baseline_y: 0.02 + m_height * m_scale,
            }],
            rules: vec![
                ProofRule {
                    x: 0.06,
                    y: 0.55,
                    width: 0.86,
                    thickness: rule_thickness,
                    dashed: false,
                },
                ProofRule {
                    x: 0.06,
                    y: 0.78,
                    width: 0.86,
                    thickness: rule_thickness,
                    dashed: false,
                },
            ],
        },
        color: options.color,
    }
}

/// Build a vertical-bar delimiter LayoutBox using the same SVG as KaTeX `tallDelim` (`vert` / `doublevert`).
/// `total_height` is the requested full span in em (`sizeToMaxHeight` for `\big`/`\Big`/…).
fn make_vert_delim_box(
    total_height: f64,
    is_double: bool,
    kind: VertDelimKind,
    options: &LayoutOptions,
) -> LayoutBox {
    let geometry = stacked_vert_geometry(total_height, is_double, kind, options);
    let d = tall_vert_svg_path_data(geometry.middle_height, is_double);
    let raw = parse_svg_path_data(&d);
    let scaled = scale_svg_path_to_em(&raw);
    let commands = map_vert_path_y_to_baseline(
        scaled,
        geometry.height,
        geometry.depth,
        geometry.view_box_height,
    );

    LayoutBox {
        width: geometry.width,
        height: geometry.height,
        depth: geometry.depth,
        content: BoxContent::SvgPath {
            commands,
            fill: true,
        },
        color: options.color,
    }
}

/// Select a delimiter glyph large enough for the given total height.
fn make_stretchy_delim(delim: &str, total_height: f64, options: &LayoutOptions) -> LayoutBox {
    if delim == "." || delim.is_empty() {
        return LayoutBox::new_kern(0.0);
    }

    // stackAlwaysDelimiters: use SVG path only when the required height exceeds
    // the natural font-glyph height (1.0em for single vert, same for double).
    // Natural bars keep font hinting; stacked bars use the unified path/box
    // geometry above.
    const VERT_NATURAL_HEIGHT: f64 = 1.0; // MainRegular |: 0.75+0.25
    if is_vert_delim(delim) {
        return if total_height > VERT_NATURAL_HEIGHT {
            make_vert_delim_box(total_height, false, VertDelimKind::Stretchy, options)
        } else {
            make_natural_vert_delim_box(false, options)
        };
    }
    if is_double_vert_delim(delim) {
        return if total_height > VERT_NATURAL_HEIGHT {
            make_vert_delim_box(total_height, true, VertDelimKind::Stretchy, options)
        } else {
            make_natural_vert_delim_box(true, options)
        };
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
        return make_vert_delim_box(total, false, VertDelimKind::Sized, options);
    }
    if is_double_vert_delim(delim) {
        let total = SIZE_TO_MAX_HEIGHT[size.min(4) as usize];
        return make_vert_delim_box(total, true, VertDelimKind::Sized, options);
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

/// Shared row/column measurement for table-like layouts.
///
/// Arrays and commutative diagrams differ in how their cells are built, but
/// they must agree on the geometry contract consumed by `BoxContent::Array`:
/// each column owns one maximum width and each row owns one ascent/depth pair.
#[derive(Debug, Clone)]
struct GridMetrics {
    col_widths: Vec<f64>,
    row_heights: Vec<f64>,
    row_depths: Vec<f64>,
}

#[derive(Debug, Clone, Copy)]
struct GridDimensions {
    width: f64,
    total_height: f64,
    offset: f64,
}

impl GridMetrics {
    fn measure(
        cells: &[Vec<LayoutBox>],
        num_cols: usize,
        min_row_height: f64,
        min_row_depth: f64,
    ) -> Self {
        let mut col_widths = vec![0.0_f64; num_cols];
        let mut row_heights = Vec::with_capacity(cells.len());
        let mut row_depths = Vec::with_capacity(cells.len());

        for row in cells {
            let mut row_height = min_row_height;
            let mut row_depth = min_row_depth;
            for (column, cell) in row.iter().take(num_cols).enumerate() {
                col_widths[column] = col_widths[column].max(cell.width);
                row_height = row_height.max(cell.height);
                row_depth = row_depth.max(cell.depth);
            }
            row_heights.push(row_height);
            row_depths.push(row_depth);
        }

        Self {
            col_widths,
            row_heights,
            row_depths,
        }
    }

    fn dimensions(
        &self,
        col_gaps: &[f64],
        axis_height: f64,
        content_x_offset: f64,
        content_x_end_offset: f64,
    ) -> GridDimensions {
        debug_assert_eq!(col_gaps.len(), self.col_widths.len().saturating_sub(1));
        debug_assert_eq!(self.row_heights.len(), self.row_depths.len());

        let width = self.col_widths.iter().sum::<f64>()
            + col_gaps.iter().sum::<f64>()
            + content_x_offset
            + content_x_end_offset;
        let total_height = self
            .row_heights
            .iter()
            .zip(&self.row_depths)
            .map(|(height, depth)| height + depth)
            .sum::<f64>();

        GridDimensions {
            width,
            total_height,
            offset: total_height / 2.0 + axis_height,
        }
    }
}

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
    tags: Option<&[ArrayTag]>,
    _leqno: bool,
    options: &LayoutOptions,
) -> LayoutBox {
    let metrics = options.metrics();
    let pt = 1.0 / metrics.pt_per_em;
    let baselineskip = 12.0 * pt;
    let jot = 3.0 * pt;
    let arrayskip = arraystretch * baselineskip;
    let arstrut_h = 0.7 * arrayskip;
    let arstrut_d = 0.3 * arrayskip;
    let default_side_gap = match col_sep_type {
        Some("small") => {
            // smallmatrix: thickspace × (script_multiplier / current_multiplier)
            // KaTeX: arraycolsep = 0.2778em × (scriptMultiplier / sizeMultiplier)
            mu_to_em(5.0, metrics.quad) * MathStyle::Script.size_multiplier()
                / options.size_multiplier()
        }
        Some("align") | Some("alignat") => 0.0,
        _ => 5.0 * pt, // arraycolsep
    };
    let cell_options = options.clone();

    let num_rows = body.len();
    if num_rows == 0 {
        return LayoutBox::new_empty();
    }

    let num_cols = body.iter().map(|r| r.len()).max().unwrap_or(0);

    // Extract per-column alignment and column separators from cols spec.
    use ratex_parser::parse_node::AlignType;
    let align_specs: Vec<&ratex_parser::parse_node::AlignSpec> = cols
        .map(|cs| {
            cs.iter()
                .filter(|s| matches!(s.align_type, AlignType::Align))
                .collect()
        })
        .unwrap_or_default();
    let col_aligns: Vec<u8> = {
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
    let side_gaps: Vec<(f64, f64)> = (0..num_cols)
        .map(|c| {
            let spec = align_specs.get(c);
            (
                spec.and_then(|s| s.pregap).unwrap_or(default_side_gap),
                spec.and_then(|s| s.postgap).unwrap_or(default_side_gap),
            )
        })
        .collect();
    let col_gaps: Vec<f64> = (0..num_cols.saturating_sub(1))
        .map(|c| side_gaps[c].1 + side_gaps[c + 1].0)
        .collect();

    // Detect vertical separator positions in the column spec.
    // col_separators[i]: None = no rule, Some(false) = solid '|', Some(true) = dashed ':'.
    let col_separators: Vec<Option<bool>> = {
        let mut seps = vec![None; num_cols + 1];
        let mut align_count = 0usize;
        if let Some(cs) = cols {
            for spec in cs {
                match spec.align_type {
                    AlignType::Align => align_count += 1,
                    AlignType::Separator
                        if spec.align.as_deref() == Some("|") && align_count <= num_cols =>
                    {
                        seps[align_count] = Some(false);
                    }
                    AlignType::Separator
                        if spec.align.as_deref() == Some(":") && align_count <= num_cols =>
                    {
                        seps[align_count] = Some(true);
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
    for row in body {
        let mut row_boxes = Vec::with_capacity(num_cols);

        for cell in row {
            let cell_nodes = match cell {
                ParseNode::OrdGroup { body, .. } => body.as_slice(),
                other => std::slice::from_ref(other),
            };
            let cell_box = layout_expression(cell_nodes, &cell_options, true);
            row_boxes.push(cell_box);
        }

        // Pad missing columns
        while row_boxes.len() < num_cols {
            row_boxes.push(LayoutBox::new_empty());
        }

        cell_boxes.push(row_boxes);
    }

    let GridMetrics {
        col_widths,
        mut row_heights,
        mut row_depths,
    } = GridMetrics::measure(&cell_boxes, num_cols, arstrut_h, arstrut_d);

    // KaTeX adds \jot between rows, never below the final row.
    if add_jot {
        for row_depth in row_depths.iter_mut().take(num_rows.saturating_sub(1)) {
            *row_depth += jot;
        }
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

    // Extra x padding before col 0 and after last col (hskip_before_and_after).
    let content_x_offset = if hskip {
        side_gaps.first().map(|g| g.0).unwrap_or(0.0)
    } else {
        0.0
    };
    let content_x_end_offset = if hskip {
        side_gaps.last().map(|g| g.1).unwrap_or(0.0)
    } else {
        0.0
    };

    // Width of the cell grid including horizontal padding (no tag column).
    let grid_dimensions = GridMetrics {
        col_widths: col_widths.clone(),
        row_heights: row_heights.clone(),
        row_depths: row_depths.clone(),
    }
    .dimensions(
        &col_gaps,
        metrics.axis_height,
        content_x_offset,
        content_x_end_offset,
    );
    let array_inner_width = grid_dimensions.width;

    let mut row_tag_boxes: Vec<Option<LayoutBox>> = (0..num_rows).map(|_| None).collect();
    let mut tag_col_width = 0.0_f64;
    let text_opts = options.with_style(options.style.text());
    if let Some(tag_slice) = tags {
        if tag_slice.len() == num_rows {
            for (r, t) in tag_slice.iter().enumerate() {
                if let ArrayTag::Explicit(nodes) = t {
                    if !nodes.is_empty() {
                        let tb = layout_expression(nodes, &text_opts, true);
                        tag_col_width = tag_col_width.max(tb.width);
                        row_tag_boxes[r] = Some(tb);
                    }
                }
            }
        }
    }
    let tag_gap_em = if tag_col_width > 0.0 {
        // Compact automatic equation numbers need the extra 0.1em reserved by
        // KaTeX's block tag column; wide explicit tags retain the standard 1em.
        let gap_factor = if tag_col_width > 2.0 { 1.0 } else { 1.1 };
        gap_factor * text_opts.metrics().quad
    } else {
        0.0
    };
    // leqno (tags on the left) is parsed but not yet laid out; keep tags on the right.
    let tags_left = false;

    let total_width = array_inner_width + tag_gap_em + tag_col_width;

    let height = grid_dimensions.offset;
    let depth = grid_dimensions.total_height - grid_dimensions.offset;

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
            col_gaps: col_gaps.into_boxed_slice(),
            offset: grid_dimensions.offset,
            content_x_offset,
            content_x_end_offset,
            col_separators,
            hlines_before_row,
            rule_thickness,
            double_rule_sep,
            array_inner_width,
            tag_gap_em,
            tag_col_width,
            row_tags: row_tag_boxes,
            tags_left,
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

    // KaTeX `Options.havingSize`: inner is built in `this.style.text()` (≥ textstyle).
    // Keep the requested size as absolute layout state so nested sizing resets
    // replace, rather than compound with, the surrounding explicit size.
    let current_multiplier = options.size_multiplier() * options.explicit_size_multiplier;
    let inner_opts = options
        .with_style(options.style.text())
        .with_explicit_size_multiplier(multiplier);
    let inner = layout_expression(body, &inner_opts, true);
    let ratio = multiplier / current_multiplier;
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

#[derive(Default)]
struct HtmlStyle {
    color: Option<Color>,
    font_size_scale: Option<f64>,
    bold: bool,
    italic: bool,
    background_color: Option<Color>,
    underline: bool,
}

fn layout_html(
    attributes: &HashMap<String, String>,
    body: &[ParseNode],
    options: &LayoutOptions,
) -> LayoutBox {
    let style = attributes
        .get("style")
        .map(|style| parse_html_style(style))
        .unwrap_or_default();

    let body_options = match style.color {
        Some(color) => options.with_color(color),
        None => options.clone(),
    };
    let font_id = match (style.bold, style.italic) {
        (true, true) => Some(FontId::MainBoldItalic),
        (true, false) => Some(FontId::MainBold),
        (false, true) => Some(FontId::MainItalic),
        (false, false) => None,
    };

    let body_node = ParseNode::OrdGroup {
        mode: body.first().map(ParseNode::mode).unwrap_or(Mode::Math),
        body: body.to_vec(),
        semisimple: None,
        loc: None,
    };
    let mut lbox = match font_id {
        Some(font_id) => layout_with_font(&body_node, font_id, &body_options),
        None => layout_expression(body, &body_options, true),
    };

    if let Some(scale) = style.font_size_scale {
        if (scale - 1.0).abs() >= 0.001 {
            lbox = LayoutBox {
                width: lbox.width * scale,
                height: lbox.height * scale,
                depth: lbox.depth * scale,
                content: BoxContent::Scaled {
                    body: Box::new(lbox),
                    child_scale: scale,
                },
                color: body_options.color,
            };
        }
    }

    if let Some(background_color) = style.background_color {
        lbox = LayoutBox {
            width: lbox.width,
            height: lbox.height,
            depth: lbox.depth,
            content: BoxContent::Framed {
                body: Box::new(lbox),
                padding: 0.0,
                border_thickness: 0.0,
                has_border: false,
                bg_color: Some(background_color),
                border_color: Color::BLACK,
            },
            color: body_options.color,
        };
    }

    if style.underline {
        lbox = layout_underline_laid_out(lbox, options, body_options.color);
    }

    if options.leftright_delim_height.is_none() && body_contains_middle(body, body_options.style) {
        let total = SIZE_TO_MAX_HEIGHT[1];
        let axis = body_options.metrics().axis_height;
        let height = total / 2.0 + axis;
        let depth = total - height;
        lbox.height = lbox.height.max(height);
        lbox.depth = lbox.depth.max(depth);
    }

    lbox
}

fn parse_html_style(style: &str) -> HtmlStyle {
    let mut parsed = HtmlStyle::default();
    for declaration in style.split(';') {
        let Some((property, value)) = declaration.split_once(':') else {
            continue;
        };
        let property = property.trim().to_ascii_lowercase();
        let value = value.trim();
        match property.as_str() {
            "color" => parsed.color = Color::parse(value),
            "font-size" => parsed.font_size_scale = parse_css_font_size(value),
            "font-weight" => parsed.bold = is_css_bold(value),
            "font-style" => parsed.italic = is_css_italic(value),
            "background" | "background-color" => parsed.background_color = Color::parse(value),
            "text-decoration" | "text-decoration-line" => {
                parsed.underline = value
                    .split_whitespace()
                    .any(|part| part.eq_ignore_ascii_case("underline"));
            }
            _ => {}
        }
    }
    parsed
}

fn parse_css_font_size(value: &str) -> Option<f64> {
    let value = value.trim().to_ascii_lowercase().replace(' ', "");
    let parse_number = |s: &str| s.parse::<f64>().ok().filter(|n| n.is_finite() && *n > 0.0);
    if let Some(px) = value.strip_suffix("px") {
        parse_number(px).map(|n| n / 16.0)
    } else if let Some(em) = value
        .strip_suffix("em")
        .or_else(|| value.strip_suffix("rem"))
    {
        parse_number(em)
    } else if let Some(percent) = value.strip_suffix('%') {
        parse_number(percent).map(|n| n / 100.0)
    } else {
        None
    }
}

fn is_css_bold(value: &str) -> bool {
    let value = value.trim();
    value.eq_ignore_ascii_case("bold")
        || value.eq_ignore_ascii_case("bolder")
        || value.parse::<u16>().is_ok_and(|weight| weight >= 600)
}

fn is_css_italic(value: &str) -> bool {
    let value = value.trim();
    value.eq_ignore_ascii_case("italic") || value.eq_ignore_ascii_case("oblique")
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
                None => (FontId::TypewriterRegular, 0.5, metrics.x_height, 0.0),
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

fn glyph_run_box(children: Vec<LayoutBox>, color: Color, inter_glyph_kern: f64) -> LayoutBox {
    let inter_glyph_kern = if inter_glyph_kern > 0.0 {
        inter_glyph_kern
    } else {
        0.0
    };
    let gap_count = children.len().saturating_sub(1);
    let width =
        children.iter().map(|child| child.width).sum::<f64>() + gap_count as f64 * inter_glyph_kern;
    let height = children
        .iter()
        .map(|child| child.height)
        .fold(0.0_f64, f64::max);
    let depth = children
        .iter()
        .map(|child| child.depth)
        .fold(0.0_f64, f64::max);
    let mut x = 0.0;
    let glyph_count = children.len();
    let glyphs = children
        .into_iter()
        .enumerate()
        .map(|(index, child)| {
            let BoxContent::Glyph { font_id, char_code } = child.content else {
                unreachable!("glyph_run_box only accepts glyph boxes");
            };
            let glyph = RunGlyph {
                x,
                font_id,
                char_code,
            };
            x += child.width;
            if index + 1 < glyph_count {
                x += inter_glyph_kern;
            }
            glyph
        })
        .collect();

    LayoutBox {
        width,
        height,
        depth,
        content: BoxContent::GlyphRun { glyphs },
        color,
    }
}

/// Coalesce adjacent glyphs into internal text runs while retaining structural
/// boxes (accents, inline math, spacing commands, fallback images) as run
/// boundaries.
fn make_text_runs(children: Vec<LayoutBox>, color: Color, inter_glyph_kern: f64) -> LayoutBox {
    let inter_glyph_kern = if inter_glyph_kern > 0.0 {
        inter_glyph_kern
    } else {
        0.0
    };
    let mut output = Vec::new();
    let mut pending_glyphs = Vec::new();
    let mut has_previous_child = false;

    let flush = |pending: &mut Vec<LayoutBox>, output: &mut Vec<LayoutBox>| {
        if !pending.is_empty() {
            output.push(glyph_run_box(
                std::mem::take(pending),
                color,
                inter_glyph_kern,
            ));
        }
    };

    for child in children {
        if matches!(child.content, BoxContent::Glyph { .. }) && child.color == color {
            if pending_glyphs.is_empty() && has_previous_child && inter_glyph_kern > 0.0 {
                output.push(LayoutBox::new_kern(inter_glyph_kern));
            }
            pending_glyphs.push(child);
        } else {
            flush(&mut pending_glyphs, &mut output);
            if has_previous_child && inter_glyph_kern > 0.0 {
                output.push(LayoutBox::new_kern(inter_glyph_kern));
            }
            output.push(child);
        }
        has_previous_child = true;
    }
    flush(&mut pending_glyphs, &mut output);
    make_hbox(output)
}

/// Lay out `\text{…}` / `HBox` contents as positioned glyph runs.
fn layout_text(body: &[ParseNode], options: &LayoutOptions) -> LayoutBox {
    let mut children = Vec::new();
    for node in body {
        match node {
            ParseNode::TextOrd { text, mode, .. } | ParseNode::MathOrd { text, mode, .. } => {
                children.push(layout_symbol(text, *mode, options));
            }
            ParseNode::SpacingNode { text, .. } => {
                children.push(layout_spacing_command(text, options));
            }
            _ => {
                children.push(layout_node(node, options));
            }
        }
    }
    make_text_runs(children, options.color, 0.0)
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
    let children = vec![kern_x, shadow, kern_back, base];
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
        .unwrap_or(options.color);

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
fn layout_cancel(label: &str, body: &ParseNode, options: &LayoutOptions) -> LayoutBox {
    use crate::layout_box::BoxContent;
    let inner = layout_node(body, options);
    let w = inner.width.max(0.01);
    let h = inner.height;
    let d = inner.depth;

    // \sout uses no padding — the line spans exactly the content width/height.
    // KaTeX cancel padding: single character gets vertical extension, multi-char gets horizontal.
    let single = is_single_char_body(body);
    let (v_pad, h_pad) = if label == "\\sout" {
        (0.0, 0.0)
    } else if single {
        (0.2, 0.0)
    } else {
        (0.0, 0.2)
    };

    // Path coordinates: y=0 at baseline, y<0 above (height), y>0 below (depth).
    // \cancel  = "/" diagonal: bottom-left → top-right
    // \bcancel = "\" diagonal: top-left → bottom-right
    let commands: Vec<PathCommand> = match label {
        "\\cancel" => vec![
            PathCommand::MoveTo {
                x: -h_pad,
                y: d + v_pad,
            }, // bottom-left
            PathCommand::LineTo {
                x: w + h_pad,
                y: -h - v_pad,
            }, // top-right
        ],
        "\\bcancel" => vec![
            PathCommand::MoveTo {
                x: -h_pad,
                y: -h - v_pad,
            }, // top-left
            PathCommand::LineTo {
                x: w + h_pad,
                y: d + v_pad,
            }, // bottom-right
        ],
        "\\xcancel" => vec![
            PathCommand::MoveTo {
                x: -h_pad,
                y: d + v_pad,
            },
            PathCommand::LineTo {
                x: w + h_pad,
                y: -h - v_pad,
            },
            PathCommand::MoveTo {
                x: -h_pad,
                y: -h - v_pad,
            },
            PathCommand::LineTo {
                x: w + h_pad,
                y: d + v_pad,
            },
        ],
        "\\sout" => {
            // Horizontal line at –0.5× x-height, extended to content edges.
            let mid_y = -0.5 * options.metrics().x_height;
            vec![
                PathCommand::MoveTo { x: 0.0, y: mid_y },
                PathCommand::LineTo { x: w, y: mid_y },
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
        content: BoxContent::SvgPath {
            commands,
            fill: false,
        },
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
        PathCommand::MoveTo {
            x: right_x,
            y: vy(y_svg),
        },
        PathCommand::LineTo {
            x: 0.0,
            y: vy(y_svg),
        },
        PathCommand::LineTo {
            x: x_peak * sx,
            y: vy(0.0),
        },
        PathCommand::LineTo {
            x: (x_peak + 65.0) * sx,
            y: vy(45.0),
        },
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

    let body_shifted = make_hbox(vec![LayoutBox::new_kern(left_pad), inner.clone()]);

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
                content: BoxContent::SvgPath {
                    commands,
                    fill: true,
                },
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
        PathCommand::LineTo {
            x: w,
            y: inner.depth + 0.3_f64,
        },
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
    if matches!(font, "text" | "\\text") {
        return match body {
            ParseNode::OrdGroup { body, .. } => layout_text(body, options),
            _ => layout_node(body, options),
        };
    }

    let font_id = match font {
        "textnormal" | "\\textnormal" | "textmd" | "\\textmd" | "textup" | "\\textup"
        | "mathrm" | "\\mathrm" | "textrm" | "\\textrm" | "rm" | "\\rm" => {
            Some(FontId::MainRegular)
        }
        "mathbf" | "\\mathbf" | "textbf" | "\\textbf" | "bf" | "\\bf" => Some(FontId::MainBold),
        "mathit" | "\\mathit" | "it" | "\\it" => Some(FontId::MathItalic),
        "textit" | "\\textit" | "\\emph" => Some(FontId::MainItalic),
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
            let children = body
                .iter()
                .map(|node| layout_with_font(node, font_id, options))
                .collect();
            make_text_runs(children, options.color, options.inter_glyph_kern_em)
        }
        ParseNode::SupSub { base, sup, sub, .. } => {
            if let Some(base_node) = base.as_deref() {
                if should_use_op_limits(base_node, options) {
                    return layout_op_with_limits(
                        base_node,
                        sup.as_deref(),
                        sub.as_deref(),
                        options,
                    );
                }
            }
            layout_supsub(
                base.as_deref(),
                sup.as_deref(),
                sub.as_deref(),
                options,
                Some(font_id),
            )
        }
        ParseNode::MathOrd { text, mode, .. }
        | ParseNode::TextOrd { text, mode, .. }
        | ParseNode::Atom { text, mode, .. } => {
            if let Some(fitted) =
                layout_symbol_from_render_spec(text, *mode, options, Some(font_id))
            {
                return fitted;
            }
            let ch = resolve_symbol_char(text, *mode);
            let char_code = ch as u32;
            let metric_cp = ratex_font::font_and_metric_for_mathematical_alphanumeric(char_code)
                .map(|(_, m)| m)
                .unwrap_or(char_code);
            if let Some(m) = get_char_metrics(font_id, metric_cp) {
                LayoutBox {
                    // Text mode: no italic correction (it's a typographic hint for math sub/sup).
                    width: math_glyph_advance_em(&m, *mode),
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

#[derive(Debug, Clone, Copy)]
struct RuleGeometry {
    thickness: f64,
    line_center_offset: f64,
    total_extent: f64,
}

impl RuleGeometry {
    fn overline(options: &LayoutOptions) -> Self {
        let thickness = options.metrics().default_rule_thickness;
        Self {
            thickness,
            // KaTeX vlist: 3r clearance, then the r-thick rule.
            line_center_offset: 3.5 * thickness,
            // One additional rule unit is retained at the outer edge.
            total_extent: 5.0 * thickness,
        }
    }

    fn underline(options: &LayoutOptions) -> Self {
        let thickness = options.metrics().default_rule_thickness;
        Self {
            thickness,
            line_center_offset: 4.0 * thickness,
            total_extent: 4.5 * thickness,
        }
    }
}

fn layout_overline(body: &ParseNode, options: &LayoutOptions) -> LayoutBox {
    let cramped = options.with_style(options.style.cramped());
    let body_box = layout_node(body, &cramped);
    let rule = RuleGeometry::overline(options);

    let height = body_box.height + rule.total_extent;
    LayoutBox {
        width: body_box.width,
        height,
        depth: body_box.depth,
        content: BoxContent::Overline {
            body: Box::new(body_box),
            rule_thickness: rule.thickness,
            offset: rule.line_center_offset,
        },
        color: options.color,
    }
}

fn layout_underline(body: &ParseNode, options: &LayoutOptions) -> LayoutBox {
    let body_box = layout_node(body, options);
    let rule = RuleGeometry::underline(options);

    let depth = body_box.depth + rule.total_extent;
    LayoutBox {
        width: body_box.width,
        height: body_box.height,
        depth,
        content: BoxContent::Underline {
            body: Box::new(body_box),
            rule_thickness: rule.thickness,
            offset: rule.line_center_offset,
        },
        color: options.color,
    }
}

/// `\href` / `\url`: link color on the glyphs and an underline in the same color (KaTeX-style).
fn layout_href(body: &[ParseNode], options: &LayoutOptions) -> LayoutBox {
    let link_color = Color::from_name("blue").unwrap_or_else(|| Color::rgb(0.0, 0.0, 1.0));
    let body_opts = options.with_color(link_color);
    let body_box = layout_expression(body, &body_opts, true);
    layout_link_underline_laid_out(body_box, options, link_color)
}

/// Same geometry as [`layout_underline`], but for an already computed inner box.
fn layout_underline_laid_out(
    body_box: LayoutBox,
    options: &LayoutOptions,
    color: Color,
) -> LayoutBox {
    let metrics = options.metrics();
    let rule = metrics.default_rule_thickness;
    layout_underline_laid_out_with_rule(body_box, rule, 2.5 * rule, color)
}

fn layout_underline_laid_out_with_rule(
    body_box: LayoutBox,
    rule: f64,
    offset: f64,
    color: Color,
) -> LayoutBox {
    let depth = body_box.depth + offset + 0.5 * rule;
    LayoutBox {
        width: body_box.width,
        height: body_box.height,
        depth,
        content: BoxContent::Underline {
            body: Box::new(body_box),
            rule_thickness: rule,
            offset,
        },
        color,
    }
}

fn link_underline_skip_cuts(lbox: &LayoutBox, x: f64, scale: f64, cuts: &mut Vec<(f64, f64)>) {
    let mut pending = vec![(lbox, x, scale)];
    while let Some((current, current_x, current_scale)) = pending.pop() {
        match &current.content {
            BoxContent::HBox(children) => {
                let mut child_positions = Vec::with_capacity(children.len());
                let mut child_x = current_x;
                for child in children {
                    child_positions.push((child, child_x, current_scale));
                    child_x += child.width * current_scale;
                }
                pending.extend(child_positions.into_iter().rev());
            }
            BoxContent::Glyph { char_code, .. } => {
                if let Some(ch) = char::from_u32(*char_code) {
                    if matches!(ch, 'g' | 'j' | 'p' | 'q' | 'y') {
                        let pad = (0.04 * current_scale).min(current.width * current_scale / 3.0);
                        cuts.push((
                            current_x + pad,
                            current_x + current.width * current_scale - pad,
                        ));
                    }
                }
            }
            BoxContent::GlyphRun { glyphs } => {
                for (index, glyph) in glyphs.iter().enumerate() {
                    let Some(ch) = char::from_u32(glyph.char_code) else {
                        continue;
                    };
                    if !matches!(ch, 'g' | 'j' | 'p' | 'q' | 'y') {
                        continue;
                    }
                    let glyph_right = glyphs
                        .get(index + 1)
                        .map(|next| next.x)
                        .unwrap_or(current.width);
                    let glyph_width = (glyph_right - glyph.x).max(0.0);
                    let pad = (0.04 * current_scale).min(glyph_width * current_scale / 3.0);
                    cuts.push((
                        current_x + glyph.x * current_scale + pad,
                        current_x + glyph_right * current_scale - pad,
                    ));
                }
            }
            BoxContent::Scaled { body, child_scale } => {
                pending.push((body, current_x, current_scale * child_scale));
            }
            BoxContent::RaiseBox { body, .. } => {
                pending.push((body, current_x, current_scale));
            }
            _ => {}
        }
    }
}

fn link_underline_segments(body_box: &LayoutBox) -> Vec<(f64, f64)> {
    let mut cuts = Vec::new();
    link_underline_skip_cuts(body_box, 0.0, 1.0, &mut cuts);
    if cuts.is_empty() {
        return vec![(0.0, body_box.width)];
    }
    cuts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    let mut segments = Vec::new();
    let mut x = 0.0;
    for (cut_l, cut_r) in cuts {
        let l = cut_l.clamp(0.0, body_box.width);
        let r = cut_r.clamp(0.0, body_box.width);
        if l > x + 0.02 {
            segments.push((x, l - x));
        }
        x = x.max(r);
    }
    if body_box.width > x + 0.02 {
        segments.push((x, body_box.width - x));
    }
    if segments.is_empty() {
        vec![(0.0, body_box.width)]
    } else {
        segments
    }
}

fn layout_link_underline_laid_out(
    body_box: LayoutBox,
    options: &LayoutOptions,
    color: Color,
) -> LayoutBox {
    let metrics = options.metrics();
    let rule = metrics.default_rule_thickness;
    // Browser text decoration uses the font underline position measured from
    // the baseline. Keep it out of a descender's ink box when the run is
    // deeper than that nominal position.
    let center_from_baseline = body_box.depth.max(3.125 * rule);
    let offset = center_from_baseline - body_box.depth;
    let segments = link_underline_segments(&body_box);
    if segments.len() <= 1 {
        return layout_underline_laid_out_with_rule(body_box, rule, offset, color);
    }

    let height = body_box.height;
    let depth = body_box.depth.max(center_from_baseline + 0.5 * rule);
    let rule_y = height + body_box.depth + offset;
    let width = body_box.width;
    LayoutBox {
        width,
        height,
        depth,
        content: BoxContent::ProofTree {
            children: vec![PlacedBox {
                box_: body_box,
                x: 0.0,
                baseline_y: height,
            }],
            rules: segments
                .into_iter()
                .map(|(x, width)| ProofRule {
                    x,
                    y: rule_y,
                    width,
                    thickness: rule,
                    dashed: false,
                })
                .collect(),
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
    // SupSub and HTML wrappers can form input-controlled chains.  Walk them
    // explicitly so spacing classification does not consume one call frame per node.
    let mut pending = vec![node];
    while let Some(current) = pending.pop() {
        match current {
            ParseNode::MathOrd { .. } | ParseNode::TextOrd { .. } => {
                return Some(MathClass::Ord);
            }
            ParseNode::Atom { family, .. } => return Some(family_to_math_class(*family)),
            ParseNode::OpToken { .. } | ParseNode::Op { .. } | ParseNode::OperatorName { .. } => {
                return Some(MathClass::Op);
            }
            ParseNode::OrdGroup { .. } => return Some(MathClass::Ord),
            // KaTeX genfrac.js: with delimiters (e.g. \binom) → mord; without
            // (e.g. \frac) → minner.
            ParseNode::GenFrac {
                left_delim,
                right_delim,
                ..
            } => {
                let has_delim = left_delim
                    .as_ref()
                    .is_some_and(|d| !d.is_empty() && d != ".")
                    || right_delim
                        .as_ref()
                        .is_some_and(|d| !d.is_empty() && d != ".");
                return Some(if has_delim {
                    MathClass::Ord
                } else {
                    MathClass::Inner
                });
            }
            ParseNode::Sqrt { .. } => return Some(MathClass::Ord),
            ParseNode::SupSub { base, .. } => {
                if let Some(base) = base {
                    pending.push(base);
                }
            }
            ParseNode::MClass { mclass, .. } => {
                return Some(mclass_str_to_math_class(mclass));
            }
            ParseNode::SpacingNode { .. } | ParseNode::Kern { .. } | ParseNode::Lap { .. } => {}
            ParseNode::HtmlMathMl { html, .. } => {
                pending.extend(html.iter().rev());
            }
            ParseNode::Html { body, .. } => {
                pending.extend(body.iter().rev());
            }
            ParseNode::LeftRight { .. } => return Some(MathClass::Inner),
            ParseNode::AccentToken { .. } => return Some(MathClass::Ord),
            // \xrightarrow and CD arrows are relations for TeX spacing.
            ParseNode::XArrow { .. } | ParseNode::CdArrow { .. } => {
                return Some(MathClass::Rel);
            }
            ParseNode::DelimSizing { mclass, .. } => {
                return Some(mclass_str_to_math_class(mclass));
            }
            ParseNode::Middle { .. } => return Some(MathClass::Ord),
            _ => return Some(MathClass::Ord),
        }
    }
    None
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
/// KaTeX `getBaseElem` (`utils.js`): unwrap `ordgroup` / `color` with a single child, and `font`.
/// Used for TeX "character box" checks in superscript Rule 18a (`supsub.js`).
fn get_base_elem(node: &ParseNode) -> &ParseNode {
    let mut current = node;
    loop {
        current = match current {
            ParseNode::OrdGroup { body, .. } if body.len() == 1 => &body[0],
            ParseNode::Color { body, .. } if body.len() == 1 => &body[0],
            ParseNode::Html { body, .. } if body.len() == 1 => &body[0],
            ParseNode::Font { body, .. } => body,
            _ => return current,
        };
    }
}

fn is_character_box(node: &ParseNode) -> bool {
    matches!(
        get_base_elem(node),
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
    func_label: &str,
    options: &LayoutOptions,
) -> LayoutBox {
    let body_box = layout_node(base, options);

    let is_bracket = func_label.trim_start_matches('\\').ends_with("bracket");

    // `\overbrace`/`\underbrace` and mathtools `\overbracket`/`\underbracket`: KaTeX stretchy SVG (filled paths).
    let stretch_key = if is_bracket {
        if is_over {
            "overbracket"
        } else {
            "underbracket"
        }
    } else if is_over {
        "overbrace"
    } else {
        "underbrace"
    };
    let w = body_box
        .width
        .max(crate::katex_svg::katex_stretchy_min_width_em(stretch_key).unwrap_or(0.5));

    let (raw_commands, brace_h, brace_fill) =
        match crate::katex_svg::katex_stretchy_path(stretch_key, w) {
            Some((c, h)) => (c, h, true),
            None => {
                let h = 0.35_f64;
                (horiz_brace_path(w, h, is_over), h, false)
            }
        };

    // Shift y-coordinates: centered commands → SVG-downward convention (height=0, depth=brace_h).
    // The raw path is centered at y=0 (range ±brace_h/2). Shift by +brace_h/2 so that:
    //   overbrace: peak at y=0 (top), feet at y=+brace_h (bottom)
    //   underbrace: feet at y=0 (top), peak at y=+brace_h (bottom)
    // Both use height=0, depth=brace_h so the rendering code's SVG accent path handles them.
    let y_shift = brace_h / 2.0;
    let commands = shift_path_y(raw_commands, y_shift);

    let brace_box = LayoutBox {
        width: w,
        height: 0.0,
        depth: brace_h,
        content: BoxContent::SvgPath {
            commands,
            fill: brace_fill,
        },
        color: options.color,
    };

    compose_stretchy(
        padded_to_width(body_box, w),
        brace_box,
        StretchySpec::horizontal_brace(is_over, options),
        0.0,
        options.color,
    )
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
    let axis = metrics.axis_height; // 0.25em
    let arrow_half = actual_arrow_h / 2.0;
    let gap = 0.111; // 2mu gap (KaTeX constant)

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
            x1: cx + r,
            y1: cy - k * r,
            x2: cx + k * r,
            y2: cy - r,
            x: cx,
            y: cy - r,
        },
        PathCommand::CubicTo {
            x1: cx - k * r,
            y1: cy - r,
            x2: cx - r,
            y2: cy - k * r,
            x: cx - r,
            y: cy,
        },
        PathCommand::CubicTo {
            x1: cx - r,
            y1: cy + k * r,
            x2: cx - k * r,
            y2: cy + r,
            x: cx,
            y: cy + r,
        },
        PathCommand::CubicTo {
            x1: cx + k * r,
            y1: cy + r,
            x2: cx + r,
            y2: cy + k * r,
            x: cx + r,
            y: cy,
        },
        PathCommand::Close,
    ];

    let circle_box = LayoutBox {
        width: diameter,
        height: r - cy.min(0.0),
        depth: (r + cy).max(0.0),
        content: BoxContent::SvgPath {
            commands: circle_commands,
            fill: false,
        },
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
                x1: ox + rad,
                y1: cy - k * rad,
                x2: ox + k * rad,
                y2: cy - rad,
                x: ox,
                y: cy - rad,
            },
            PathCommand::CubicTo {
                x1: ox - k * rad,
                y1: cy - rad,
                x2: ox - rad,
                y2: cy - k * rad,
                x: ox - rad,
                y: cy,
            },
            PathCommand::CubicTo {
                x1: ox - rad,
                y1: cy + k * rad,
                x2: ox - k * rad,
                y2: cy + rad,
                x: ox,
                y: cy + rad,
            },
            PathCommand::CubicTo {
                x1: ox + k * rad,
                y1: cy + rad,
                x2: ox + rad,
                y2: cy + k * rad,
                x: ox + rad,
                y: cy,
            },
            PathCommand::Close,
        ]
    };

    let disk = LayoutBox {
        width: 2.0 * r,
        height: h,
        depth: d,
        content: BoxContent::SvgPath {
            commands: circle_commands(cx, r),
            fill: true,
        },
        color: options.color,
    };

    let ring = LayoutBox {
        width: 2.0 * r,
        height: h,
        depth: d,
        content: BoxContent::SvgPath {
            commands: circle_commands(cx, r_ring),
            fill: false,
        },
        color: options.color,
    };

    // Connecting bar centred on the same axis as the circles.
    // Rule.raise = distance from baseline to the bottom edge of the rule.
    // bar centre at |cy| = 0.2625em  →  raise = 0.2625 − bar_th/2
    let bar_len: f64 = 0.25;
    let bar_th: f64 = 0.04;
    let bar_raise: f64 = cy.abs() - bar_th / 2.0; // 0.2625 − 0.02 = 0.2425

    let bar = LayoutBox::new_rule(bar_len, h, d, bar_th, bar_raise).with_color(options.color);

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
    let b = 0.3_f64; // vertical semi-axis (0.1 * 2)
    let k = 0.62_f64; // Bezier factor: larger = fuller ellipse (0.5523 ≈ exact circle)
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
    cmds.into_iter()
        .map(|c| match c {
            PathCommand::MoveTo { x, y } => PathCommand::MoveTo { x, y: y + dy },
            PathCommand::LineTo { x, y } => PathCommand::LineTo { x, y: y + dy },
            PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => PathCommand::CubicTo {
                x1,
                y1: y1 + dy,
                x2,
                y2: y2 + dy,
                x,
                y: y + dy,
            },
            PathCommand::QuadTo { x1, y1, x, y } => PathCommand::QuadTo {
                x1,
                y1: y1 + dy,
                x,
                y: y + dy,
            },
            PathCommand::Close => PathCommand::Close,
        })
        .collect()
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
                PathCommand::MoveTo {
                    x: ah,
                    y: mid_y - ah,
                },
                PathCommand::LineTo { x: 0.0, y: mid_y },
                PathCommand::LineTo {
                    x: ah,
                    y: mid_y + ah,
                },
                PathCommand::MoveTo { x: 0.0, y: mid_y },
                PathCommand::LineTo { x: width, y: mid_y },
            ]
        }
        "\\overleftrightarrow"
        | "\\underleftrightarrow"
        | "\\xleftrightarrow"
        | "\\xLeftrightarrow" => {
            vec![
                PathCommand::MoveTo {
                    x: ah,
                    y: mid_y - ah,
                },
                PathCommand::LineTo { x: 0.0, y: mid_y },
                PathCommand::LineTo {
                    x: ah,
                    y: mid_y + ah,
                },
                PathCommand::MoveTo { x: 0.0, y: mid_y },
                PathCommand::LineTo { x: width, y: mid_y },
                PathCommand::MoveTo {
                    x: width - ah,
                    y: mid_y - ah,
                },
                PathCommand::LineTo { x: width, y: mid_y },
                PathCommand::LineTo {
                    x: width - ah,
                    y: mid_y + ah,
                },
            ]
        }
        "\\xlongequal" => {
            let gap = 0.04;
            vec![
                PathCommand::MoveTo {
                    x: 0.0,
                    y: mid_y - gap,
                },
                PathCommand::LineTo {
                    x: width,
                    y: mid_y - gap,
                },
                PathCommand::MoveTo {
                    x: 0.0,
                    y: mid_y + gap,
                },
                PathCommand::LineTo {
                    x: width,
                    y: mid_y + gap,
                },
            ]
        }
        "\\xhookleftarrow" => {
            vec![
                PathCommand::MoveTo {
                    x: ah,
                    y: mid_y - ah,
                },
                PathCommand::LineTo { x: 0.0, y: mid_y },
                PathCommand::LineTo {
                    x: ah,
                    y: mid_y + ah,
                },
                PathCommand::MoveTo { x: 0.0, y: mid_y },
                PathCommand::LineTo { x: width, y: mid_y },
                PathCommand::QuadTo {
                    x1: width + ah,
                    y1: mid_y,
                    x: width + ah,
                    y: mid_y + ah,
                },
            ]
        }
        "\\xhookrightarrow" => {
            vec![
                PathCommand::MoveTo {
                    x: 0.0 - ah,
                    y: mid_y - ah,
                },
                PathCommand::QuadTo {
                    x1: 0.0 - ah,
                    y1: mid_y,
                    x: 0.0,
                    y: mid_y,
                },
                PathCommand::LineTo { x: width, y: mid_y },
                PathCommand::MoveTo {
                    x: width - ah,
                    y: mid_y - ah,
                },
                PathCommand::LineTo { x: width, y: mid_y },
                PathCommand::LineTo {
                    x: width - ah,
                    y: mid_y + ah,
                },
            ]
        }
        "\\xrightharpoonup" | "\\xleftharpoonup" => {
            let right = label.contains("right");
            if right {
                vec![
                    PathCommand::MoveTo { x: 0.0, y: mid_y },
                    PathCommand::LineTo { x: width, y: mid_y },
                    PathCommand::MoveTo {
                        x: width - ah,
                        y: mid_y - ah,
                    },
                    PathCommand::LineTo { x: width, y: mid_y },
                ]
            } else {
                vec![
                    PathCommand::MoveTo {
                        x: ah,
                        y: mid_y - ah,
                    },
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
                    PathCommand::MoveTo {
                        x: width - ah,
                        y: mid_y + ah,
                    },
                    PathCommand::LineTo { x: width, y: mid_y },
                ]
            } else {
                vec![
                    PathCommand::MoveTo {
                        x: ah,
                        y: mid_y + ah,
                    },
                    PathCommand::LineTo { x: 0.0, y: mid_y },
                    PathCommand::LineTo { x: width, y: mid_y },
                ]
            }
        }
        "\\xrightleftharpoons" | "\\xleftrightharpoons" => {
            let gap = 0.06;
            vec![
                PathCommand::MoveTo {
                    x: 0.0,
                    y: mid_y - gap,
                },
                PathCommand::LineTo {
                    x: width,
                    y: mid_y - gap,
                },
                PathCommand::MoveTo {
                    x: width - ah,
                    y: mid_y - gap - ah,
                },
                PathCommand::LineTo {
                    x: width,
                    y: mid_y - gap,
                },
                PathCommand::MoveTo {
                    x: width,
                    y: mid_y + gap,
                },
                PathCommand::LineTo {
                    x: 0.0,
                    y: mid_y + gap,
                },
                PathCommand::MoveTo {
                    x: ah,
                    y: mid_y + gap + ah,
                },
                PathCommand::LineTo {
                    x: 0.0,
                    y: mid_y + gap,
                },
            ]
        }
        "\\xtofrom" | "\\xrightleftarrows" => {
            let gap = 0.06;
            vec![
                PathCommand::MoveTo {
                    x: 0.0,
                    y: mid_y - gap,
                },
                PathCommand::LineTo {
                    x: width,
                    y: mid_y - gap,
                },
                PathCommand::MoveTo {
                    x: width - ah,
                    y: mid_y - gap - ah,
                },
                PathCommand::LineTo {
                    x: width,
                    y: mid_y - gap,
                },
                PathCommand::LineTo {
                    x: width - ah,
                    y: mid_y - gap + ah,
                },
                PathCommand::MoveTo {
                    x: width,
                    y: mid_y + gap,
                },
                PathCommand::LineTo {
                    x: 0.0,
                    y: mid_y + gap,
                },
                PathCommand::MoveTo {
                    x: ah,
                    y: mid_y + gap - ah,
                },
                PathCommand::LineTo {
                    x: 0.0,
                    y: mid_y + gap,
                },
                PathCommand::LineTo {
                    x: ah,
                    y: mid_y + gap + ah,
                },
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
                PathCommand::MoveTo {
                    x: width - ah,
                    y: mid_y - ah,
                },
                PathCommand::LineTo { x: width, y: mid_y },
                PathCommand::LineTo {
                    x: width - ah,
                    y: mid_y + ah,
                },
            ]
        }
    }
}

// ============================================================================
// CD (amscd commutative diagram) layout
// ============================================================================

/// Wrap a horizontal arrow cell with left/right kerns (KaTeX `.cd-arrow-pad`).
fn cd_wrap_hpad(inner: LayoutBox, pad_l: f64, pad_r: f64, color: Color) -> LayoutBox {
    let h = inner.height;
    let d = inner.depth;
    let w = inner.width + pad_l + pad_r;
    let mut children: Vec<LayoutBox> = Vec::with_capacity(3);
    if pad_l > 0.0 {
        children.push(LayoutBox::new_kern(pad_l));
    }
    children.push(inner);
    if pad_r > 0.0 {
        children.push(LayoutBox::new_kern(pad_r));
    }
    LayoutBox {
        width: w,
        height: h,
        depth: d,
        content: BoxContent::HBox(children),
        color,
    }
}

fn cd_shift_label_ink_left(label: LayoutBox, color: Color) -> LayoutBox {
    const SHIFT: f64 = -0.1;
    let width = label.width;
    let height = label.height;
    let depth = label.depth;
    LayoutBox {
        width,
        height,
        depth,
        content: BoxContent::HBox(vec![
            LayoutBox::new_kern(SHIFT),
            label,
            LayoutBox::new_kern(-SHIFT),
        ]),
        color,
    }
}

fn thin_cd_arrow_shaft(commands: Vec<PathCommand>) -> Vec<PathCommand> {
    let thin_y = |y: f64| {
        if y.abs() <= 0.021 {
            y * 0.5
        } else {
            y
        }
    };
    commands
        .into_iter()
        .map(|command| match command {
            PathCommand::MoveTo { x, y } => PathCommand::MoveTo { x, y: thin_y(y) },
            PathCommand::LineTo { x, y } => PathCommand::LineTo { x, y: thin_y(y) },
            PathCommand::CubicTo {
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => PathCommand::CubicTo {
                x1,
                y1: thin_y(y1),
                x2,
                y2: thin_y(y2),
                x,
                y: thin_y(y),
            },
            PathCommand::QuadTo { x1, y1, x, y } => PathCommand::QuadTo {
                x1,
                y1: thin_y(y1),
                x,
                y: thin_y(y),
            },
            PathCommand::Close => PathCommand::Close,
        })
        .collect()
}

fn cd_label_has_ink(label: Option<&ParseNode>) -> bool {
    match label {
        None => false,
        Some(ParseNode::OrdGroup { body, .. }) => !body.is_empty(),
        Some(_) => true,
    }
}

/// Wrap a side label for a vertical CD arrow so it is vertically centered on the shaft.
///
/// The resulting box reports `height = box_h, depth = box_d` (same as the shaft) so it
/// does not change the row's allocated height.  The label body is raised/lowered via
/// `RaiseBox` so that the label's visual center aligns with the shaft's vertical center.
///
/// Derivation (screen coords, y+ downward):
///   shaft center  = (box_d − box_h) / 2
///   label center  = −shift − (label_h − label_d) / 2
///   solving gives  shift = (box_h − box_d + label_d − label_h) / 2
fn cd_vcenter_side_label(label: LayoutBox, box_h: f64, box_d: f64, color: Color) -> LayoutBox {
    let shift = (box_h - box_d + label.depth - label.height) / 2.0;
    LayoutBox {
        width: label.width,
        height: box_h,
        depth: box_d,
        content: BoxContent::RaiseBox {
            body: Box::new(label),
            shift,
        },
        color,
    }
}

/// Side labels on vertical `{CD}` arrows: KaTeX `\\\\cdleft` / `\\\\cdright` both use
/// `options.style.sup()` (`cd.js` htmlBuilder), then our pipeline must scale like `OpLimits`
/// scripts — `RaiseBox` in `to_display` does not apply script size, so wrap in `Scaled`.
fn cd_side_label_scaled(body: &ParseNode, options: &LayoutOptions) -> LayoutBox {
    let sup_style = options.style.superscript();
    let sup_opts = options.with_style(sup_style);
    let sup_ratio = sup_style.size_multiplier() / options.style.size_multiplier();
    let inner = layout_node(body, &sup_opts);
    if (sup_ratio - 1.0).abs() < 1e-6 {
        inner
    } else {
        LayoutBox {
            width: inner.width * sup_ratio,
            height: inner.height * sup_ratio,
            depth: inner.depth * sup_ratio,
            content: BoxContent::Scaled {
                body: Box::new(inner),
                child_scale: sup_ratio,
            },
            color: options.color,
        }
    }
}

/// KaTeX `makeStackedDelim` geometry for the `\Big\uparrow` / `\Big\downarrow`
/// used by `{CD}`. The arrow end and U+23D0 repeat come from Size1-Regular;
/// the middle shaft is a filled 0.043em strip with 0.008em overlaps.
fn cd_stacked_vert_arrow_box(
    requested_height: f64,
    down: bool,
    options: &LayoutOptions,
) -> LayoutBox {
    const ARROW_UP: u32 = 0x2191;
    const ARROW_DOWN: u32 = 0x2193;
    const REPEAT: u32 = 0x23d0;
    const LAP: f64 = 0.008;

    let glyph_box = |char_code: u32| {
        let metrics = get_char_metrics(FontId::Size1Regular, char_code)
            .expect("KaTeX Size1 stacked-arrow glyph");
        LayoutBox {
            width: metrics.width,
            height: metrics.height,
            depth: metrics.depth,
            content: BoxContent::Glyph {
                font_id: FontId::Size1Regular,
                char_code,
            },
            color: options.color,
        }
    };

    let arrow = glyph_box(if down { ARROW_DOWN } else { ARROW_UP });
    let repeat = glyph_box(REPEAT);
    let top = if down { repeat.clone() } else { arrow.clone() };
    let bottom = if down { arrow } else { repeat.clone() };
    let top_total = top.height + top.depth;
    let bottom_total = bottom.height + bottom.depth;
    let repeat_total = repeat.height + repeat.depth;
    let minimum = top_total + bottom_total;
    let repeat_count = ((requested_height.max(minimum) - minimum) / repeat_total)
        .ceil()
        .max(0.0) as usize;
    let real_height = minimum + repeat_count as f64 * repeat_total;
    let inner_height = real_height - minimum + 2.0 * LAP;
    let width = top.width.max(bottom.width).max(repeat.width);

    let shaft = LayoutBox {
        width: repeat.width,
        height: inner_height,
        depth: 0.0,
        content: BoxContent::SvgPath {
            commands: vec![
                PathCommand::MoveTo {
                    x: 0.312,
                    y: -inner_height,
                },
                PathCommand::LineTo {
                    x: 0.355,
                    y: -inner_height,
                },
                PathCommand::LineTo { x: 0.355, y: 0.0 },
                PathCommand::LineTo { x: 0.312, y: 0.0 },
                PathCommand::Close,
            ],
            fill: true,
        },
        color: options.color,
    };
    // Size1's stacked-arrow pieces have a right side-bearing that does not
    // participate in the DOM centering box. Compensate so the visible shaft,
    // arrowhead, and object-column center share one x coordinate.
    const VISIBLE_CENTER_ADJUST: f64 = -0.0875;
    let centered = |box_: LayoutBox| VBoxChild {
        shift: (width - box_.width) / 2.0 + VISIBLE_CENTER_ADJUST,
        kind: VBoxChildKind::Box(Box::new(box_)),
    };
    let children = vec![
        centered(top),
        VBoxChild {
            kind: VBoxChildKind::Kern(-LAP),
            shift: 0.0,
        },
        centered(shaft),
        VBoxChild {
            kind: VBoxChildKind::Kern(-LAP),
            shift: 0.0,
        },
        centered(bottom),
    ];
    let depth = (real_height / 2.0 - options.metrics().axis_height).max(0.0);

    LayoutBox {
        width,
        height: real_height - depth,
        depth,
        content: BoxContent::VBox(children),
        color: options.color,
    }
}

fn cd_rotated_vert_arrow_box(total_height: f64, down: bool, options: &LayoutOptions) -> LayoutBox {
    let axis = options.metrics().axis_height;
    let depth = (total_height / 2.0 - axis).max(0.0);
    let height = total_height - depth;
    if let Some((commands, width)) =
        crate::katex_svg::katex_cd_vert_arrow_from_rightarrow(down, total_height, axis)
    {
        LayoutBox {
            width,
            height,
            depth,
            content: BoxContent::SvgPath {
                commands,
                fill: true,
            },
            color: options.color,
        }
    } else {
        make_stretchy_delim(
            if down { "\\downarrow" } else { "\\uparrow" },
            total_height,
            options,
        )
    }
}

/// Render a single CdArrow cell.
///
/// `target_size`:
/// - `w > 0` for horizontal arrows: shaft length is exactly `w` em (KaTeX: per-cell natural width,
///   not the full column max — see `.katex .mtable` + `.stretchy { width: 100% }` where the cell
///   span is only as wide as content; narrow arrows stay at `max(labels, minCDarrowwidth)` and sit
///   centered in a wider column).
/// - `h > 0` for vertical arrows: shaft total height (height+depth) = `h`.
/// - `0.0` = natural size (pass 1).
///
/// `target_col_width`: when `> 0`, center the cell in this column width (horizontal: side kerns;
/// vertical: kerns around shaft + labels).
///
fn layout_cd_arrow(
    direction: &str,
    label_above: Option<&ParseNode>,
    label_below: Option<&ParseNode>,
    target_size: f64,
    target_col_width: f64,
    compact_labels_enabled: bool,
    options: &LayoutOptions,
) -> LayoutBox {
    let metrics = options.metrics();
    let axis = metrics.axis_height;

    match direction {
        "right" | "left" | "horiz_eq" => {
            // ── Horizontal arrow: reuse katex_stretchy_path for proper KaTeX shape ──
            let sup_style = options.style.superscript();
            let sub_style = options.style.subscript();
            let sup_opts = options.with_style(sup_style);
            let sub_opts = options.with_style(sub_style);
            let sup_ratio = sup_style.size_multiplier() / options.style.size_multiplier();
            let sub_ratio = sub_style.size_multiplier() / options.style.size_multiplier();

            let above_has_ink = cd_label_has_ink(label_above);
            let below_has_ink = cd_label_has_ink(label_below);
            let use_compact_cd_labels =
                compact_labels_enabled && direction != "horiz_eq" && !below_has_ink;
            let above_box = label_above.map(|n| {
                let box_ = layout_node(n, &sup_opts);
                if use_compact_cd_labels && above_has_ink {
                    cd_shift_label_ink_left(box_, options.color)
                } else {
                    box_
                }
            });
            let below_box = label_below.map(|n| layout_node(n, &sub_opts));

            let above_w = above_box
                .as_ref()
                .map(|b| b.width * sup_ratio)
                .unwrap_or(0.0);
            let below_w = below_box
                .as_ref()
                .map(|b| b.width * sub_ratio)
                .unwrap_or(0.0);

            // KaTeX `stretchy.js`: CD uses `\\cdrightarrow` / `\\cdleftarrow` / `\\cdlongequal` (minWidth 3.0em).
            let path_label = if direction == "right" {
                "\\cdrightarrow"
            } else if direction == "left" {
                "\\cdleftarrow"
            } else {
                "\\cdlongequal"
            };
            let min_shaft_w =
                crate::katex_svg::katex_stretchy_min_width_em(path_label).unwrap_or(1.0);
            // Based on KaTeX `.cd-arrow-pad` (0.27778 / 0.55556 script-em); slightly trimmed so
            // `natural_w` matches golden KaTeX PNGs in our box model (e.g. 0150).
            const CD_LABEL_PAD_L: f64 = 0.22;
            const CD_LABEL_PAD_R: f64 = 0.48;
            let cd_pad_sup = (CD_LABEL_PAD_L + CD_LABEL_PAD_R) * sup_ratio;
            let cd_pad_sub = (CD_LABEL_PAD_L + CD_LABEL_PAD_R) * sub_ratio;
            let upper_need = above_box
                .as_ref()
                .map(|_| above_w + cd_pad_sup)
                .unwrap_or(0.0);
            let lower_need = below_box
                .as_ref()
                .map(|_| below_w + cd_pad_sub)
                .unwrap_or(0.0);
            let natural_w = upper_need.max(lower_need).max(0.0);
            let shaft_w = if target_size > 0.0 {
                target_size
            } else {
                natural_w.max(min_shaft_w)
            };

            let (commands, actual_arrow_h, fill_arrow) =
                match crate::katex_svg::katex_stretchy_path(path_label, shaft_w) {
                    Some((c, h)) => (c, h, true),
                    None => {
                        // Fallback hand-drawn (should not happen for these labels)
                        let arrow_h = 0.3_f64;
                        let ah = 0.12_f64;
                        let cmds = if direction == "horiz_eq" {
                            let gap = 0.06;
                            vec![
                                PathCommand::MoveTo { x: 0.0, y: -gap },
                                PathCommand::LineTo {
                                    x: shaft_w,
                                    y: -gap,
                                },
                                PathCommand::MoveTo { x: 0.0, y: gap },
                                PathCommand::LineTo { x: shaft_w, y: gap },
                            ]
                        } else if direction == "right" {
                            vec![
                                PathCommand::MoveTo { x: 0.0, y: 0.0 },
                                PathCommand::LineTo { x: shaft_w, y: 0.0 },
                                PathCommand::MoveTo {
                                    x: shaft_w - ah,
                                    y: -ah,
                                },
                                PathCommand::LineTo { x: shaft_w, y: 0.0 },
                                PathCommand::LineTo {
                                    x: shaft_w - ah,
                                    y: ah,
                                },
                            ]
                        } else {
                            vec![
                                PathCommand::MoveTo { x: shaft_w, y: 0.0 },
                                PathCommand::LineTo { x: 0.0, y: 0.0 },
                                PathCommand::MoveTo { x: ah, y: -ah },
                                PathCommand::LineTo { x: 0.0, y: 0.0 },
                                PathCommand::LineTo { x: ah, y: ah },
                            ]
                        };
                        (cmds, arrow_h, false)
                    }
                };
            let commands = if matches!(direction, "right" | "left") && use_compact_cd_labels {
                thin_cd_arrow_shaft(commands)
            } else {
                commands
            };

            // Arrow box centered at y=0 (same as layout_xarrow)
            let arrow_half = actual_arrow_h / 2.0;
            let arrow_box = LayoutBox {
                width: shaft_w,
                height: arrow_half,
                depth: arrow_half,
                content: BoxContent::SvgPath {
                    commands,
                    fill: fill_arrow,
                },
                color: options.color,
            };

            // Total height/depth for OpLimits (mirrors layout_xarrow / KaTeX arrow.ts)
            // The CD label span already carries script-style line height; using
            // the generic x-arrow 2mu kern leaves a visible gap that KaTeX's
            // `.cd-arrow-pad` box does not have. Keep only one rule-thickness
            // of clearance between the label ink and shaft.
            let (sup_gap, sub_gap) = if use_compact_cd_labels {
                (-options.metrics().default_rule_thickness, 0.025)
            } else {
                (0.111, 0.111)
            };
            let sup_h = above_box
                .as_ref()
                .map(|b| b.height * sup_ratio)
                .unwrap_or(0.0);
            let sup_d = above_box
                .as_ref()
                .map(|b| b.depth * sup_ratio)
                .unwrap_or(0.0);
            // KaTeX arrow.ts: label depth only shifts the label up when depth > 0.25
            // (at the label's own scale). Otherwise the label baseline stays fixed and
            // depth extends into the gap without increasing the cell height.
            let sup_d_contrib = if above_box.as_ref().map(|b| b.depth).unwrap_or(0.0) > 0.25 {
                sup_d
            } else {
                0.0
            };
            let height = axis + arrow_half + sup_gap + sup_h + sup_d_contrib;
            let sub_h_raw = below_box
                .as_ref()
                .map(|b| b.height * sub_ratio)
                .unwrap_or(0.0);
            let sub_d_raw = below_box
                .as_ref()
                .map(|b| b.depth * sub_ratio)
                .unwrap_or(0.0);
            let depth = if below_box.is_some() {
                (arrow_half - axis).max(0.0) + sub_gap + sub_h_raw + sub_d_raw
            } else {
                (arrow_half - axis).max(0.0)
            };

            let inner = LayoutBox {
                width: shaft_w,
                height,
                depth,
                content: BoxContent::OpLimits {
                    base: Box::new(arrow_box),
                    sup: above_box.map(Box::new),
                    sub: below_box.map(Box::new),
                    base_shift: -axis,
                    sup_kern: sup_gap,
                    sub_kern: sub_gap,
                    slant: 0.0,
                    sup_scale: sup_ratio,
                    sub_scale: sub_ratio,
                },
                color: options.color,
            };

            // KaTeX HTML: column width is max(cell widths); each cell stays intrinsic width and is
            // centered in the column (`col-align-c`). Match with side kerns, not by stretching the
            // shaft to the column max.
            if target_col_width > inner.width + 1e-6 {
                let extra = target_col_width - inner.width;
                let kl = extra / 2.0;
                let kr = extra - kl;
                cd_wrap_hpad(inner, kl, kr, options.color)
            } else {
                inner
            }
        }

        "down" | "up" | "vert_eq" => {
            // Pass 1: \Big (~1.8em). Pass 2: stretch ↑/↓ / ‖ to the full arrow-row span (em).
            let big_total = SIZE_TO_MAX_HEIGHT[2]; // 1.8em

            let has_side_label = cd_label_has_ink(label_above) || cd_label_has_ink(label_below);
            let shaft_box = match direction {
                "vert_eq" if target_size > 0.0 => make_vert_delim_box(
                    target_size.max(big_total),
                    true,
                    VertDelimKind::Stretchy,
                    options,
                ),
                "vert_eq" => make_stretchy_delim("\\Vert", big_total, options),
                "down" if has_side_label => {
                    cd_rotated_vert_arrow_box(target_size.max(big_total), true, options)
                }
                "up" if has_side_label => {
                    cd_rotated_vert_arrow_box(target_size.max(big_total), false, options)
                }
                "down" if target_size > 0.0 => {
                    cd_stacked_vert_arrow_box(target_size.max(1.0), true, options)
                }
                "up" if target_size > 0.0 => {
                    cd_stacked_vert_arrow_box(target_size.max(1.0), false, options)
                }
                "down" => cd_stacked_vert_arrow_box(big_total, true, options),
                "up" => cd_stacked_vert_arrow_box(big_total, false, options),
                _ => cd_stacked_vert_arrow_box(big_total, true, options),
            };
            let box_h = shaft_box.height;
            let box_d = shaft_box.depth;
            let shaft_w = shaft_box.width;

            // Side labels: KaTeX uses `style.sup()` for both left and right; scale via `Scaled`
            // so `to_display::RaiseBox` does not leave them at display size (unlike `OpLimits`).
            let left_box = label_above.map(|n| {
                cd_vcenter_side_label(
                    cd_side_label_scaled(n, options),
                    box_h,
                    box_d,
                    options.color,
                )
            });
            let right_box = label_below.map(|n| {
                cd_vcenter_side_label(
                    cd_side_label_scaled(n, options),
                    box_h,
                    box_d,
                    options.color,
                )
            });

            let left_w = left_box.as_ref().map(|box_| box_.width).unwrap_or(0.0);
            let right_w = right_box.as_ref().map(|box_| box_.width).unwrap_or(0.0);
            let has_side_labels = left_w > 0.0 || right_w > 0.0;
            let (total_w, children) = if has_side_labels {
                // Side-label overflow affects the raster canvas in RaTeX. Keep
                // it explicit for now while preserving the legacy label cases;
                // unlabeled shafts use the centered object-column model below.
                const SIDE_KERN: f64 = 0.11;
                let left_part = left_w + if left_w > 0.0 { SIDE_KERN } else { 0.0 };
                let right_part = if right_w > 0.0 { SIDE_KERN } else { 0.0 } + right_w;
                let inner_w = left_part + shaft_w + right_part;
                let total_w = target_col_width.max(inner_w);
                let outer_pad = (total_w - inner_w) / 2.0;
                let mut children = vec![LayoutBox::new_kern(outer_pad)];
                if let Some(lb) = left_box {
                    children.push(lb);
                    children.push(LayoutBox::new_kern(SIDE_KERN));
                }
                children.push(shaft_box);
                if let Some(rb) = right_box {
                    children.push(LayoutBox::new_kern(SIDE_KERN));
                    children.push(rb);
                }
                children.push(LayoutBox::new_kern(total_w - inner_w - outer_pad));
                (total_w, children)
            } else {
                let total_w = target_col_width.max(shaft_w);
                let pad = (total_w - shaft_w) / 2.0;
                (
                    total_w,
                    vec![
                        LayoutBox::new_kern(pad),
                        shaft_box,
                        LayoutBox::new_kern(total_w - shaft_w - pad),
                    ],
                )
            };

            LayoutBox {
                width: total_w,
                height: box_h,
                depth: box_d,
                content: BoxContent::HBox(children),
                color: options.color,
            }
        }

        // "none" or unknown: empty placeholder
        _ => LayoutBox::new_empty(),
    }
}

/// Layout a `\begin{CD}...\end{CD}` commutative diagram with two-pass stretching.
fn layout_cd(body: &[Vec<ParseNode>], options: &LayoutOptions) -> LayoutBox {
    let metrics = options.metrics();
    let pt = 1.0 / metrics.pt_per_em;
    // KaTeX CD uses `baselineskip = 3ex` (array.ts line 312), NOT the standard 12pt.
    let baselineskip = 3.0 * metrics.x_height;
    let arstrut_h = 0.7 * baselineskip;
    let arstrut_d = 0.3 * baselineskip;

    let num_rows = body.len();
    if num_rows == 0 {
        return LayoutBox::new_empty();
    }
    let num_cols = body.iter().map(|r| r.len()).max().unwrap_or(0);
    if num_cols == 0 {
        return LayoutBox::new_empty();
    }
    let has_vertical_side_labels = body.iter().flatten().any(|cell| {
        matches!(
            cell,
            ParseNode::CdArrow {
                direction,
                label_above,
                label_below,
                ..
            } if matches!(direction.as_str(), "down" | "up")
                && (cd_label_has_ink(label_above.as_deref())
                    || cd_label_has_ink(label_below.as_deref()))
        )
    });
    let has_horizontal_ink_label = body.iter().flatten().any(|cell| {
        matches!(
            cell,
            ParseNode::CdArrow {
                direction,
                label_above,
                label_below,
                ..
            } if matches!(direction.as_str(), "right" | "left")
                && (cd_label_has_ink(label_above.as_deref())
                    || cd_label_has_ink(label_below.as_deref()))
        )
    });
    let compact_labels_enabled =
        !(has_vertical_side_labels || num_cols > 3 && has_horizontal_ink_label);

    // `\jot` (3pt): added to every row depth below; include in vertical-arrow stretch span.
    let jot = 3.0 * pt;

    // ── Pass 1: layout all cells at natural size ────────────────────────────
    let mut cell_boxes: Vec<Vec<LayoutBox>> = Vec::with_capacity(num_rows);

    for row in body {
        let mut row_boxes: Vec<LayoutBox> = Vec::with_capacity(num_cols);

        for cell in row {
            let cbox = match cell {
                ParseNode::CdArrow {
                    direction,
                    label_above,
                    label_below,
                    ..
                } => {
                    layout_cd_arrow(
                        direction,
                        label_above.as_deref(),
                        label_below.as_deref(),
                        0.0, // natural size in pass 1
                        0.0, // natural column width
                        compact_labels_enabled,
                        options,
                    )
                }
                // KaTeX CD object cells are `styling` nodes; `sizingGroup` builds the body with
                // `buildExpression(..., false)` (see katex `functions/sizing.js`), so no inter-atom
                // math glue inside a cell — matching that avoids spurious Ord–Bin space (e.g. golden 0963).
                ParseNode::OrdGroup {
                    body: cell_body, ..
                } => layout_expression(cell_body, options, false),
                other => layout_node(other, options),
            };

            row_boxes.push(cbox);
        }

        // Pad missing columns
        while row_boxes.len() < num_cols {
            row_boxes.push(LayoutBox::new_empty());
        }
        cell_boxes.push(row_boxes);
    }

    let GridMetrics {
        mut col_widths,
        row_heights,
        mut row_depths,
    } = GridMetrics::measure(&cell_boxes, num_cols, arstrut_h, arstrut_d);

    // Column targets after pass 1 (max natural width per column). Horizontal shafts use per-cell
    // `target_size`, not this max — same as KaTeX: minCDarrowwidth is min-width on the glyph span,
    // not “stretch every row to column max”.
    let col_target_w: Vec<f64> = col_widths.clone();

    // ── Pass 2: re-layout arrow cells with target dimensions ───────────────
    for (r, row) in body.iter().enumerate() {
        let is_arrow_row = r % 2 == 1;
        for (c, cell) in row.iter().enumerate() {
            if let ParseNode::CdArrow {
                direction,
                label_above,
                label_below,
                ..
            } = cell
            {
                let is_horiz = matches!(direction.as_str(), "right" | "left" | "horiz_eq");
                let (new_box, col_w) = if !is_arrow_row && c % 2 == 1 && is_horiz {
                    let b = layout_cd_arrow(
                        direction,
                        label_above.as_deref(),
                        label_below.as_deref(),
                        cell_boxes[r][c].width,
                        col_target_w[c],
                        compact_labels_enabled,
                        options,
                    );
                    let w = b.width;
                    (b, w)
                } else if is_arrow_row && c % 2 == 0 {
                    // Vertical arrow: KaTeX uses a fixed `\Big` delimiter, not a
                    // stretchy arrow.  Match by using the pass-1 row span (without
                    // \jot) so the shaft height stays at the natural row h+d.
                    let v_span = row_heights[r] + row_depths[r];
                    let b = layout_cd_arrow(
                        direction,
                        label_above.as_deref(),
                        label_below.as_deref(),
                        v_span,
                        col_widths[c],
                        compact_labels_enabled,
                        options,
                    );
                    let w = b.width;
                    (b, w)
                } else {
                    continue;
                };
                col_widths[c] = col_widths[c].max(col_w);
                cell_boxes[r][c] = new_box;
            }
        }
    }

    // KaTeX `environments/cd.js` sets `addJot: true` for CD; `array.js` adds `\jot` (3pt) to each
    // row's depth (same as `layout_array` when `add_jot` is set).
    for rd in row_depths.iter_mut().take(num_rows.saturating_sub(1)) {
        *rd += jot;
    }

    // ── Build the final Array LayoutBox ────────────────────────────────────
    // KaTeX CD uses `pregap: 0.25, postgap: 0.25` per column (cd.ts line 216-217),
    // giving 0.5em between adjacent columns.  `hskipBeforeAndAfter` is unset (false),
    // so no outer padding.
    let col_gaps = vec![0.5; num_cols.saturating_sub(1)];

    // Column alignment: objects are centered, arrows are centered
    let col_aligns: Vec<u8> = (0..num_cols).map(|_| b'c').collect();

    // No vertical separators for CD
    let col_separators = vec![None; num_cols + 1];

    let grid_dimensions = GridMetrics {
        col_widths: col_widths.clone(),
        row_heights: row_heights.clone(),
        row_depths: row_depths.clone(),
    }
    .dimensions(&col_gaps, metrics.axis_height, 0.0, 0.0);
    let height = grid_dimensions.offset;
    let depth = grid_dimensions.total_height - grid_dimensions.offset;
    let total_width = grid_dimensions.width;

    // Build hlines_before_row (all empty for CD)
    let hlines_before_row: Vec<Vec<bool>> = (0..=num_rows).map(|_| vec![]).collect();

    LayoutBox {
        width: total_width,
        height,
        depth,
        content: BoxContent::Array {
            cells: cell_boxes,
            col_widths,
            col_aligns,
            row_heights,
            row_depths,
            col_gaps: col_gaps.into_boxed_slice(),
            offset: grid_dimensions.offset,
            content_x_offset: 0.0,
            content_x_end_offset: 0.0,
            col_separators,
            hlines_before_row,
            rule_thickness: 0.04 * pt,
            double_rule_sep: metrics.double_rule_sep,
            array_inner_width: total_width,
            tag_gap_em: 0.0,
            tag_col_width: 0.0,
            row_tags: (0..num_rows).map(|_| None).collect(),
            tags_left: false,
        },
        color: options.color,
    }
}

struct ProofTreeLayout {
    width: f64,
    height: f64,
    depth: f64,
    root_width: f64,
    root_center_x: f64,
    children: Vec<PlacedBox>,
    rules: Vec<ProofRule>,
}

/// MathJax `bussproofs` typesets each axiom / inference **cell** with upright letters
/// (roman), not math italic. RaTeX otherwise lays out cell bodies like ordinary math
/// (`P` → Math-Italic). Wrap the cell in `\mathrm{...}` so single-letter conclusions
/// match reference PNGs; relation symbols like `\Rightarrow` from `\fCenter` still
/// fall through `layout_with_font` to default `layout_node` and keep correct spacing.
fn layout_proof_roman_content(nodes: &[ParseNode], options: &LayoutOptions) -> LayoutBox {
    if nodes.is_empty() {
        return LayoutBox::new_empty();
    }
    let group = ParseNode::OrdGroup {
        mode: Mode::Math,
        body: nodes.to_vec(),
        semisimple: None,
        loc: None,
    };
    let wrapped = ParseNode::Font {
        mode: Mode::Math,
        font: "mathrm".to_string(),
        body: Box::new(group),
        loc: None,
    };
    layout_node(&wrapped, options)
}

fn layout_proof_cell_content(nodes: &[ParseNode], options: &LayoutOptions) -> LayoutBox {
    let mut box_ = layout_proof_roman_content(nodes, options);
    box_.depth = box_.depth.max(0.20);
    box_
}

fn layout_proof_tree(tree: &ProofBranch, options: &LayoutOptions) -> LayoutBox {
    let laid = layout_proof_branch(tree, options);
    LayoutBox {
        width: laid.width,
        height: laid.height,
        depth: laid.depth,
        content: BoxContent::ProofTree {
            children: laid.children,
            rules: laid.rules,
        },
        color: options.color,
    }
}

fn layout_proof_branch(tree: &ProofBranch, options: &LayoutOptions) -> ProofTreeLayout {
    let metrics = options.metrics();
    let rule_thickness = (metrics.default_rule_thickness * 2.0).max(0.08);
    // Bussproofs spacing constants (in em).
    // Tune premise spacing to better match MathJax/bussproofs horizontal spread.
    //   premise_gap  → horizontal gap between sibling premises
    //   label_gap    → gap between inference rule and its left/right label
    //   vertical_gap → vertical gap between rule and surrounding content
    let premise_gap = 2.0;
    let label_gap = 0.08;
    let vertical_gap = 0.32;

    let conclusion = layout_proof_cell_content(&tree.conclusion, options);
    let conclusion_width = conclusion.width;

    if tree.premises.is_empty() {
        let width = conclusion.width;
        let height = conclusion.height;
        let depth = conclusion.depth;
        return ProofTreeLayout {
            width,
            height,
            depth,
            root_width: width,
            root_center_x: width / 2.0,
            children: vec![PlacedBox {
                box_: conclusion,
                x: 0.0,
                baseline_y: height,
            }],
            rules: Vec::new(),
        };
    }

    let premise_layouts: Vec<ProofTreeLayout> = tree
        .premises
        .iter()
        .map(|p| layout_proof_branch(p, options))
        .collect();

    let premise_width = premise_layouts.iter().map(|p| p.width).sum::<f64>()
        + premise_gap * premise_layouts.len().saturating_sub(1) as f64;
    let premise_height = premise_layouts
        .iter()
        .map(|p| p.height)
        .fold(0.0_f64, f64::max);
    let premise_depth = premise_layouts
        .iter()
        .map(|p| p.depth)
        .fold(0.0_f64, f64::max);
    let premise_total = premise_height + premise_depth;

    let mut premise_root_left = f64::INFINITY;
    let mut premise_root_right = f64::NEG_INFINITY;
    let mut cursor = 0.0_f64;
    for premise in &premise_layouts {
        let root_left = cursor + premise.root_center_x - premise.root_width / 2.0;
        let root_right = cursor + premise.root_center_x + premise.root_width / 2.0;
        premise_root_left = premise_root_left.min(root_left);
        premise_root_right = premise_root_right.max(root_right);
        cursor += premise.width + premise_gap;
    }
    let rule_padding = if tree.premises.len() > 1 { 0.65 } else { 0.0 };
    let premise_rule_width = premise_root_right - premise_root_left + rule_padding;
    let premise_root_center = (premise_root_left + premise_root_right) / 2.0;
    let rule_width = premise_rule_width.max(conclusion_width).max(1.12);
    let core_width = premise_width.max(rule_width).max(conclusion_width);
    let center_x = core_width / 2.0;
    let premise_start_x = center_x - premise_width / 2.0;
    let root_center_x = premise_start_x + premise_root_center;
    let rule_x = root_center_x - rule_width / 2.0;
    let rule_y = premise_total + vertical_gap + rule_thickness / 2.0;
    let conclusion_x = root_center_x - conclusion_width / 2.0;
    let conclusion_baseline_y = rule_y + rule_thickness / 2.0 + vertical_gap + conclusion.height;

    let mut children = Vec::new();
    let mut rules = Vec::new();
    let mut min_x = 0.0_f64;
    let mut max_x = core_width;

    if tree.root_at_top {
        let conclusion_baseline_y = conclusion.height;
        let rule_y = conclusion.height + conclusion.depth + vertical_gap + rule_thickness / 2.0;
        let premise_top_y = rule_y + rule_thickness / 2.0 + vertical_gap;
        let premise_baseline_y = premise_top_y + premise_height;

        min_x = min_x.min(conclusion_x);
        max_x = max_x.max(conclusion_x + conclusion_width);
        children.push(PlacedBox {
            box_: conclusion,
            x: conclusion_x,
            baseline_y: conclusion_baseline_y,
        });

        let mut cursor = premise_start_x;
        for premise in premise_layouts {
            let child_top_y = premise_baseline_y - premise.height;
            for child in premise.children {
                let x = cursor + child.x;
                let y = child_top_y + child.baseline_y;
                min_x = min_x.min(x);
                max_x = max_x.max(x + child.box_.width);
                children.push(PlacedBox {
                    box_: child.box_,
                    x,
                    baseline_y: y,
                });
            }
            for rule in premise.rules {
                min_x = min_x.min(cursor + rule.x);
                max_x = max_x.max(cursor + rule.x + rule.width);
                rules.push(ProofRule {
                    x: cursor + rule.x,
                    y: child_top_y + rule.y,
                    width: rule.width,
                    thickness: rule.thickness,
                    dashed: rule.dashed,
                });
            }
            cursor += premise.width + premise_gap;
        }

        if !matches!(tree.line_style, ProofLineStyle::None) {
            rules.push(ProofRule {
                x: rule_x,
                y: rule_y,
                width: rule_width,
                thickness: rule_thickness,
                dashed: matches!(tree.line_style, ProofLineStyle::Dashed),
            });
        }

        if let Some(label) = tree.left_label.as_ref() {
            let label_opts = options.with_style(options.style.text());
            let label_box = layout_proof_roman_content(label, &label_opts);
            let x = rule_x - label_gap - label_box.width;
            let baseline_y = rule_y + (label_box.height - label_box.depth) / 2.0;
            min_x = min_x.min(x);
            max_x = max_x.max(x + label_box.width);
            children.push(PlacedBox {
                box_: label_box,
                x,
                baseline_y,
            });
        }

        if let Some(label) = tree.right_label.as_ref() {
            let label_opts = options.with_style(options.style.text());
            let label_box = layout_proof_roman_content(label, &label_opts);
            let x = rule_x + rule_width + label_gap;
            let baseline_y = rule_y + (label_box.height - label_box.depth) / 2.0;
            min_x = min_x.min(x);
            max_x = max_x.max(x + label_box.width);
            children.push(PlacedBox {
                box_: label_box,
                x,
                baseline_y,
            });
        }

        let mut root_center_x = root_center_x;
        if min_x < 0.0 {
            let shift_x = -min_x;
            for child in &mut children {
                child.x += shift_x;
            }
            for rule in &mut rules {
                rule.x += shift_x;
            }
            root_center_x += shift_x;
        }

        return ProofTreeLayout {
            width: max_x - min_x,
            height: conclusion_baseline_y,
            depth: children
                .iter()
                .map(|c| c.baseline_y + c.box_.depth - conclusion_baseline_y)
                .fold(0.0_f64, f64::max)
                .max(0.0),
            root_width: conclusion_width,
            root_center_x,
            children,
            rules,
        };
    }

    let mut cursor = premise_start_x;
    for premise in premise_layouts {
        let baseline_y = premise_height;
        let child_top_y = baseline_y - premise.height;
        for child in premise.children {
            let x = cursor + child.x;
            let y = child_top_y + child.baseline_y;
            min_x = min_x.min(x);
            max_x = max_x.max(x + child.box_.width);
            children.push(PlacedBox {
                box_: child.box_,
                x,
                baseline_y: y,
            });
        }
        for rule in premise.rules {
            min_x = min_x.min(cursor + rule.x);
            max_x = max_x.max(cursor + rule.x + rule.width);
            rules.push(ProofRule {
                x: cursor + rule.x,
                y: child_top_y + rule.y,
                width: rule.width,
                thickness: rule.thickness,
                dashed: rule.dashed,
            });
        }
        cursor += premise.width + premise_gap;
    }

    min_x = min_x.min(conclusion_x);
    max_x = max_x.max(conclusion_x + conclusion_width);
    children.push(PlacedBox {
        box_: conclusion,
        x: conclusion_x,
        baseline_y: conclusion_baseline_y,
    });

    if !matches!(tree.line_style, ProofLineStyle::None) {
        rules.push(ProofRule {
            x: rule_x,
            y: rule_y,
            width: rule_width,
            thickness: rule_thickness,
            dashed: matches!(tree.line_style, ProofLineStyle::Dashed),
        });
    }

    if let Some(label) = tree.left_label.as_ref() {
        let label_opts = options.with_style(options.style.text());
        let label_box = layout_proof_roman_content(label, &label_opts);
        let x = rule_x - label_gap - label_box.width;
        let baseline_y = rule_y + (label_box.height - label_box.depth) / 2.0;
        min_x = min_x.min(x);
        max_x = max_x.max(x + label_box.width);
        children.push(PlacedBox {
            box_: label_box,
            x,
            baseline_y,
        });
    }

    if let Some(label) = tree.right_label.as_ref() {
        let label_opts = options.with_style(options.style.text());
        let label_box = layout_proof_roman_content(label, &label_opts);
        let x = rule_x + rule_width + label_gap;
        let baseline_y = rule_y + (label_box.height - label_box.depth) / 2.0;
        min_x = min_x.min(x);
        max_x = max_x.max(x + label_box.width);
        children.push(PlacedBox {
            box_: label_box,
            x,
            baseline_y,
        });
    }

    let mut root_center_x = root_center_x;
    if min_x < 0.0 {
        let shift = -min_x;
        for child in &mut children {
            child.x += shift;
        }
        for rule in &mut rules {
            rule.x += shift;
        }
        root_center_x += shift;
    }

    ProofTreeLayout {
        width: max_x - min_x,
        height: conclusion_baseline_y,
        depth: children
            .iter()
            .map(|c| c.baseline_y + c.box_.depth - conclusion_baseline_y)
            .fold(0.0_f64, f64::max),
        root_width: conclusion_width,
        root_center_x,
        children,
        rules,
    }
}

fn horiz_brace_path(width: f64, height: f64, is_over: bool) -> Vec<PathCommand> {
    let mid = width / 2.0;
    let q = height * 0.6;
    if is_over {
        vec![
            PathCommand::MoveTo { x: 0.0, y: 0.0 },
            PathCommand::QuadTo {
                x1: 0.0,
                y1: -q,
                x: mid * 0.4,
                y: -q,
            },
            PathCommand::LineTo {
                x: mid - 0.05,
                y: -q,
            },
            PathCommand::LineTo { x: mid, y: -height },
            PathCommand::LineTo {
                x: mid + 0.05,
                y: -q,
            },
            PathCommand::LineTo {
                x: width - mid * 0.4,
                y: -q,
            },
            PathCommand::QuadTo {
                x1: width,
                y1: -q,
                x: width,
                y: 0.0,
            },
        ]
    } else {
        vec![
            PathCommand::MoveTo { x: 0.0, y: 0.0 },
            PathCommand::QuadTo {
                x1: 0.0,
                y1: q,
                x: mid * 0.4,
                y: q,
            },
            PathCommand::LineTo {
                x: mid - 0.05,
                y: q,
            },
            PathCommand::LineTo { x: mid, y: height },
            PathCommand::LineTo {
                x: mid + 0.05,
                y: q,
            },
            PathCommand::LineTo {
                x: width - mid * 0.4,
                y: q,
            },
            PathCommand::QuadTo {
                x1: width,
                y1: q,
                x: width,
                y: 0.0,
            },
        ]
    }
}

#[cfg(test)]
mod missing_glyph_width_em_tests {
    use super::{missing_glyph_height_em, missing_glyph_width_em};
    use ratex_font::get_global_metrics;

    #[test]
    fn supplementary_plane_emoji_is_one_em() {
        assert_eq!(missing_glyph_width_em('😊'), 1.0);
        assert_eq!(missing_glyph_width_em('🚀'), 1.0);
    }

    #[test]
    fn supplementary_plane_emoji_uses_shorter_box_height() {
        let m = get_global_metrics(0);
        let emoji_h = missing_glyph_height_em('😊', m);
        let default_h = (m.quad * 0.92).max(m.x_height);
        assert!(
            emoji_h < default_h,
            "tall placeholder box must not push \\sqrt past KaTeX's small-surd threshold"
        );
        assert!((emoji_h - 0.74).abs() < 1e-9);
    }

    #[test]
    fn dingbats_block_is_one_em() {
        assert_eq!(missing_glyph_width_em('\u{2708}'), 1.0); // AIRPLANE
    }

    #[test]
    fn miscellaneous_symbols_is_one_em() {
        assert_eq!(missing_glyph_width_em('\u{2605}'), 1.0); // ★ BLACK STAR
        assert_eq!(missing_glyph_width_em('\u{2615}'), 1.0); // ☕ HOT BEVERAGE
    }

    #[test]
    fn misc_symbols_and_arrows_is_one_em() {
        assert_eq!(missing_glyph_width_em('\u{2B50}'), 1.0); // ⭐ WHITE MEDIUM STAR
        assert_eq!(missing_glyph_width_em('\u{2B1B}'), 1.0); // ⬛ BLACK LARGE SQUARE
    }

    #[test]
    fn latin_without_metrics_stays_half_em() {
        assert_eq!(missing_glyph_width_em('z'), 0.5);
    }
}

#[cfg(test)]
mod vertical_delimiter_geometry_tests {
    use super::*;

    fn path_y_bounds(commands: &[PathCommand]) -> (f64, f64) {
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut include = |y: f64| {
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        };
        for command in commands {
            match command {
                PathCommand::MoveTo { y, .. } | PathCommand::LineTo { y, .. } => include(*y),
                PathCommand::CubicTo { y1, y2, y, .. } => {
                    include(*y1);
                    include(*y2);
                    include(*y);
                }
                PathCommand::QuadTo { y1, y, .. } => {
                    include(*y1);
                    include(*y);
                }
                PathCommand::Close => {}
            }
        }
        (min_y, max_y)
    }

    #[test]
    fn stacked_vertical_delimiters_use_exact_katex_repeat_counts() {
        let options = LayoutOptions::default();
        let piece = vert_repeat_piece_height(false);

        for (requested, repeats) in [(1.2, 0.0), (1.8, 1.0), (2.4, 2.0), (3.0, 3.0)] {
            let geometry = stacked_vert_geometry(requested, false, VertDelimKind::Sized, &options);
            let expected_total = (2.0 + repeats) * piece * KATEX_SIZED_DELIM_SCALE;
            assert!((geometry.height + geometry.depth - expected_total).abs() < 1e-9);
            assert!(
                (geometry.height
                    - geometry.depth
                    - 2.0 * options.metrics().axis_height * KATEX_SIZED_DELIM_SCALE)
                    .abs()
                    < 1e-9
            );
        }
    }

    #[test]
    fn stacked_vertical_bar_path_matches_box_bounds() {
        let options = LayoutOptions::default();
        let bar = make_vert_delim_box(2.4, false, VertDelimKind::Sized, &options);
        let BoxContent::SvgPath { commands, .. } = &bar.content else {
            panic!("stacked vertical bar must be a path");
        };
        let (min_y, max_y) = path_y_bounds(commands);
        assert!((min_y + bar.height).abs() < 1e-9);
        assert!((max_y - bar.depth).abs() < 1e-9);
    }

    #[test]
    fn natural_vertical_bar_uses_main_regular_metrics() {
        let options = LayoutOptions::default();
        let bar = make_natural_vert_delim_box(false, &options);
        let BoxContent::Glyph { font_id, char_code } = bar.content else {
            panic!("natural vertical bar must be a glyph");
        };
        let metrics = get_char_metrics(FontId::MainRegular, 8739).unwrap();
        assert_eq!(font_id, FontId::MainRegular);
        assert_eq!(char_code, 8739);
        assert_eq!(bar.width, metrics.width);
        assert_eq!(bar.height, metrics.height);
        assert_eq!(bar.depth, metrics.depth);
    }
}

#[cfg(test)]
mod shared_stretchy_geometry_tests {
    use super::*;
    use crate::to_display::to_display_list;
    use ratex_parser::parser::parse;
    use ratex_types::display_item::DisplayItem;

    fn test_box(width: f64, height: f64, depth: f64) -> LayoutBox {
        LayoutBox {
            width,
            height,
            depth,
            content: BoxContent::Empty,
            color: Color::BLACK,
        }
    }

    fn physical_multidot_dimensions(
        style: MathStyle,
        explicit_size_multiplier: f64,
    ) -> (f64, f64, f64) {
        let options = LayoutOptions::default()
            .with_style(style)
            .with_explicit_size_multiplier(explicit_size_multiplier);
        let scale = options.size_multiplier() * options.explicit_size_multiplier;
        let accent =
            layout_multidot_accent(test_box(0.4 / scale, 0.4 / scale, 0.1 / scale), 3, &options);
        let mark = match &accent.content {
            BoxContent::Accent { accent, .. } => accent,
            other => panic!("expected accent box, got {other:?}"),
        };
        (
            mark.width * scale,
            mark.height * scale,
            accent.width * scale,
        )
    }

    #[test]
    fn multidot_marks_keep_normal_size_in_script_styles() {
        let display = physical_multidot_dimensions(MathStyle::Display, 1.0);

        for style in [MathStyle::Script, MathStyle::ScriptScript] {
            let actual = physical_multidot_dimensions(style, 1.0);
            assert!((actual.0 - display.0).abs() < 1e-9);
            assert!((actual.1 - display.1).abs() < 1e-9);
            assert!((actual.2 - display.2).abs() < 1e-9);
        }
    }

    #[test]
    fn multidot_marks_keep_normal_size_under_explicit_sizing() {
        let normal = physical_multidot_dimensions(MathStyle::Display, 1.0);

        for (style, explicit_size_multiplier) in [
            (MathStyle::Display, 0.5),
            (MathStyle::Display, 0.9),
            (MathStyle::Display, 1.44),
            (MathStyle::Display, 2.488),
            (MathStyle::Script, 0.9),
            (MathStyle::Script, 2.488),
        ] {
            let actual = physical_multidot_dimensions(style, explicit_size_multiplier);
            assert!((actual.0 - normal.0).abs() < 1e-9);
            assert!((actual.1 - normal.1).abs() < 1e-9);
            assert!((actual.2 - normal.2).abs() < 1e-9);
        }
    }

    fn rendered_multidot_path_dimensions(latex: &str) -> (f64, f64) {
        let ast = parse(latex).unwrap();
        let display = to_display_list(&layout(&ast, &LayoutOptions::default()));
        let paths: Vec<&[PathCommand]> = display
            .items
            .iter()
            .filter_map(|item| match item {
                DisplayItem::Path { commands, .. } => Some(commands.as_slice()),
                _ => None,
            })
            .collect();
        assert_eq!(paths.len(), 1, "{latex}");

        let mut min_x = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut include = |x: f64, y: f64| {
            min_x = min_x.min(x);
            max_x = max_x.max(x);
            min_y = min_y.min(y);
            max_y = max_y.max(y);
        };
        for command in paths[0] {
            match command {
                PathCommand::MoveTo { x, y } | PathCommand::LineTo { x, y } => include(*x, *y),
                PathCommand::CubicTo {
                    x1,
                    y1,
                    x2,
                    y2,
                    x,
                    y,
                } => {
                    include(*x1, *y1);
                    include(*x2, *y2);
                    include(*x, *y);
                }
                PathCommand::QuadTo { x1, y1, x, y } => {
                    include(*x1, *y1);
                    include(*x, *y);
                }
                PathCommand::Close => {}
            }
        }
        (max_x - min_x, max_y - min_y)
    }

    #[test]
    fn multidot_explicit_size_commands_reset_the_rendered_mark_to_normal() {
        let normal_triple = rendered_multidot_path_dimensions(r"\dddot{x}");
        for latex in [
            r"\tiny\dddot{x}",
            r"\Large\dddot{x}",
            r"\small x_{\dddot{y}}",
        ] {
            let actual = rendered_multidot_path_dimensions(latex);
            assert!((actual.0 - normal_triple.0).abs() < 1e-9, "{latex}");
            assert!((actual.1 - normal_triple.1).abs() < 1e-9, "{latex}");
        }

        let normal_quadruple = rendered_multidot_path_dimensions(r"\ddddot{x}");
        for latex in [
            r"\small\ddddot{x}",
            r"\Huge\ddddot{x}",
            r"\Huge x_{\ddddot{y}}",
        ] {
            let actual = rendered_multidot_path_dimensions(latex);
            assert!((actual.0 - normal_quadruple.0).abs() < 1e-9, "{latex}");
            assert!((actual.1 - normal_quadruple.1).abs() < 1e-9, "{latex}");
        }
    }

    #[test]
    fn nested_explicit_sizing_replaces_the_outer_multiplier() {
        let normal = rendered_multidot_path_dimensions(r"\dddot{x}");
        let actual = rendered_multidot_path_dimensions(r"\small{\Large\dddot{x}}");
        assert!((actual.0 - normal.0).abs() < 1e-9);
        assert!((actual.1 - normal.1).abs() < 1e-9);
    }

    #[test]
    fn multidot_marks_keep_normal_size_in_combined_style_and_sizing() {
        let display = physical_multidot_dimensions(MathStyle::Display, 1.0);
        for (style, multiplier) in [
            (MathStyle::Script, 0.5),
            (MathStyle::ScriptScript, 0.9),
            (MathStyle::Script, 1.44),
            (MathStyle::ScriptScript, 2.488),
        ] {
            let actual = physical_multidot_dimensions(style, multiplier);
            assert!((actual.0 - display.0).abs() < 1e-9);
            assert!((actual.1 - display.1).abs() < 1e-9);
            assert!((actual.2 - display.2).abs() < 1e-9);
        }
    }

    #[test]
    fn stretchy_specs_derive_clearance_from_font_rule_metrics() {
        let options = LayoutOptions::default();
        let rule = options.metrics().default_rule_thickness;

        let widehat = StretchySpec::accent("\\widehat", false, &options);
        assert_eq!(widehat.placement, StretchyPlacement::Above);
        assert_eq!(widehat.gap, 0.0);

        let utilde = StretchySpec::accent("\\utilde", true, &options);
        assert_eq!(utilde.placement, StretchyPlacement::Below);
        assert!((utilde.gap - 3.0 * rule).abs() < 1e-9);

        let underbrace = StretchySpec::horizontal_brace(false, &options);
        assert_eq!(underbrace.placement, StretchyPlacement::Below);
        assert!((underbrace.gap - 2.5 * rule).abs() < 1e-9);
    }

    #[test]
    fn overline_geometry_matches_katex_vlist_rule_units() {
        let options = LayoutOptions::default();
        let rule = options.metrics().default_rule_thickness;
        let geometry = RuleGeometry::overline(&options);

        assert_eq!(geometry.thickness, rule);
        assert!((geometry.line_center_offset - 3.5 * rule).abs() < 1e-9);
        assert!((geometry.total_extent - 5.0 * rule).abs() < 1e-9);
    }
}

#[cfg(test)]
mod shared_grid_geometry_tests {
    use super::*;
    use ratex_parser::parser::parse;

    fn test_box(width: f64, height: f64, depth: f64) -> LayoutBox {
        LayoutBox {
            width,
            height,
            depth,
            content: BoxContent::Empty,
            color: Color::BLACK,
        }
    }

    #[test]
    fn grid_measurement_uses_column_maxima_and_row_ascent_depth() {
        let cells = vec![
            vec![test_box(1.0, 0.4, 0.1), test_box(0.5, 0.7, 0.0)],
            vec![test_box(1.5, 0.3, 0.2), test_box(0.25, 0.2, 0.4)],
        ];
        let grid = GridMetrics::measure(&cells, 2, 0.6, 0.25);

        assert_eq!(grid.col_widths, vec![1.5, 0.5]);
        assert_eq!(grid.row_heights, vec![0.7, 0.6]);
        assert_eq!(grid.row_depths, vec![0.25, 0.4]);
    }

    #[test]
    fn grid_dimensions_share_one_axis_and_padding_model() {
        let grid = GridMetrics {
            col_widths: vec![1.0, 2.0],
            row_heights: vec![0.7, 0.8],
            row_depths: vec![0.3, 0.2],
        };
        let dimensions = grid.dimensions(&[0.5], 0.25, 0.1, 0.2);

        assert!((dimensions.width - 3.8).abs() < 1e-9);
        assert!((dimensions.total_height - 2.0).abs() < 1e-9);
        assert!((dimensions.offset - 1.25).abs() < 1e-9);
    }

    fn cd_grid(latex: &str) -> (Vec<f64>, Vec<f64>) {
        fn find_array(lbox: &LayoutBox) -> Option<(&[f64], &[f64])> {
            match &lbox.content {
                BoxContent::Array {
                    col_widths,
                    col_gaps,
                    ..
                } => Some((col_widths, col_gaps)),
                BoxContent::HBox(children) => children.iter().find_map(find_array),
                BoxContent::Scaled { body, .. } | BoxContent::RaiseBox { body, .. } => {
                    find_array(body)
                }
                _ => None,
            }
        }

        let ast = parse(latex).unwrap();
        let lbox = layout(&ast, &LayoutOptions::default());
        let (widths, gaps) = find_array(&lbox).expect("CD must produce an array layout");
        (widths.to_vec(), gaps.to_vec())
    }

    fn assert_half_em_gaps(gaps: &[f64], latex: &str) {
        assert!(!gaps.is_empty(), "{latex}");
        assert!(
            gaps.iter().all(|gap| (*gap - 0.5).abs() < 1e-9),
            "{latex}: {gaps:?}"
        );
    }

    #[test]
    fn cd_arrow_labels_do_not_change_any_column_gap() {
        let unlabeled = r"\begin{CD} A @>>> B @>>> C \\ D @>>> E @>>> F \end{CD}";
        let labeled = r"\begin{CD} A @>f>g>> B @>>> C \\ D @>>> E @>>> F \end{CD}";
        let (_, unlabeled_gaps) = cd_grid(unlabeled);
        let (_, labeled_gaps) = cd_grid(labeled);

        assert_eq!(unlabeled_gaps, labeled_gaps);
        assert_half_em_gaps(&unlabeled_gaps, unlabeled);
    }

    #[test]
    fn cd_object_width_crossing_two_em_does_not_reflow_column_gaps() {
        let narrow = r"\begin{CD} \rule{1.99em}{0.1em} @>>> B \\ C @>>> D \end{CD}";
        let wide = r"\begin{CD} \rule{2.01em}{0.1em} @>>> B \\ C @>>> D \end{CD}";
        let (narrow_widths, narrow_gaps) = cd_grid(narrow);
        let (wide_widths, wide_gaps) = cd_grid(wide);

        assert!(narrow_widths[0] < 2.0, "{narrow_widths:?}");
        assert!(wide_widths[0] > 2.0, "{wide_widths:?}");
        assert_eq!(narrow_gaps, wide_gaps);
        assert_half_em_gaps(&narrow_gaps, narrow);
    }

    #[test]
    fn cd_column_gaps_are_always_half_an_em_for_matching_grid_structure() {
        for latex in [
            r"\begin{CD} A @>>> B \\ C @>>> D \end{CD}",
            r"\begin{CD} A @= B \\ C @= D \end{CD}",
            r"\begin{CD} A @>f>> B \\ C @>g>h>> D \end{CD}",
            r"\begin{CD} A @>>> B @>>> C \\ D @>>> E @>>> F \end{CD}",
        ] {
            let (_, gaps) = cd_grid(latex);
            assert_half_em_gaps(&gaps, latex);
        }
    }
}

#[cfg(test)]
mod operator_limits_geometry_tests {
    use super::*;
    use ratex_parser::parser::parse;

    #[test]
    fn limits_kern_uses_scaled_script_metrics_without_magic_padding() {
        let options = LayoutOptions::default();
        let script = parse("x").unwrap().remove(0);
        let base = LayoutBox {
            width: 1.0,
            height: 1.05,
            depth: 0.55,
            content: BoxContent::Empty,
            color: Color::BLACK,
        };

        let result = layout_op_limits_inner(&base, Some(&script), None, 0.0, 0.0, true, &options);
        let BoxContent::OpLimits {
            sup,
            sup_kern,
            sup_scale,
            ..
        } = &result.content
        else {
            panic!("expected operator limits box");
        };
        let script_box = sup.as_ref().unwrap();
        let expected = options
            .metrics()
            .big_op_spacing1
            .max(options.metrics().big_op_spacing3 - script_box.depth * sup_scale);
        assert!((*sup_kern - expected).abs() < 1e-9);
    }

    #[test]
    fn unicode_and_command_big_operators_have_identical_geometry() {
        let options = LayoutOptions::default();
        for (unicode, command) in [
            ("∏_i", "\\prod_i"),
            ("∑_i", "\\sum_i"),
            ("⋀_i", "\\bigwedge_i"),
            ("⋁_i", "\\bigvee_i"),
            ("⋂_i", "\\bigcap_i"),
            ("⋃_i", "\\bigcup_i"),
            ("⨀_i", "\\bigodot_i"),
            ("⨁_i", "\\bigoplus_i"),
            ("⨂_i", "\\bigotimes_i"),
        ] {
            let unicode_box = layout(&parse(unicode).unwrap(), &options);
            let command_box = layout(&parse(command).unwrap(), &options);
            assert_eq!(unicode_box.width, command_box.width, "{unicode}");
            assert_eq!(unicode_box.height, command_box.height, "{unicode}");
            assert_eq!(unicode_box.depth, command_box.depth, "{unicode}");
        }
    }

    #[test]
    fn operatorname_limits_keep_the_base_upright() {
        fn find_limits(layout_box: &LayoutBox) -> Option<&LayoutBox> {
            match &layout_box.content {
                BoxContent::OpLimits { .. } => Some(layout_box),
                BoxContent::HBox(children) => children.iter().find_map(find_limits),
                BoxContent::Scaled { body, .. } | BoxContent::RaiseBox { body, .. } => {
                    find_limits(body)
                }
                _ => None,
            }
        }

        fn glyph_fonts(layout_box: &LayoutBox, fonts: &mut Vec<FontId>) {
            match &layout_box.content {
                BoxContent::Glyph { font_id, .. } => fonts.push(*font_id),
                BoxContent::HBox(children) => {
                    for child in children {
                        glyph_fonts(child, fonts);
                    }
                }
                _ => {}
            }
        }

        let options = LayoutOptions::default();
        let result = layout(
            &parse("\\operatorname*{asin}\\limits_y x").unwrap(),
            &options,
        );
        let limits = find_limits(&result).expect("operatorname must use limits");
        let BoxContent::OpLimits { base, .. } = &limits.content else {
            unreachable!();
        };
        let mut fonts = Vec::new();
        glyph_fonts(base, &mut fonts);
        assert_eq!(fonts, vec![FontId::MainRegular; 4]);
    }
}

#[cfg(test)]
mod cjk_font_switching_tests {
    use super::super::to_display::to_display_list;
    use super::*;
    use ratex_parser::parser::parse;
    use ratex_types::display_item::DisplayItem;

    fn first_glyph_font_name(latex: &str) -> Option<String> {
        let ast = parse(latex).ok()?;
        let lbox = layout(&ast, &LayoutOptions::default());
        let dl = to_display_list(&lbox);
        for item in &dl.items {
            if let DisplayItem::GlyphPath { font, .. } = item {
                return Some(font.clone());
            }
        }
        None
    }

    #[test]
    fn cjk_in_text_uses_cjk_regular() {
        assert_eq!(
            first_glyph_font_name(r"\text{中}").as_deref(),
            Some("CJK-Regular")
        );
    }

    #[test]
    fn emoji_in_text_uses_cjk_regular() {
        assert_eq!(
            first_glyph_font_name(r"\text{😊}").as_deref(),
            Some("CJK-Regular")
        );
    }

    #[test]
    fn latin_in_text_is_not_cjk() {
        assert_ne!(
            first_glyph_font_name(r"\text{a}").as_deref(),
            Some("CJK-Regular")
        );
    }

    #[test]
    fn hiragana_in_text_uses_cjk_regular() {
        assert_eq!(
            first_glyph_font_name(r"\text{あ}").as_deref(),
            Some("CJK-Regular")
        );
    }
}
