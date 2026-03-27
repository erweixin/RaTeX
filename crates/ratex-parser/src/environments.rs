use std::collections::HashMap;

use crate::error::{ParseError, ParseResult};
use crate::macro_expander::MacroDefinition;
use crate::parse_node::{AlignSpec, AlignType, Measurement, Mode, ParseNode, StyleStr};
use crate::parser::Parser;

// ── Environment registry ─────────────────────────────────────────────────

pub struct EnvContext<'a, 'b> {
    pub mode: Mode,
    pub env_name: String,
    pub parser: &'a mut Parser<'b>,
}

pub type EnvHandler = fn(
    ctx: &mut EnvContext,
    args: Vec<ParseNode>,
    opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode>;

pub struct EnvSpec {
    pub num_args: usize,
    pub num_optional_args: usize,
    pub handler: EnvHandler,
}

pub static ENVIRONMENTS: std::sync::LazyLock<HashMap<&'static str, EnvSpec>> =
    std::sync::LazyLock::new(|| {
        let mut map = HashMap::new();
        register_array(&mut map);
        register_matrix(&mut map);
        register_cases(&mut map);
        register_align(&mut map);
        register_gathered(&mut map);
        register_equation(&mut map);
        register_smallmatrix(&mut map);
        register_alignat(&mut map);
        register_subarray(&mut map);
        map
    });

// ── ArrayConfig ──────────────────────────────────────────────────────────

#[derive(Default)]
pub struct ArrayConfig {
    pub hskip_before_and_after: Option<bool>,
    pub add_jot: Option<bool>,
    pub cols: Option<Vec<AlignSpec>>,
    pub arraystretch: Option<f64>,
    pub col_separation_type: Option<String>,
    pub single_row: bool,
    pub empty_single_row: bool,
    pub max_num_cols: Option<usize>,
    pub leqno: Option<bool>,
}


// ── parseArray ───────────────────────────────────────────────────────────

fn get_hlines(parser: &mut Parser) -> ParseResult<Vec<bool>> {
    let mut hline_info = Vec::new();
    parser.consume_spaces()?;

    let mut nxt = parser.fetch()?.text.clone();
    if nxt == "\\relax" {
        parser.consume();
        parser.consume_spaces()?;
        nxt = parser.fetch()?.text.clone();
    }
    while nxt == "\\hline" || nxt == "\\hdashline" {
        parser.consume();
        hline_info.push(nxt == "\\hdashline");
        parser.consume_spaces()?;
        nxt = parser.fetch()?.text.clone();
    }
    Ok(hline_info)
}

fn d_cell_style(env_name: &str) -> Option<StyleStr> {
    if env_name.starts_with('d') {
        Some(StyleStr::Display)
    } else {
        Some(StyleStr::Text)
    }
}

pub fn parse_array(
    parser: &mut Parser,
    config: ArrayConfig,
    style: Option<StyleStr>,
) -> ParseResult<ParseNode> {
    parser.gullet.begin_group();

    if !config.single_row {
        parser
            .gullet
            .set_text_macro("\\cr", "\\\\\\relax");
    }

    let arraystretch = config.arraystretch.unwrap_or_else(|| {
        // Check if \arraystretch is defined as a macro (e.g., via \def\arraystretch{1.5})
        if let Some(def) = parser.gullet.get_macro("\\arraystretch") {
            let s = match def {
                MacroDefinition::Text(s) => s.clone(),
                MacroDefinition::Tokens { tokens, .. } => {
                    // Tokens are stored in reverse order (stack convention for expansion)
                    tokens.iter().rev().map(|t| t.text.as_str()).collect::<String>()
                }
                MacroDefinition::Function(_) => String::new(),
            };
            s.parse::<f64>().unwrap_or(1.0)
        } else {
            1.0
        }
    });

    parser.gullet.begin_group();

    let mut row: Vec<ParseNode> = Vec::new();
    let mut body: Vec<Vec<ParseNode>> = Vec::new();
    let mut row_gaps: Vec<Option<Measurement>> = Vec::new();
    let mut hlines_before_row: Vec<Vec<bool>> = Vec::new();

    hlines_before_row.push(get_hlines(parser)?);

    loop {
        let break_token = if config.single_row { "\\end" } else { "\\\\" };
        let cell_body = parser.parse_expression(false, Some(break_token))?;
        parser.gullet.end_group();
        parser.gullet.begin_group();

        let mut cell = ParseNode::OrdGroup {
            mode: parser.mode,
            body: cell_body,
            semisimple: None,
            loc: None,
        };

        if let Some(s) = style {
            cell = ParseNode::Styling {
                mode: parser.mode,
                style: s,
                body: vec![cell],
                loc: None,
            };
        }

        row.push(cell.clone());
        let next = parser.fetch()?.text.clone();

        if next == "&" {
            if let Some(max) = config.max_num_cols {
                if row.len() >= max {
                    return Err(ParseError::msg("Too many tab characters: &"));
                }
            }
            parser.consume();
        } else if next == "\\end" {
            // Check for trailing empty row and remove it
            let is_empty_trailing = if let Some(s) = style {
                if s == StyleStr::Text || s == StyleStr::Display {
                    if let ParseNode::Styling { body: ref sb, .. } = cell {
                        if let Some(ParseNode::OrdGroup {
                            body: ref ob, ..
                        }) = sb.first()
                        {
                            ob.is_empty()
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else if let ParseNode::OrdGroup { body: ref ob, .. } = cell {
                ob.is_empty()
            } else {
                false
            };

            body.push(row);

            if is_empty_trailing
                && (body.len() > 1 || !config.empty_single_row)
            {
                body.pop();
            }

            if hlines_before_row.len() < body.len() + 1 {
                hlines_before_row.push(vec![]);
            }
            break;
        } else if next == "\\\\" {
            parser.consume();
            let size = if parser.gullet.future().text != " " {
                parser.parse_size_group(true)?
            } else {
                None
            };
            let gap = size.and_then(|s| {
                if let ParseNode::Size { value, .. } = s {
                    Some(value)
                } else {
                    None
                }
            });
            row_gaps.push(gap);

            body.push(row);
            hlines_before_row.push(get_hlines(parser)?);
            row = Vec::new();
        } else {
            return Err(ParseError::msg(format!(
                "Expected & or \\\\ or \\cr or \\end, got '{}'",
                next
            )));
        }
    }

    parser.gullet.end_group();
    parser.gullet.end_group();

    Ok(ParseNode::Array {
        mode: parser.mode,
        body,
        row_gaps,
        hlines_before_row,
        cols: config.cols,
        col_separation_type: config.col_separation_type,
        hskip_before_and_after: config.hskip_before_and_after,
        add_jot: config.add_jot,
        arraystretch,
        tags: None,
        leqno: config.leqno,
        is_cd: None,
        loc: None,
    })
}

// ── array / darray ───────────────────────────────────────────────────────

fn register_array(map: &mut HashMap<&'static str, EnvSpec>) {
    fn handle_array(
        ctx: &mut EnvContext,
        args: Vec<ParseNode>,
        _opt_args: Vec<Option<ParseNode>>,
    ) -> ParseResult<ParseNode> {
        let colalign = match &args[0] {
            ParseNode::OrdGroup { body, .. } => body.clone(),
            other if other.is_symbol_node() => vec![other.clone()],
            _ => return Err(ParseError::msg("Invalid column alignment for array")),
        };

        let mut cols = Vec::new();
        for nde in &colalign {
            let ca = nde
                .symbol_text()
                .ok_or_else(|| ParseError::msg("Expected column alignment character"))?;
            match ca {
                "l" | "c" | "r" => cols.push(AlignSpec {
                    align_type: AlignType::Align,
                    align: Some(ca.to_string()),
                    pregap: None,
                    postgap: None,
                }),
                "|" => cols.push(AlignSpec {
                    align_type: AlignType::Separator,
                    align: Some("|".to_string()),
                    pregap: None,
                    postgap: None,
                }),
                ":" => cols.push(AlignSpec {
                    align_type: AlignType::Separator,
                    align: Some(":".to_string()),
                    pregap: None,
                    postgap: None,
                }),
                _ => {
                    return Err(ParseError::msg(format!(
                        "Unknown column alignment: {}",
                        ca
                    )))
                }
            }
        }

        let max_num_cols = cols.len();
        let config = ArrayConfig {
            cols: Some(cols),
            hskip_before_and_after: Some(true),
            max_num_cols: Some(max_num_cols),
            ..Default::default()
        };
        parse_array(ctx.parser, config, d_cell_style(&ctx.env_name))
    }

    for name in &["array", "darray"] {
        map.insert(
            name,
            EnvSpec {
                num_args: 1,
                num_optional_args: 0,
                handler: handle_array,
            },
        );
    }
}

// ── matrix variants ──────────────────────────────────────────────────────

fn register_matrix(map: &mut HashMap<&'static str, EnvSpec>) {
    fn handle_matrix(
        ctx: &mut EnvContext,
        _args: Vec<ParseNode>,
        _opt_args: Vec<Option<ParseNode>>,
    ) -> ParseResult<ParseNode> {
        let base_name = ctx.env_name.replace('*', "");
        let delimiters: Option<(&str, &str)> = match base_name.as_str() {
            "matrix" => None,
            "pmatrix" => Some(("(", ")")),
            "bmatrix" => Some(("[", "]")),
            "Bmatrix" => Some(("\\{", "\\}")),
            "vmatrix" => Some(("|", "|")),
            "Vmatrix" => Some(("\\Vert", "\\Vert")),
            _ => None,
        };

        let mut col_align = "c".to_string();

        // mathtools starred matrix: parse optional [l|c|r] alignment
        if ctx.env_name.ends_with('*') {
            ctx.parser.gullet.consume_spaces();
            if ctx.parser.gullet.future().text == "[" {
                ctx.parser.gullet.pop_token();
                ctx.parser.gullet.consume_spaces();
                let align_tok = ctx.parser.gullet.pop_token();
                if !"lcr".contains(align_tok.text.as_str()) {
                    return Err(ParseError::new(
                        "Expected l or c or r".to_string(),
                        Some(&align_tok),
                    ));
                }
                col_align = align_tok.text.clone();
                ctx.parser.gullet.consume_spaces();
                let close = ctx.parser.gullet.pop_token();
                if close.text != "]" {
                    return Err(ParseError::new(
                        "Expected ]".to_string(),
                        Some(&close),
                    ));
                }
            }
        }

        let config = ArrayConfig {
            hskip_before_and_after: Some(false),
            cols: Some(vec![AlignSpec {
                align_type: AlignType::Align,
                align: Some(col_align.clone()),
                pregap: None,
                postgap: None,
            }]),
            ..Default::default()
        };

        let mut res = parse_array(ctx.parser, config, d_cell_style(&ctx.env_name))?;

        // Fix cols to match actual number of columns
        if let ParseNode::Array {
            ref body,
            ref mut cols,
            ..
        } = res
        {
            let num_cols = body.iter().map(|r| r.len()).max().unwrap_or(0);
            *cols = Some(
                (0..num_cols)
                    .map(|_| AlignSpec {
                        align_type: AlignType::Align,
                        align: Some(col_align.to_string()),
                        pregap: None,
                        postgap: None,
                    })
                    .collect(),
            );
        }

        match delimiters {
            Some((left, right)) => Ok(ParseNode::LeftRight {
                mode: ctx.mode,
                body: vec![res],
                left: left.to_string(),
                right: right.to_string(),
                right_color: None,
                loc: None,
            }),
            None => Ok(res),
        }
    }

    for name in &[
        "matrix", "pmatrix", "bmatrix", "Bmatrix", "vmatrix", "Vmatrix",
        "matrix*", "pmatrix*", "bmatrix*", "Bmatrix*", "vmatrix*", "Vmatrix*",
    ] {
        map.insert(
            name,
            EnvSpec {
                num_args: 0,
                num_optional_args: 0,
                handler: handle_matrix,
            },
        );
    }
}

// ── cases / dcases / rcases / drcases ────────────────────────────────────

fn register_cases(map: &mut HashMap<&'static str, EnvSpec>) {
    fn handle_cases(
        ctx: &mut EnvContext,
        _args: Vec<ParseNode>,
        _opt_args: Vec<Option<ParseNode>>,
    ) -> ParseResult<ParseNode> {
        let config = ArrayConfig {
            arraystretch: Some(1.2),
            cols: Some(vec![
                AlignSpec {
                    align_type: AlignType::Align,
                    align: Some("l".to_string()),
                    pregap: Some(0.0),
                    postgap: Some(1.0),
                },
                AlignSpec {
                    align_type: AlignType::Align,
                    align: Some("l".to_string()),
                    pregap: Some(0.0),
                    postgap: Some(0.0),
                },
            ]),
            ..Default::default()
        };

        let res = parse_array(ctx.parser, config, d_cell_style(&ctx.env_name))?;

        let (left, right) = if ctx.env_name.contains('r') {
            (".", "\\}")
        } else {
            ("\\{", ".")
        };

        Ok(ParseNode::LeftRight {
            mode: ctx.mode,
            body: vec![res],
            left: left.to_string(),
            right: right.to_string(),
            right_color: None,
            loc: None,
        })
    }

    for name in &["cases", "dcases", "rcases", "drcases"] {
        map.insert(
            name,
            EnvSpec {
                num_args: 0,
                num_optional_args: 0,
                handler: handle_cases,
            },
        );
    }
}

// ── align / align* / aligned / split / alignat / alignat* / alignedat ────

fn handle_aligned(
    ctx: &mut EnvContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
        let is_split = ctx.env_name == "split";
        let is_alignat = ctx.env_name.contains("at");
        let sep_type = if is_alignat { "alignat" } else { "align" };

        let config = ArrayConfig {
            add_jot: Some(true),
            empty_single_row: true,
            col_separation_type: Some(sep_type.to_string()),
            max_num_cols: if is_split { Some(2) } else { None },
            ..Default::default()
        };

        let mut res = parse_array(ctx.parser, config, Some(StyleStr::Display))?;

        // Extract explicit column count from first arg (alignat only)
        let mut num_maths = 0usize;
        let mut explicit_cols = 0usize;
        if let Some(ParseNode::OrdGroup { body, .. }) = args.first() {
            let mut arg_str = String::new();
            for node in body {
                if let Some(t) = node.symbol_text() {
                    arg_str.push_str(t);
                }
            }
            if let Ok(n) = arg_str.parse::<usize>() {
                num_maths = n;
                explicit_cols = n * 2;
            }
        }
        let is_aligned = explicit_cols == 0;

        // Determine actual number of columns
        let mut num_cols = if let ParseNode::Array { ref body, .. } = res {
            body.iter().map(|r| r.len()).max().unwrap_or(0)
        } else {
            0
        };

        if let ParseNode::Array {
            body: ref mut array_body,
            ..
        } = res
        {
            for row in array_body.iter_mut() {
                // Prepend empty group at every even-indexed cell (2nd, 4th, ...)
                let mut i = 1;
                while i < row.len() {
                    if let ParseNode::Styling {
                        body: ref mut styling_body,
                        ..
                    } = row[i]
                    {
                        if let Some(ParseNode::OrdGroup {
                            body: ref mut og_body,
                            ..
                        }) = styling_body.first_mut()
                        {
                            og_body.insert(
                                0,
                                ParseNode::OrdGroup {
                                    mode: ctx.mode,
                                    body: vec![],
                                    semisimple: None,
                                    loc: None,
                                },
                            );
                        }
                    }
                    i += 2;
                }

                if !is_aligned {
                    let cur_maths = row.len() / 2;
                    if num_maths < cur_maths {
                        return Err(ParseError::msg(format!(
                            "Too many math in a row: expected {}, but got {}",
                            num_maths, cur_maths
                        )));
                    }
                } else if num_cols < row.len() {
                    num_cols = row.len();
                }
            }
        }

        if !is_aligned {
            num_cols = explicit_cols;
        }

        let mut cols = Vec::new();
        for i in 0..num_cols {
            let (align, pregap) = if i % 2 == 1 {
                ("l", 0.0)
            } else if i > 0 && is_aligned {
                ("r", 1.0)
            } else {
                ("r", 0.0)
            };
            cols.push(AlignSpec {
                align_type: AlignType::Align,
                align: Some(align.to_string()),
                pregap: Some(pregap),
                postgap: Some(0.0),
            });
        }

        if let ParseNode::Array {
            cols: ref mut array_cols,
            col_separation_type: ref mut array_sep_type,
            ..
        } = res
        {
            *array_cols = Some(cols);
            *array_sep_type = Some(
                if is_aligned { "align" } else { "alignat" }.to_string(),
            );
        }

    Ok(res)
}

fn register_align(map: &mut HashMap<&'static str, EnvSpec>) {
    for name in &["align", "align*", "aligned", "split"] {
        map.insert(
            name,
            EnvSpec {
                num_args: 0,
                num_optional_args: 0,
                handler: handle_aligned,
            },
        );
    }
}

// ── gathered / gather / gather* ──────────────────────────────────────────

fn register_gathered(map: &mut HashMap<&'static str, EnvSpec>) {
    fn handle_gathered(
        ctx: &mut EnvContext,
        _args: Vec<ParseNode>,
        _opt_args: Vec<Option<ParseNode>>,
    ) -> ParseResult<ParseNode> {
        let config = ArrayConfig {
            cols: Some(vec![AlignSpec {
                align_type: AlignType::Align,
                align: Some("c".to_string()),
                pregap: None,
                postgap: None,
            }]),
            add_jot: Some(true),
            col_separation_type: Some("gather".to_string()),
            empty_single_row: true,
            ..Default::default()
        };
        parse_array(ctx.parser, config, Some(StyleStr::Display))
    }

    for name in &["gathered", "gather", "gather*"] {
        map.insert(
            name,
            EnvSpec {
                num_args: 0,
                num_optional_args: 0,
                handler: handle_gathered,
            },
        );
    }
}

// ── equation / equation* ─────────────────────────────────────────────────

fn register_equation(map: &mut HashMap<&'static str, EnvSpec>) {
    fn handle_equation(
        ctx: &mut EnvContext,
        _args: Vec<ParseNode>,
        _opt_args: Vec<Option<ParseNode>>,
    ) -> ParseResult<ParseNode> {
        let config = ArrayConfig {
            empty_single_row: true,
            single_row: true,
            max_num_cols: Some(1),
            ..Default::default()
        };
        parse_array(ctx.parser, config, Some(StyleStr::Display))
    }

    for name in &["equation", "equation*"] {
        map.insert(
            name,
            EnvSpec {
                num_args: 0,
                num_optional_args: 0,
                handler: handle_equation,
            },
        );
    }
}

// ── smallmatrix ──────────────────────────────────────────────────────────

fn register_smallmatrix(map: &mut HashMap<&'static str, EnvSpec>) {
    fn handle_smallmatrix(
        ctx: &mut EnvContext,
        _args: Vec<ParseNode>,
        _opt_args: Vec<Option<ParseNode>>,
    ) -> ParseResult<ParseNode> {
        let config = ArrayConfig {
            arraystretch: Some(0.5),
            ..Default::default()
        };
        let mut res = parse_array(ctx.parser, config, Some(StyleStr::Script))?;
        if let ParseNode::Array {
            ref mut col_separation_type,
            ..
        } = res
        {
            *col_separation_type = Some("small".to_string());
        }
        Ok(res)
    }

    map.insert(
        "smallmatrix",
        EnvSpec {
            num_args: 0,
            num_optional_args: 0,
            handler: handle_smallmatrix,
        },
    );
}

// ── alignat / alignat* / alignedat ──────────────────────────────────────

fn register_alignat(map: &mut HashMap<&'static str, EnvSpec>) {
    for name in &["alignat", "alignat*", "alignedat"] {
        map.insert(
            name,
            EnvSpec {
                num_args: 1,
                num_optional_args: 0,
                handler: handle_aligned,
            },
        );
    }
}

// ── subarray ────────────────────────────────────────────────────────────

fn register_subarray(map: &mut HashMap<&'static str, EnvSpec>) {
    fn handle_subarray(
        ctx: &mut EnvContext,
        args: Vec<ParseNode>,
        _opt_args: Vec<Option<ParseNode>>,
    ) -> ParseResult<ParseNode> {
        let colalign = match &args[0] {
            ParseNode::OrdGroup { body, .. } => body.clone(),
            other if other.is_symbol_node() => vec![other.clone()],
            _ => return Err(ParseError::msg("Invalid column alignment for subarray")),
        };

        let mut cols = Vec::new();
        for nde in &colalign {
            let ca = nde
                .symbol_text()
                .ok_or_else(|| ParseError::msg("Expected column alignment character"))?;
            match ca {
                "l" | "c" => cols.push(AlignSpec {
                    align_type: AlignType::Align,
                    align: Some(ca.to_string()),
                    pregap: None,
                    postgap: None,
                }),
                _ => {
                    return Err(ParseError::msg(format!(
                        "Unknown column alignment: {}",
                        ca
                    )))
                }
            }
        }

        if cols.len() > 1 {
            return Err(ParseError::msg("{subarray} can contain only one column"));
        }

        let config = ArrayConfig {
            cols: Some(cols),
            hskip_before_and_after: Some(false),
            arraystretch: Some(0.5),
            ..Default::default()
        };

        let res = parse_array(ctx.parser, config, Some(StyleStr::Script))?;

        if let ParseNode::Array { ref body, .. } = res {
            if !body.is_empty() && body[0].len() > 1 {
                return Err(ParseError::msg("{subarray} can contain only one column"));
            }
        }

        Ok(res)
    }

    map.insert(
        "subarray",
        EnvSpec {
            num_args: 1,
            num_optional_args: 0,
            handler: handle_subarray,
        },
    );
}
