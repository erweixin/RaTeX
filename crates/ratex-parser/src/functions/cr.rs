use std::collections::HashMap;

use crate::error::ParseResult;
use crate::functions::{define_function_full, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\\\", "\\newline"],
        "cr",
        0,
        0,
        None,
        false,
        false,
        true,
        false,
        false,
        handle_cr,
    );
}

fn handle_cr(
    ctx: &mut FunctionContext,
    _args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    // Only parse optional [size] if next token is directly "[" (no space)
    let size = if ctx.parser.gullet.future().text == "[" {
        ctx.parser.parse_size_group(true)?
    } else {
        None
    };

    let size = size.and_then(|node| {
        if let ParseNode::Size { value, .. } = node {
            Some(value)
        } else {
            None
        }
    });

    let new_line = true;

    Ok(ParseNode::Cr {
        mode: ctx.parser.mode,
        new_line,
        size,
        loc: None,
    })
}
