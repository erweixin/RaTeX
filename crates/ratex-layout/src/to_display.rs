use ratex_types::display_item::{DisplayItem, DisplayList};
use ratex_types::path_command::PathCommand;

use crate::layout_box::{BoxContent, LayoutBox, VBoxChildKind};
use crate::surd::surd_font_for_inner_height;

/// Unicode √ (U+221A), same glyph KaTeX uses for `\sqrt` surd.
const SURD_CHAR: u32 = 0x221A;

/// Convert a LayoutBox tree into a flat DisplayList with absolute coordinates.
///
/// The coordinate system:
/// - x increases to the right
/// - y increases downward (screen coordinates)
/// - The origin (0, 0) is at the top-left of the bounding box
/// - The baseline is at y = height
pub fn to_display_list(root: &LayoutBox) -> DisplayList {
    let mut items = Vec::new();
    let baseline_y = root.height;
    emit_box(root, 0.0, baseline_y, 1.0, &mut items);

    if items.is_empty() {
        return DisplayList {
            items,
            width: root.width,
            height: root.height,
            depth: root.depth,
        };
    }

    // Compute visual bounding box from actual display items.
    // This handles cases like \smash (zero height/depth) and \mathllap (zero width)
    // where content extends beyond the nominal box dimensions.
    // Only expand when the nominal dimension is very small (near-zero),
    // to avoid expanding for content that intentionally overflows (e.g. \clap inside \sum).
    let (min_x, max_x, min_y, max_y) = compute_visual_bounds(&items);

    let mut width = root.width;
    let mut height = root.height;
    let mut depth = root.depth;
    let total_h = root.height + root.depth;

    // Expand vertical dimensions only when nominal total height is near-zero (e.g. \smash)
    if total_h < 0.01 {
        if min_y < -0.001 {
            let extra = -min_y;
            height += extra;
            for item in &mut items {
                shift_item_y(item, extra);
            }
        }
        let nominal_bottom = height + depth;
        let shifted_max_y = if min_y < -0.001 { max_y - min_y } else { max_y };
        if shifted_max_y > nominal_bottom + 0.001 {
            depth = shifted_max_y - height;
        }
    }

    // Expand horizontal dimensions only when nominal width is near-zero (e.g. \mathllap, \mathrlap)
    if root.width < 0.01 {
        if min_x < -0.001 {
            let extra = -min_x;
            width += extra;
            for item in &mut items {
                shift_item_x(item, extra);
            }
        }
        let shifted_max_x = if min_x < -0.001 { max_x - min_x } else { max_x };
        if shifted_max_x > width + 0.001 {
            width = shifted_max_x;
        }
    }

    DisplayList {
        items,
        width,
        height,
        depth,
    }
}

/// Recursively emit DisplayItems for a LayoutBox at the given position.
///
/// `x`, `y` are the position of the box's baseline-left corner in absolute coordinates.
/// `scale` is the cumulative size multiplier (1.0 at root, 0.7 in script, 0.5 in scriptscript).
fn emit_box(lbox: &LayoutBox, x: f64, y: f64, scale: f64, items: &mut Vec<DisplayItem>) {
    match &lbox.content {
        BoxContent::HBox(children) => {
            let mut cur_x = x;
            for child in children {
                emit_box(child, cur_x, y, scale, items);
                cur_x += child.width * scale;
            }
        }

        BoxContent::VBox(children) => {
            let mut cur_y = y - lbox.height * scale;
            for child in children {
                match &child.kind {
                    VBoxChildKind::Box(b) => {
                        cur_y += b.height * scale;
                        emit_box(b, x + child.shift * scale, cur_y, scale, items);
                        cur_y += b.depth * scale;
                    }
                    VBoxChildKind::Kern(k) => {
                        cur_y += k * scale;
                    }
                }
            }
        }

        BoxContent::Glyph { font_id, char_code } => {
            let (w, h, d) = if let Some(metrics) = ratex_font::get_char_metrics(*font_id, *char_code) {
                (metrics.width, metrics.height, metrics.depth)
            } else {
                (lbox.width, lbox.height, lbox.depth)
            };
            let commands = glyph_placeholder_commands(w, h, d);
            items.push(DisplayItem::GlyphPath {
                x,
                y,
                scale,
                font: font_id.as_str().to_string(),
                char_code: *char_code,
                commands,
                color: lbox.color,
            });
        }

        BoxContent::Rule { thickness, raise } => {
            // Baseline at `y` (downward screen coords); bottom of ink is `raise` em above baseline.
            let cy = y - (raise + thickness / 2.0) * scale;
            items.push(DisplayItem::Line {
                x,
                y: cy,
                width: lbox.width * scale,
                thickness: thickness * scale,
                color: lbox.color,
                dashed: false,
            });
        }

        BoxContent::Fraction {
            numer,
            denom,
            numer_shift,
            denom_shift,
            bar_thickness,
            numer_scale: n_sc,
            denom_scale: d_sc,
        } => {
            let child_numer_scale = scale * n_sc;
            let child_denom_scale = scale * d_sc;

            let frac_x = x + (lbox.width * scale - numer.width * child_numer_scale) / 2.0;
            emit_box(numer, frac_x, y - numer_shift * scale, child_numer_scale, items);

            let frac_x = x + (lbox.width * scale - denom.width * child_denom_scale) / 2.0;
            emit_box(denom, frac_x, y + denom_shift * scale, child_denom_scale, items);

            if *bar_thickness > 0.0 {
                let metrics = ratex_font::get_global_metrics(0);
                items.push(DisplayItem::Line {
                    x,
                    y: y - metrics.axis_height * scale,
                    width: lbox.width * scale,
                    thickness: bar_thickness * scale,
                    color: lbox.color,
                    dashed: false,
                });
            }
        }

        BoxContent::SupSub {
            base,
            sup,
            sub,
            sup_shift,
            sub_shift,
            sup_scale: ss,
            sub_scale: bs,
            center_scripts,
            italic_correction,
        } => {
            let base_x = if *center_scripts {
                x + (lbox.width - base.width) * scale / 2.0
            } else {
                x
            };
            emit_box(base, base_x, y, scale, items);
            if let Some(sup_box) = sup {
                let child_scale = scale * ss;
                let sup_x = if *center_scripts {
                    x + (lbox.width * scale - sup_box.width * child_scale) / 2.0
                } else {
                    base_x + (base.width + italic_correction) * scale
                };
                emit_box(sup_box, sup_x, y - sup_shift * scale, child_scale, items);
            }
            if let Some(sub_box) = sub {
                let child_scale = scale * bs;
                let sub_x = if *center_scripts {
                    x + (lbox.width * scale - sub_box.width * child_scale) / 2.0
                } else {
                    base_x + base.width * scale
                };
                emit_box(sub_box, sub_x, y + sub_shift * scale, child_scale, items);
            }
        }

        BoxContent::Radical {
            body,
            index,
            index_offset,
            index_scale,
            rule_thickness,
            inner_height,
        } => {
            let radical_width = lbox.width - index_offset - body.width;

            if let Some(index_box) = index {
                // Root index (scriptscript): KaTeX `htmlBuilder` builds a vlist with
                // `positionType: "shift", positionData: -toShift` where
                // `toShift = 0.6 * (body.height - body.depth)` and `body` is the sqrt
                // inner+vlist (same height/depth as this radical box, excluding the index).
                // That aligns the root span with the sqrt body on the math baseline, then
                // shifts the glyph upward by `toShift` — not pinned to the top of the surd.
                //
                // Horizontal: KaTeX uses `margin-left: 5/18 em` on `.sqrt > .root` (`\mkern 5mu`).
                // Outline-font U+221A vs KaTeX SVG shifts the crook; ~0.55em parent-em inset matches
                // `tests/golden/fixtures/0744.png` better than 5/18 (~0.278) for our glyph pipeline.
                const INDEX_PADDING_LEFT_EM: f64 = 0.62;
                let to_shift = 0.6 * (lbox.height - lbox.depth);
                let index_baseline_y = y - to_shift * scale;
                let child_scale = scale * index_scale;
                emit_box(
                    index_box,
                    x + INDEX_PADDING_LEFT_EM * scale,
                    index_baseline_y,
                    child_scale,
                    items,
                );
            }

            let surd_x = x + index_offset * scale;
            let surd_font = surd_font_for_inner_height(*inner_height);
            let (gw, gh, gd) = ratex_font::get_char_metrics(surd_font, SURD_CHAR)
                .map(|m| (m.width, m.height, m.depth))
                .unwrap_or((radical_width, lbox.height, lbox.depth));
            let commands = glyph_placeholder_commands(gw, gh, gd);

            // Shift the surd glyph up so its top bar aligns with the vinculum.
            // The layout places the vinculum at `lbox.height` above the baseline, but the
            // font glyph height `gh` can be less than `lbox.height - rule_thickness`, causing
            // the glyph's bar to sit too close to the body content. We shift the glyph up by
            // the difference so the bar top is exactly at `lbox.height - rule_thickness`.
            let surd_shift = lbox.height - rule_thickness - gh;
            let surd_y = y - surd_shift * scale;
            items.push(DisplayItem::GlyphPath {
                x: surd_x,
                y: surd_y,
                scale,
                font: surd_font.as_str().to_string(),
                char_code: SURD_CHAR,
                commands,
                color: lbox.color,
            });

            // Vinculum: a horizontal rule extending the glyph's top bar over the body.
            // Center it on the glyph's top bar position.
            let line_center_y = surd_y - gh * scale + (rule_thickness * scale) / 2.0;
            items.push(DisplayItem::Line {
                x: surd_x + radical_width * scale,
                y: line_center_y,
                width: body.width * scale,
                thickness: rule_thickness * scale,
                color: lbox.color,
                dashed: false,
            });

            emit_box(body, surd_x + radical_width * scale, y, scale, items);
        }

        BoxContent::OpLimits {
            base,
            sup,
            sub,
            base_shift,
            sup_kern,
            sub_kern,
            slant,
            sup_scale: ss,
            sub_scale: bs,
        } => {
            let base_x = x + (lbox.width - base.width) * scale / 2.0;
            emit_box(base, base_x, y + base_shift * scale, scale, items);

            if let Some(sup_box) = sup {
                let child_scale = scale * ss;
                let sup_x = x + (lbox.width * scale - sup_box.width * child_scale) / 2.0 + slant * scale / 2.0;
                let sup_y = y - (base.height - base_shift) * scale - sup_kern * scale - sup_box.depth * child_scale;
                emit_box(sup_box, sup_x, sup_y, child_scale, items);
            }
            if let Some(sub_box) = sub {
                let child_scale = scale * bs;
                let sub_x = x + (lbox.width * scale - sub_box.width * child_scale) / 2.0 - slant * scale / 2.0;
                let sub_y = y + (base.depth + base_shift) * scale + sub_kern * scale + sub_box.height * child_scale;
                emit_box(sub_box, sub_x, sub_y, child_scale, items);
            }
        }

        BoxContent::Accent {
            base,
            accent,
            clearance,
            skew,
            is_below,
        } => {
            emit_box(base, x, y, scale, items);
            if *is_below {
                let accent_x = x + (base.width - accent.width) * scale / 2.0;
                let accent_y = y + base.depth * scale + accent.height * scale;
                emit_box(accent, accent_x, accent_y, scale, items);
            } else {
                let accent_x = x + (base.width - accent.width) * scale / 2.0 + skew * scale;
                // Position accent so its TOP is at (clearance + effective_accent_height) above baseline.
                // For SVG accents (height=0, depth=h), position at clearance above baseline.
                // For glyph accents (height>0, depth=0), the visual mark is at the top of the glyph.
                // Place the glyph so its top aligns with clearance + small_mark_height above baseline.
                let is_svg_accent = accent.height <= 0.001;
                let accent_y = if is_svg_accent {
                    y - clearance * scale - accent.depth * scale
                } else {
                    // Glyph accent: position baseline so top of glyph = clearance above text baseline
                    // accent_y - accent.height = y - (lbox.height) where lbox.height = clearance + eff_h
                    // => accent_y = y - clearance - eff_h + accent.height
                    // Simpler: shift glyph so top is at y - clearance - small_gap
                    y - clearance * scale + (accent.height - 0.35_f64.min(accent.height)) * scale
                };
                emit_box(accent, accent_x, accent_y, scale, items);
            }
        }

        BoxContent::LeftRight { left, right, inner } => {
            let mut cur_x = x;
            emit_box(left, cur_x, y, scale, items);
            cur_x += left.width * scale;
            emit_box(inner, cur_x, y, scale, items);
            cur_x += inner.width * scale;
            emit_box(right, cur_x, y, scale, items);
        }

        BoxContent::Array {
            cells,
            col_widths,
            col_aligns,
            row_heights,
            row_depths,
            col_gap,
            offset,
            content_x_offset,
            col_separators,
            hlines_before_row,
            rule_thickness,
            double_rule_sep,
        } => {
            let y_top = y - offset * scale;
            let array_total_height = (lbox.height + lbox.depth) * scale;
            let line_thickness = rule_thickness * scale;

            // Compute y positions of row boundaries (0 = top of array, num_rows = bottom).
            let mut boundary_ys: Vec<f64> = Vec::with_capacity(cells.len() + 1);
            let mut cur_boundary = y_top;
            boundary_ys.push(cur_boundary);
            for r in 0..cells.len() {
                cur_boundary += (row_heights[r] + row_depths[r]) * scale;
                boundary_ys.push(cur_boundary);
            }

            // Draw horizontal lines (hlines) before/after each row.
            // Each entry in hlines vec is one \hline (false) or \hdashline (true).
            // Consecutive rules are separated by double_rule_sep (= \doublerulesep = 2pt).
            //
            // Extra space for n > 1 hlines was already added to the layout:
            //   - r == 0: added to row_heights[0], so lines start at boundary_ys[0] going down.
            //   - r >= 1: added to row_depths[r-1], so lines occupy the range
            //             [boundary_ys[r] - (n-1)*rule_step, boundary_ys[r]], above row r.
            let rule_step = line_thickness + double_rule_sep * scale;
            for (r, hlines) in hlines_before_row.iter().enumerate() {
                if r < boundary_ys.len() {
                    let n = hlines.len();
                    let start_y = if r == 0 {
                        boundary_ys[0]
                    } else {
                        boundary_ys[r] - (n.saturating_sub(1)) as f64 * rule_step
                    };
                    for (i, &is_dashed) in hlines.iter().enumerate() {
                        items.push(DisplayItem::Line {
                            x,
                            y: start_y + i as f64 * rule_step,
                            width: lbox.width * scale,
                            thickness: line_thickness,
                            color: lbox.color,
                            dashed: is_dashed,
                        });
                    }
                }
            }

            // Draw vertical column separator lines ('|').
            // Separator at position i has local x = content_x_offset - col_gap/2 + sum(col_widths[..i]) + col_gap * i.
            let col_gap_half = col_gap / 2.0;
            for (i, &has_sep) in col_separators.iter().enumerate() {
                if has_sep {
                    let prefix_w: f64 = col_widths[..i].iter().sum();
                    let local_x = content_x_offset - col_gap_half + prefix_w + col_gap * i as f64;
                    let abs_x = x + local_x * scale - line_thickness / 2.0;
                    items.push(DisplayItem::Rect {
                        x: abs_x,
                        y: y_top,
                        width: line_thickness,
                        height: array_total_height,
                        color: lbox.color,
                    });
                }
            }

            // Render cells.
            let mut cur_y = y_top;
            for (r, row) in cells.iter().enumerate() {
                let rh = row_heights[r];
                cur_y += rh * scale;
                let mut cur_x = x + content_x_offset * scale;
                for (c, cell) in row.iter().enumerate() {
                    let cw = col_widths[c];
                    let align = col_aligns.get(c).copied().unwrap_or(b'c');
                    let cell_x = match align {
                        b'l' => cur_x,
                        b'r' => cur_x + (cw - cell.width) * scale,
                        _ => cur_x + (cw - cell.width) * scale / 2.0,
                    };
                    emit_box(cell, cell_x, cur_y, scale, items);
                    cur_x += cw * scale;
                    if c + 1 < row.len() {
                        cur_x += col_gap * scale;
                    }
                }
                cur_y += row_depths[r] * scale;
            }
        }

        BoxContent::SvgPath { commands, fill } => {
            let scaled: Vec<PathCommand> = commands
                .iter()
                .map(|c| scale_path_command(c, scale))
                .collect();
            items.push(DisplayItem::Path {
                x,
                y,
                commands: scaled,
                fill: *fill,
                color: lbox.color,
            });
        }

        BoxContent::Framed {
            body,
            padding,
            border_thickness,
            has_border,
            bg_color,
            border_color,
        } => {
            let outer_w = lbox.width * scale;
            let outer_h = lbox.height * scale;
            let outer_d = lbox.depth * scale;
            let top_y = y - outer_h;

            // Background fill
            if let Some(bg) = bg_color {
                items.push(DisplayItem::Rect {
                    x,
                    y: top_y,
                    width: outer_w,
                    height: outer_h + outer_d,
                    color: *bg,
                });
            }

            // Border (4 sides as Rect strips)
            if *has_border {
                let bt = border_thickness * scale;
                // Top
                items.push(DisplayItem::Rect {
                    x,
                    y: top_y,
                    width: outer_w,
                    height: bt,
                    color: *border_color,
                });
                // Bottom
                items.push(DisplayItem::Rect {
                    x,
                    y: y + outer_d - bt,
                    width: outer_w,
                    height: bt,
                    color: *border_color,
                });
                // Left
                items.push(DisplayItem::Rect {
                    x,
                    y: top_y,
                    width: bt,
                    height: outer_h + outer_d,
                    color: *border_color,
                });
                // Right
                items.push(DisplayItem::Rect {
                    x: x + outer_w - bt,
                    y: top_y,
                    width: bt,
                    height: outer_h + outer_d,
                    color: *border_color,
                });
            }

            // Body content (shifted by padding + border from left baseline)
            let inner_offset = (padding + border_thickness) * scale;
            emit_box(body, x + inner_offset, y, scale, items);
        }

        BoxContent::RaiseBox { body, shift } => {
            emit_box(body, x, y - shift * scale, scale, items);
        }

        BoxContent::Scaled { body, child_scale } => {
            emit_box(body, x, y, scale * child_scale, items);
        }

        BoxContent::Angl { path_commands, body } => {
            let scaled: Vec<PathCommand> = path_commands
                .iter()
                .map(|c| scale_path_command(c, scale))
                .collect();
            items.push(DisplayItem::Path {
                x,
                y,
                commands: scaled,
                fill: false,
                color: lbox.color,
            });
            emit_box(body, x, y, scale, items);
        }

        BoxContent::Overline { body, rule_thickness } => {
            emit_box(body, x, y, scale, items);
            // Rule center is at 2.5 * rule_thickness above the body's top
            let rule_center_y = y - (body.height + 2.5 * rule_thickness) * scale;
            items.push(DisplayItem::Line {
                x,
                y: rule_center_y,
                width: lbox.width * scale,
                thickness: rule_thickness * scale,
                color: lbox.color,
                dashed: false,
            });
        }

        BoxContent::Underline { body, rule_thickness } => {
            emit_box(body, x, y, scale, items);
            // Rule center is at 2.5 * rule_thickness below the body's bottom
            let rule_center_y = y + (body.depth + 2.5 * rule_thickness) * scale;
            items.push(DisplayItem::Line {
                x,
                y: rule_center_y,
                width: lbox.width * scale,
                thickness: rule_thickness * scale,
                color: lbox.color,
                dashed: false,
            });
        }

        BoxContent::Kern | BoxContent::Empty => {}
    }
}

fn scale_path_command(cmd: &PathCommand, scale: f64) -> PathCommand {
    match *cmd {
        PathCommand::MoveTo { x, y } => PathCommand::MoveTo {
            x: x * scale,
            y: y * scale,
        },
        PathCommand::LineTo { x, y } => PathCommand::LineTo {
            x: x * scale,
            y: y * scale,
        },
        PathCommand::CubicTo { x1, y1, x2, y2, x, y } => PathCommand::CubicTo {
            x1: x1 * scale,
            y1: y1 * scale,
            x2: x2 * scale,
            y2: y2 * scale,
            x: x * scale,
            y: y * scale,
        },
        PathCommand::QuadTo { x1, y1, x, y } => PathCommand::QuadTo {
            x1: x1 * scale,
            y1: y1 * scale,
            x: x * scale,
            y: y * scale,
        },
        PathCommand::Close => PathCommand::Close,
    }
}

/// Placeholder glyph path: a simple rectangle matching the character metrics.
fn glyph_placeholder_commands(width: f64, height: f64, depth: f64) -> Vec<PathCommand> {
    vec![
        PathCommand::MoveTo { x: 0.0, y: -height },
        PathCommand::LineTo { x: width, y: -height },
        PathCommand::LineTo {
            x: width,
            y: depth,
        },
        PathCommand::LineTo { x: 0.0, y: depth },
        PathCommand::Close,
    ]
}

/// Compute the visual bounding box from glyph, line, and rect items.
/// Excludes Path items which may have extreme coordinates (e.g. KaTeX SVG viewBox artifacts).
/// Returns (min_x, max_x, min_y, max_y) in em coordinates.
fn compute_visual_bounds(items: &[DisplayItem]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::MAX;
    let mut max_x = f64::MIN;
    let mut min_y = f64::MAX;
    let mut max_y = f64::MIN;

    for item in items {
        match item {
            DisplayItem::GlyphPath { x, y, scale, commands, .. } => {
                for cmd in commands {
                    if let PathCommand::MoveTo { x: cx, y: cy }
                        | PathCommand::LineTo { x: cx, y: cy } = cmd
                    {
                        let abs_x = x + cx * scale;
                        let abs_y = y + cy * scale;
                        min_x = min_x.min(abs_x);
                        max_x = max_x.max(abs_x);
                        min_y = min_y.min(abs_y);
                        max_y = max_y.max(abs_y);
                    }
                }
            }
            DisplayItem::Line { x, y, width, thickness, .. } => {
                min_x = min_x.min(*x);
                max_x = max_x.max(x + width);
                min_y = min_y.min(y - thickness / 2.0);
                max_y = max_y.max(y + thickness / 2.0);
            }
            DisplayItem::Rect { x, y, width, height, .. } => {
                min_x = min_x.min(*x);
                max_x = max_x.max(x + width);
                min_y = min_y.min(*y);
                max_y = max_y.max(y + height);
            }
            // Skip Path items — they may contain extreme coordinates
            // (e.g. \phase SVG paths with viewBox width 400000)
            DisplayItem::Path { .. } => {}
        }
    }

    (min_x, max_x, min_y, max_y)
}

fn shift_item_y(item: &mut DisplayItem, dy: f64) {
    match item {
        DisplayItem::GlyphPath { y, .. } => *y += dy,
        DisplayItem::Line { y, .. } => *y += dy,
        DisplayItem::Rect { y, .. } => *y += dy,
        DisplayItem::Path { y, .. } => *y += dy,
    }
}

fn shift_item_x(item: &mut DisplayItem, dx: f64) {
    match item {
        DisplayItem::GlyphPath { x, .. } => *x += dx,
        DisplayItem::Line { x, .. } => *x += dx,
        DisplayItem::Rect { x, .. } => *x += dx,
        DisplayItem::Path { x, .. } => *x += dx,
    }
}

