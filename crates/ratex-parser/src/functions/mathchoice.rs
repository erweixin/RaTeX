use std::collections::HashMap;

use crate::error::ParseResult;
use crate::functions::{define_function_full, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\mathchoice"],
        "mathchoice",
        4,
        0,
        None,
        false,
        false,
        true,
        false,
        true,
        handle_mathchoice,
    );
}

fn handle_mathchoice(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let mut args = args.into_iter();
    let display = ParseNode::ord_argument(args.next().unwrap());
    let text = ParseNode::ord_argument(args.next().unwrap());
    let script = ParseNode::ord_argument(args.next().unwrap());
    let scriptscript = ParseNode::ord_argument(args.next().unwrap());

    Ok(ParseNode::MathChoice {
        mode: ctx.parser.mode,
        display,
        text,
        script,
        scriptscript,
        loc: None,
    })
}
