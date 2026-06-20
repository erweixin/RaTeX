use std::collections::HashMap;

use crate::error::ParseResult;
use crate::functions::{define_function_full, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\pmb"],
        "pmb",
        1,
        0,
        None,
        false,
        true,
        true,
        false,
        false,
        handle_pmb,
    );
}

fn binrel_class(arg: &ParseNode) -> String {
    let atom = if let ParseNode::OrdGroup { body, .. } = arg {
        if !body.is_empty() {
            &body[0]
        } else {
            arg
        }
    } else {
        arg
    };
    if let ParseNode::Atom { family, .. } = atom {
        match family {
            crate::parse_node::AtomFamily::Bin => return "mbin".to_string(),
            crate::parse_node::AtomFamily::Rel => return "mrel".to_string(),
            _ => {}
        }
    }
    "mord".to_string()
}

fn handle_pmb(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let body = args.into_iter().next().unwrap();

    Ok(ParseNode::Pmb {
        mode: ctx.parser.mode,
        mclass: binrel_class(&body),
        body: ParseNode::ord_argument(body),
        loc: None,
    })
}
