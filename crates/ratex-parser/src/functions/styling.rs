use std::collections::HashMap;

use crate::error::ParseResult;
use crate::functions::{define_function_full, FunctionContext, FunctionSpec};
use crate::parse_node::{ParseNode, StyleStr};

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &[
            "\\displaystyle",
            "\\textstyle",
            "\\scriptstyle",
            "\\scriptscriptstyle",
        ],
        "styling",
        0,
        0,
        None,
        true, // allowed_in_argument
        true,
        true,
        false,
        false,
        handle_styling,
    );
}

fn handle_styling(
    ctx: &mut FunctionContext,
    _args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let break_on = ctx.break_on_token_text.clone();
    let body = ctx.parser.parse_expression(true, break_on.as_deref())?;

    let style = match ctx.func_name.as_str() {
        "\\displaystyle" => StyleStr::Display,
        "\\textstyle" => StyleStr::Text,
        "\\scriptstyle" => StyleStr::Script,
        "\\scriptscriptstyle" => StyleStr::Scriptscript,
        _ => StyleStr::Text,
    };

    Ok(ParseNode::Styling {
        mode: ctx.parser.mode,
        style,
        body,
        loc: None,
    })
}
