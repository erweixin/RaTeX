use std::collections::HashMap;

use crate::error::ParseResult;
use crate::functions::{define_function, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function(
        map,
        &["\\nonumber", "\\notag"],
        "nonumber",
        0,
        handle_nonumber,
    );
}

fn handle_nonumber(
    ctx: &mut FunctionContext,
    _args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    Ok(ParseNode::NoNumber {
        mode: ctx.parser.mode,
        loc: None,
    })
}
