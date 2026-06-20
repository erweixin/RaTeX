use std::collections::HashMap;

use crate::error::ParseResult;
use crate::functions::{define_function_full, ArgType, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &[
            "\\text",
            "\\textrm",
            "\\textsf",
            "\\texttt",
            "\\textnormal",
            "\\textbf",
            "\\textit",
            "\\textmd",
            "\\textup",
            "\\textsl",
            "\\textsc",
        ],
        "text",
        1,
        0,
        Some(vec![ArgType::Text]),
        true, // allowed_in_argument
        true, // allowed_in_text
        true,
        false,
        false,
        handle_text,
    );
}

fn handle_text(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let body = args.into_iter().next().unwrap();
    let body_vec = ParseNode::ord_argument(body);

    let font = Some(ctx.func_name.clone());

    Ok(ParseNode::Text {
        mode: ctx.parser.mode,
        body: body_vec,
        font,
        loc: None,
    })
}
