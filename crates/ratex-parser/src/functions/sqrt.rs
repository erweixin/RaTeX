use std::collections::HashMap;

use crate::error::ParseResult;
use crate::functions::{define_function_full, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\sqrt"],
        "sqrt",
        1,
        1, // one optional arg [n]
        None,
        false,
        false,
        true,
        false,
        false,
        handle_sqrt,
    );
}

fn handle_sqrt(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let index = opt_args.into_iter().next().flatten();
    let body = args.into_iter().next().unwrap();

    Ok(ParseNode::Sqrt {
        mode: ctx.parser.mode,
        body: Box::new(body),
        index: index.map(Box::new),
        loc: None,
    })
}
