use std::collections::HashMap;

use crate::error::ParseResult;
use crate::functions::{define_function_full, ArgType, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    // \label{name}
    define_function_full(
        map,
        &["\\label"],
        "label",
        1,
        0,
        Some(vec![ArgType::Text]),
        true,  // allowed_in_argument
        true,  // allowed_in_text
        true,  // allowed_in_math
        false, // infix
        false, // primitive
        handle_label,
    );

    // \ref{name}
    define_function_full(
        map,
        &["\\ref"],
        "ref",
        1,
        0,
        Some(vec![ArgType::Text]),
        true,  // allowed_in_argument
        true,  // allowed_in_text
        true,  // allowed_in_math
        false, // infix
        false, // primitive
        handle_ref,
    );

    // \eqref{name}
    define_function_full(
        map,
        &["\\eqref"],
        "eqref",
        1,
        0,
        Some(vec![ArgType::Text]),
        true,  // allowed_in_argument
        true,  // allowed_in_text
        true,  // allowed_in_math
        false, // infix
        false, // primitive
        handle_eqref,
    );

    // \notag (suppress equation number on this row)
    define_function_full(
        map,
        &["\\notag", "\\nonumber"],
        "notag",
        0,
        0,
        None,
        true,  // allowed_in_argument
        true,  // allowed_in_text
        true,  // allowed_in_math
        false, // infix
        true,  // primitive — must not be consumed by \def etc.
        handle_notag,
    );
}

fn handle_label(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let label_text = extract_text_arg(&args, 0);
    Ok(ParseNode::Label {
        mode: ctx.parser.mode,
        label: label_text,
        loc: None,
    })
}

fn handle_ref(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let label_text = extract_text_arg(&args, 0);
    Ok(ParseNode::Ref {
        mode: ctx.parser.mode,
        label: label_text,
        loc: None,
    })
}

fn handle_eqref(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let label_text = extract_text_arg(&args, 0);
    Ok(ParseNode::EqRef {
        mode: ctx.parser.mode,
        label: label_text,
        loc: None,
    })
}

fn handle_notag(
    _ctx: &mut FunctionContext,
    _args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    Ok(ParseNode::NoTag {
        mode: _ctx.parser.mode,
        loc: None,
    })
}

/// Extract a plain text string from the first argument, which is expected to be
/// an OrdGroup containing TextOrd / MathOrd nodes.
fn extract_text_arg(args: &[ParseNode], index: usize) -> String {
    let arg = match args.get(index) {
        Some(a) => a,
        None => return String::new(),
    };
    let body = match arg {
        ParseNode::OrdGroup { body, .. } => body,
        other => return other.symbol_text().unwrap_or("").to_string(),
    };
    let mut s = String::new();
    for node in body {
        if let Some(text) = node.symbol_text() {
            s.push_str(text);
        }
    }
    s
}
