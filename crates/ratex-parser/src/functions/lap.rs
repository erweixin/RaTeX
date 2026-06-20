use std::collections::HashMap;

use crate::error::ParseResult;
use crate::functions::{define_function_full, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\mathllap", "\\mathrlap", "\\mathclap"],
        "lap",
        1,
        0,
        None,
        false,
        true,
        true,
        false,
        false,
        handle_lap,
    );
}

fn handle_lap(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let alignment = ctx.func_name[5..].to_string();

    Ok(ParseNode::Lap {
        mode: ctx.parser.mode,
        alignment,
        body: Box::new(args.into_iter().next().unwrap()),
        loc: None,
    })
}
