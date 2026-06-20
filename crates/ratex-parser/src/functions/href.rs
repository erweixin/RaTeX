use std::collections::HashMap;

use crate::error::{ParseError, ParseResult};
use crate::functions::{define_function_full, ArgType, FunctionContext, FunctionSpec};
use crate::parse_node::ParseNode;

pub fn register(map: &mut HashMap<&'static str, FunctionSpec>) {
    define_function_full(
        map,
        &["\\href"],
        "href",
        2,
        0,
        Some(vec![ArgType::Url, ArgType::Original]),
        false,
        true,
        true,
        false,
        false,
        handle_href,
    );

    define_function_full(
        map,
        &["\\url"],
        "href",
        1,
        0,
        Some(vec![ArgType::Url]),
        false,
        true,
        true,
        false,
        false,
        handle_url,
    );
}

fn handle_href(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let mut args = args.into_iter();
    let url_arg = args.next().unwrap();
    let body_arg = args.next().unwrap();

    let href = match url_arg {
        ParseNode::Url { url, .. } => url,
        _ => return Err(ParseError::msg("Expected URL")),
    };

    Ok(ParseNode::Href {
        mode: ctx.parser.mode,
        href,
        body: ParseNode::ord_argument(body_arg),
        loc: None,
    })
}

fn handle_url(
    ctx: &mut FunctionContext,
    args: Vec<ParseNode>,
    _opt_args: Vec<Option<ParseNode>>,
) -> ParseResult<ParseNode> {
    let url_arg = args.into_iter().next().unwrap();

    let url = match &url_arg {
        ParseNode::Url { url, .. } => url.clone(),
        _ => return Err(ParseError::msg("Expected URL")),
    };

    let chars: Vec<ParseNode> = url
        .chars()
        .map(|c| ParseNode::TextOrd {
            mode: crate::parse_node::Mode::Text,
            text: c.to_string(),
            loc: None,
        })
        .collect();

    let text_node = ParseNode::Text {
        mode: ctx.parser.mode,
        body: chars,
        font: Some("\\texttt".to_string()),
        loc: None,
    };

    Ok(ParseNode::Href {
        mode: ctx.parser.mode,
        href: url,
        body: vec![text_node],
        loc: None,
    })
}
