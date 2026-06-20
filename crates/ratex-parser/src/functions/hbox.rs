use std::collections::HashMap;

use crate::error::ParseResult;
use crate::functions::{define_function_full, ArgType, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\hbox"],
        "hbox",
        1,
        0,
        Some(vec![ArgType::Text]),
        false,
        true,
        true,
        false,
        true,
        handle_hbox,
    );
}

fn handle_hbox(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    Ok(ParseNode::HBox {
        mode: ctx.parser.mode,
        body: ParseNode::ord_argument(args.into_iter().next().unwrap()),
        loc: None,
    })
}
