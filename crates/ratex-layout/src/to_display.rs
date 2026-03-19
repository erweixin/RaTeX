use ratex_types::display_item::{DisplayItem, DisplayList};
use ratex_types::path_command::PathCommand;

use crate::layout_box::{BoxContent, LayoutBox, VBoxChildKind};

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

        BoxContent::Rule { thickness } => {
            items.push(DisplayItem::Line {
                x,
                y: y - lbox.height * scale / 2.0,
                width: lbox.width * scale,
                thickness: thickness * scale,
                color: lbox.color,
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
                    base_x + base.width * scale
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
            rule_thickness,
            ..
        } => {
            let radical_width = lbox.width - index_offset - body.width;

            if let Some(index_box) = index {
                // Root index: slightly smaller than script (0.62), shifted right into the notch.
                const INDEX_SCALE: f64 = 0.62;
                const INDEX_PADDING_LEFT: f64 = 0.5; // move "3" to the right
                let index_baseline_y = y - lbox.height * scale + index_box.height * INDEX_SCALE * scale;
                emit_box(index_box, x + INDEX_PADDING_LEFT * scale, index_baseline_y, scale * INDEX_SCALE, items);
            }

            let surd_x = x + index_offset * scale;
            let surd_height = lbox.height + lbox.depth;
            items.push(DisplayItem::Path {
                x: surd_x,
                y: y - lbox.height * scale,
                commands: radical_surd_path(radical_width * scale, surd_height * scale),
                fill: false,
                color: lbox.color,
            });

            items.push(DisplayItem::Line {
                x: surd_x + radical_width * scale,
                y: y - lbox.height * scale,
                width: body.width * scale,
                thickness: rule_thickness * scale,
                color: lbox.color,
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
            row_heights,
            row_depths,
            col_gap,
            offset,
        } => {
            let mut cur_y = y - offset * scale;
            for (r, row) in cells.iter().enumerate() {
                let rh = row_heights[r];
                cur_y += rh * scale;
                let mut cur_x = x;
                for (c, cell) in row.iter().enumerate() {
                    let cw = col_widths[c];
                    let cell_x = cur_x + (cw - cell.width) * scale / 2.0;
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

/// Simplified radical surd path.
fn radical_surd_path(width: f64, height: f64) -> Vec<PathCommand> {
    vec![
        PathCommand::MoveTo { x: 0.0, y: height * 0.6 },
        PathCommand::LineTo {
            x: width * 0.4,
            y: height,
        },
        PathCommand::LineTo { x: width, y: 0.0 },
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
