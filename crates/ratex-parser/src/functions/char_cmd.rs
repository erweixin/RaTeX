use std::collections::HashMap;

use crate::error::{ParseError, ParseResult};
use crate::functions::{define_function_full, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\@char"],
        "textord",
        1, 0, None,
        false, true, true, false, false,
        handle_at_char,
    );
}

fn handle_at_char(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let arg = args.into_iter().next().unwrap();
    let body = ParseNode::ord_argument(arg);

    let mut number_str = String::new();
    for node in &body {
        if let Some(text) = node.symbol_text() {
            number_str.push_str(text);
        }
    }

    let code: u32 = number_str
        .parse()
        .map_err(|_| ParseError::msg(format!("\\@char has non-numeric argument {}", number_str)))?;

    let text = char::from_u32(code)
        .ok_or_else(|| ParseError::msg(format!("\\@char with invalid code point {}", code)))?
        .to_string();

    // Match KaTeX `src/functions/char.ts`: `\@char` always yields `textord`, even when the
    // codepoint is also listed as `\\sum` / op-token in `symbols.js`. `\char"2211` must not
    // become a large `Size1` operator (that glyph is much wider than KaTeX’s textord path).
    Ok(ParseNode::TextOrd {
        mode: ctx.parser.mode,
        text,
        loc: None,
    })
}
