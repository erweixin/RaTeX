use std::collections::HashMap;

use crate::error::ParseResult;
use crate::functions::{define_function_full, ArgType, FunctionContext, FunctionSpec};
use crate::parse_node::{Measurement, ParseNode};

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\kern", "\\mkern", "\\hskip", "\\mskip"],
        "kern",
        1,
        0,
        Some(vec![ArgType::Size]),
        true, // allowed_in_argument
        true,
        true,
        false,
        false,
        handle_kern,
    );
}

fn handle_kern(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let dimension = if let ParseNode::Size { value, .. } = &args[0] {
        value.clone()
    } else {
        Measurement {
            number: 0.0,
            unit: "em".to_string(),
        }
    };

    Ok(ParseNode::Kern {
        mode: ctx.parser.mode,
        dimension,
        loc: None,
    })
}
