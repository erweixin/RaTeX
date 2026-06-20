use std::collections::HashMap;

use crate::error::ParseResult;
use crate::functions::{define_function_full, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\overline"],
        "overline",
        1,
        0,
        None,
        false,
        false,
        true,
        false,
        false,
        handle_overline,
    );

    define_function_full(
        map,
        &["\\underline"],
        "underline",
        1,
        0,
        None,
        false,
        true, // allowed_in_text
        true,
        false,
        false,
        handle_underline,
    );
}

fn handle_overline(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    Ok(ParseNode::Overline {
        mode: ctx.parser.mode,
        body: Box::new(args.into_iter().next().unwrap()),
        loc: None,
    })
}

fn handle_underline(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    Ok(ParseNode::Underline {
        mode: ctx.parser.mode,
        body: Box::new(args.into_iter().next().unwrap()),
        loc: None,
    })
}
