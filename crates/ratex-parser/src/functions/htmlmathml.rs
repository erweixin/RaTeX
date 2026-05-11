use std::collections::HashMap;

use crate::error::{ParseError, ParseResult};
use crate::functions::{define_function_full, ArgType, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\html@mathml"],
        "htmlmathml",
        2, 0, None,
        false, true, true, false, false,
        handle_htmlmathml,
    );

    define_function_full(
        map,
        &["\\htmlStyle"],
        "html",
        2, 0,
        Some(vec![ArgType::Raw, ArgType::Original]),
        false, true, true, false, false,
        handle_htmlstyle,
    );
}

fn handle_htmlmathml(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let html = ParseNode::ord_argument(args[0].clone());
    let mathml = ParseNode::ord_argument(args[1].clone());

    Ok(ParseNode::HtmlMathMl {
        mode: ctx.parser.mode,
        html,
        mathml,
        loc: None,
    })
}

fn handle_htmlstyle(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let mut args = args.into_iter();
    let style = match args.next() {
        Some(ParseNode::Raw { string, .. }) => string,
        _ => return Err(ParseError::msg("Expected raw style for \\htmlStyle")),
    };
    let body = ParseNode::ord_argument(args.next().unwrap());
    let mut attributes = HashMap::new();
    attributes.insert("style".to_string(), style);

    Ok(ParseNode::Html {
        mode: ctx.parser.mode,
        attributes,
        body,
        loc: None,
    })
}
