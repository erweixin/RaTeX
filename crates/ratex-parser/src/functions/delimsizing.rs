use std::collections::HashMap;

use crate::error::{ParseError, ParseResult};
use crate::functions::{define_function_full, ArgType, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

struct DelimInfo {
    mclass: &'static str,
    size: u8,
}

fn delim_info(name: &str) -> Option<DelimInfo> {
    match name {
        "\\bigl" => Some(DelimInfo {
            mclass: "mopen",
            size: 1,
        }),
        "\\Bigl" => Some(DelimInfo {
            mclass: "mopen",
            size: 2,
        }),
        "\\biggl" => Some(DelimInfo {
            mclass: "mopen",
            size: 3,
        }),
        "\\Biggl" => Some(DelimInfo {
            mclass: "mopen",
            size: 4,
        }),
        "\\bigr" => Some(DelimInfo {
            mclass: "mclose",
            size: 1,
        }),
        "\\Bigr" => Some(DelimInfo {
            mclass: "mclose",
            size: 2,
        }),
        "\\biggr" => Some(DelimInfo {
            mclass: "mclose",
            size: 3,
        }),
        "\\Biggr" => Some(DelimInfo {
            mclass: "mclose",
            size: 4,
        }),
        "\\bigm" => Some(DelimInfo {
            mclass: "mrel",
            size: 1,
        }),
        "\\Bigm" => Some(DelimInfo {
            mclass: "mrel",
            size: 2,
        }),
        "\\biggm" => Some(DelimInfo {
            mclass: "mrel",
            size: 3,
        }),
        "\\Biggm" => Some(DelimInfo {
            mclass: "mrel",
            size: 4,
        }),
        "\\big" => Some(DelimInfo {
            mclass: "mord",
            size: 1,
        }),
        "\\Big" => Some(DelimInfo {
            mclass: "mord",
            size: 2,
        }),
        "\\bigg" => Some(DelimInfo {
            mclass: "mord",
            size: 3,
        }),
        "\\Bigg" => Some(DelimInfo {
            mclass: "mord",
            size: 4,
        }),
        _ => None,
    }
}

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &[
            "\\bigl", "\\Bigl", "\\biggl", "\\Biggl", "\\bigr", "\\Bigr", "\\biggr", "\\Biggr",
            "\\bigm", "\\Bigm", "\\biggm", "\\Biggm", "\\big", "\\Big", "\\bigg", "\\Bigg",
        ],
        "delimsizing",
        1,
        0,
        Some(vec![ArgType::Primitive]),
        false,
        false,
        true,
        false,
        false,
        handle_delimsizing,
    );
}

fn handle_delimsizing(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let delim = check_delimiter(&args[0], &ctx.func_name)?;
    let info = delim_info(&ctx.func_name).unwrap();

    Ok(ParseNode::DelimSizing {
        mode: ctx.parser.mode,
        size: info.size,
        mclass: info.mclass.to_string(),
        delim,
        loc: None,
    })
}

fn check_delimiter(node: &ParseNode, func_name: &str) -> ParseResult<String> {
    if let Some(text) = node.symbol_text() {
        if is_valid_delimiter(text) {
            return Ok(text.to_string());
        }
        return Err(ParseError::msg(format!(
            "Invalid delimiter '{}' after '{}'",
            text, func_name
        )));
    }
    Err(ParseError::msg(format!(
        "Invalid delimiter type '{}' after '{}'",
        node.type_name(),
        func_name
    )))
}

fn is_valid_delimiter(text: &str) -> bool {
    matches!(
        text,
        "(" | "\\lparen"
            | ")"
            | "\\rparen"
            | "["
            | "\\lbrack"
            | "]"
            | "\\rbrack"
            | "\\{"
            | "\\lbrace"
            | "\\}"
            | "\\rbrace"
            | "\\lfloor"
            | "\\rfloor"
            | "\\lceil"
            | "\\rceil"
            | "<"
            | ">"
            | "\\langle"
            | "\\rangle"
            | "\\lt"
            | "\\gt"
            | "\\lvert"
            | "\\rvert"
            | "\\lVert"
            | "\\rVert"
            | "\\lgroup"
            | "\\rgroup"
            | "\\lmoustache"
            | "\\rmoustache"
            | "/"
            | "\\backslash"
            | "|"
            | "\\vert"
            | "\\|"
            | "\\Vert"
            | "\\uparrow"
            | "\\Uparrow"
            | "\\downarrow"
            | "\\Downarrow"
            | "\\updownarrow"
            | "\\Updownarrow"
            | "."
    )
}
