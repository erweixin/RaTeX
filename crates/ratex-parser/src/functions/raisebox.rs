use std::collections::HashMap;

use crate::error::{ParseError, ParseResult};
use crate::functions::{define_function_full, ArgType, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\raisebox"],
        "raisebox",
        2,
        0,
        Some(vec![ArgType::Size, ArgType::HBox]),
        false,
        true,
        true,
        false,
        false,
        handle_raisebox,
    );
}

fn handle_raisebox(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let mut args = args.into_iter();
    let dy = match args.next() {
        Some(ParseNode::Size { value, .. }) => value,
        _ => return Err(ParseError::msg("Expected size for \\raisebox")),
    };
    let body = args.next().unwrap();

    Ok(ParseNode::RaiseBox {
        mode: ctx.parser.mode,
        dy,
        body: Box::new(body),
        loc: None,
    })
}
