use std::collections::HashMap;

use crate::error::ParseResult;
use crate::functions::{define_function_full, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\phantom"],
        "phantom",
        1,
        0,
        None,
        true,
        true,
        true,
        false,
        false,
        handle_phantom,
    );

    define_function_full(
        map,
        &["\\vphantom"],
        "vphantom",
        1,
        0,
        None,
        true,
        true,
        true,
        false,
        false,
        handle_vphantom,
    );

    // \hphantom is defined as a macro: \smash{\phantom{#1}}
    // (registered in macro_expander builtins)

    define_function_full(
        map,
        &["\\smash"],
        "smash",
        1,
        1,
        None,
        true,
        true,
        true,
        false,
        false,
        handle_smash,
    );
}

fn handle_phantom(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let body = ParseNode::ord_argument(args.into_iter().next().unwrap());
    Ok(ParseNode::Phantom {
        mode: ctx.parser.mode,
        body,
        loc: None,
    })
}

fn handle_vphantom(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    Ok(ParseNode::VPhantom {
        mode: ctx.parser.mode,
        body: Box::new(args.into_iter().next().unwrap()),
        loc: None,
    })
}

fn handle_smash(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let mut smash_height = false;
    let mut smash_depth = false;

    if let Some(Some(ParseNode::OrdGroup { body, .. })) = opt_args.first() {
        for node in body {
            let text = node.symbol_text().unwrap_or("");
            match text {
                "t" => smash_height = true,
                "b" => smash_depth = true,
                _ => {
                    smash_height = false;
                    smash_depth = false;
                    break;
                }
            }
        }
    } else {
        smash_height = true;
        smash_depth = true;
    }

    Ok(ParseNode::Smash {
        mode: ctx.parser.mode,
        body: Box::new(args.into_iter().next().unwrap()),
        smash_height,
        smash_depth,
        loc: None,
    })
}
