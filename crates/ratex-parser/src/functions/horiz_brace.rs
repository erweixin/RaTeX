use std::collections::HashMap;

use crate::error::ParseResult;
use crate::functions::{define_function_full, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &[
            "\\overbrace",
            "\\underbrace",
            "\\overbracket",
            "\\underbracket",
        ],
        "horizBrace",
        1,
        0,
        None,
        false,
        false,
        true,
        false,
        false,
        handle_horiz_brace,
    );
}

fn handle_horiz_brace(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let base = args.into_iter().next().unwrap();
    let is_over = ctx.func_name.starts_with("\\over");

    Ok(ParseNode::HorizBrace {
        mode: ctx.parser.mode,
        label: ctx.func_name.clone(),
        is_over,
        base: Box::new(base),
        loc: None,
    })
}
