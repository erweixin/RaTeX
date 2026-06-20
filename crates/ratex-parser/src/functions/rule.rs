use std::collections::HashMap;

use crate::error::{ParseError, ParseResult};
use crate::functions::{define_function_full, ArgType, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\rule"],
        "rule",
        2,
        1,
        Some(vec![ArgType::Size, ArgType::Size, ArgType::Size]),
        false,
        true,
        true,
        false,
        false,
        handle_rule,
    );
}

fn handle_rule(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let shift = opt_args.into_iter().next().flatten().and_then(|n| {
        if let ParseNode::Size { value, .. } = n {
            Some(value)
        } else {
            None
        }
    });

    let mut args = args.into_iter();
    let width = match args.next() {
        Some(ParseNode::Size { value, .. }) => value,
        _ => return Err(ParseError::msg("Expected size for \\rule width")),
    };
    let height = match args.next() {
        Some(ParseNode::Size { value, .. }) => value,
        _ => return Err(ParseError::msg("Expected size for \\rule height")),
    };

    Ok(ParseNode::Rule {
        mode: ctx.parser.mode,
        shift,
        width,
        height,
        loc: None,
    })
}
