use std::collections::HashMap;

use crate::error::ParseResult;
use crate::functions::{define_function_full, ArgType, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &[
            "\\mathrm",
            "\\mathit",
            "\\mathbf",
            "\\mathnormal",
            "\\mathsfit",
            "\\mathbb",
            "\\mathcal",
            "\\mathfrak",
            "\\mathscr",
            "\\mathsf",
            "\\mathtt",
            "\\Bbb",
            "\\bold",
            "\\frak",
        ],
        "font",
        1,
        0,
        None,
        true, // allowed_in_argument
        false,
        true,
        false,
        false,
        handle_font,
    );

    // \boldsymbol and \bm produce mclass nodes
    define_function_full(
        map,
        &["\\boldsymbol", "\\bm"],
        "mclass",
        1,
        0,
        None,
        false,
        false,
        true,
        false,
        false,
        handle_boldsymbol,
    );

    // \emph — emphasis (italic in text mode)
    define_function_full(
        map,
        &["\\emph"],
        "text",
        1,
        0,
        Some(vec![ArgType::Text]),
        false,
        true,
        true,
        false,
        false,
        handle_emph,
    );

    // Old-style font switching commands: \rm, \sf, \tt, \bf, \it, \cal
    define_function_full(
        map,
        &["\\rm", "\\sf", "\\tt", "\\bf", "\\it", "\\cal"],
        "font",
        0,
        0,
        None,
        false,
        true,
        true,
        false,
        false,
        handle_old_font,
    );
}

fn handle_font(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let body = ParseNode::normalize_argument(args.into_iter().next().unwrap());

    let font_aliases: &[(&str, &str)] = &[
        ("\\Bbb", "\\mathbb"),
        ("\\bold", "\\mathbf"),
        ("\\frak", "\\mathfrak"),
        ("\\bm", "\\boldsymbol"),
    ];

    let mut func = ctx.func_name.clone();
    for &(alias, target) in font_aliases {
        if func == alias {
            func = target.to_string();
            break;
        }
    }

    let font_name = func.strip_prefix('\\').unwrap_or(func.as_str()).to_string();

    Ok(ParseNode::Font {
        mode: ctx.parser.mode,
        font: font_name,
        body: Box::new(body),
        loc: None,
    })
}

fn handle_old_font(
    ctx: &mut FunctionContext,
    _args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let font_name = format!("math{}", &ctx.func_name[1..]);
    let break_on = ctx.break_on_token_text.clone();
    let body = ctx.parser.parse_expression(true, break_on.as_deref())?;

    Ok(ParseNode::Font {
        mode: ctx.parser.mode,
        font: font_name,
        body: Box::new(ParseNode::OrdGroup {
            mode: ctx.parser.mode,
            body,
            semisimple: None,
            loc: None,
        }),
        loc: None,
    })
}

fn handle_boldsymbol(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let body = args.into_iter().next().unwrap();

    Ok(ParseNode::MClass {
        mode: ctx.parser.mode,
        mclass: "mord".to_string(),
        body: vec![ParseNode::Font {
            mode: ctx.parser.mode,
            font: "boldsymbol".to_string(),
            body: Box::new(body),
            loc: None,
        }],
        is_character_box: false,
        loc: None,
    })
}

fn handle_emph(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let body = args.into_iter().next().unwrap();
    let body_vec = ParseNode::ord_argument(body);
    Ok(ParseNode::Text {
        mode: ctx.parser.mode,
        body: body_vec,
        font: Some("\\emph".to_string()),
        loc: None,
    })
}
