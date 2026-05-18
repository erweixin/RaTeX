use std::collections::HashMap;

use crate::error::ParseResult;
use crate::functions::{define_function_full, FunctionContext, FunctionSpec};
use crate::parse_node::{AtomFamily, ParseNode};

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\fCenter"],
        "internal",
        0,
        0,
        None,
        true,
        true,
        true,
        false,
        false,
        handle_fcenter,
    );
}

fn handle_fcenter(
    ctx: &mut FunctionContext,
    _args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    // bussproofs defaults \fCenter to \Rightarrow (U+21D2 ⇒), separating the
    // left and right sides of a sequent (e.g. A \fCenter B → "A ⇒ B").
    Ok(ParseNode::Atom {
        mode: ctx.parser.mode,
        family: AtomFamily::Rel,
        text: "\\Rightarrow".to_string(),
        loc: None,
    })
}
