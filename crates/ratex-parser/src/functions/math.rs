use std::collections::HashMap;

use crate::error::{ParseError, ParseResult};
use crate::functions::{define_function_full, FunctionContext, FunctionSpec};
use crate::parse_node::{Mode, ParseNode, StyleStr};

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\(", "$"],
        "styling",
        0,
        0,
        None,
        false,
        true,
        false,
        false,
        false,
        handle_math_switch,
    );

    define_function_full(
        map,
        &["\\)", "\\]"],
        "text",
        0,
        0,
        None,
        false,
        true,
        false,
        false,
        false,
        handle_math_close,
    );
}

fn handle_math_switch(
    ctx: &mut FunctionContext,
    _args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let outer_mode = ctx.parser.mode;
    ctx.parser.switch_mode(Mode::Math);
    let close = if ctx.func_name == "\\(" { "\\)" } else { "$" };
    let body = ctx.parser.parse_expression(false, Some(close))?;
    ctx.parser.expect(close, true)?;
    ctx.parser.switch_mode(outer_mode);

    Ok(ParseNode::Styling {
        mode: ctx.parser.mode,
        style: StyleStr::Text,
        body,
        loc: None,
    })
}

fn handle_math_close(
    ctx: &mut FunctionContext,
    _args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    Err(ParseError::msg(format!("Mismatched {}", ctx.func_name)))
}
