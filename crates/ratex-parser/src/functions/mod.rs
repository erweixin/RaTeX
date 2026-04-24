pub mod genfrac;
pub mod sqrt;
pub mod op;
pub mod accent;
pub mod font;
pub mod color;
pub mod sizing;
pub mod delimsizing;
pub mod left_right;
pub mod spacing;
pub mod styling;
pub mod overline;
pub mod kern;
pub mod phantom;
pub mod text;
pub mod cr;
pub mod relax;
pub mod verb;
pub mod symbols_cmd;
pub mod environment;
pub mod mclass;
pub mod operatorname;
pub mod horiz_brace;
pub mod arrow;
pub mod enclose;
pub mod rule;
pub mod href;
pub mod hbox;
pub mod lap;
pub mod raisebox;
pub mod vcenter;
pub mod pmb;
pub mod mathchoice;
pub mod def;
pub mod htmlmathml;
pub mod char_cmd;
pub mod math;
pub mod tag;
pub mod label_ref;

use std::collections::HashMap;
use crate::error::ParseResult;
use crate::parse_node::{Mode, ParseNode};

use ratex_lexer::token::Token;

/// Context passed to function handlers.
pub struct FunctionContext<'a, 'b> {
    pub func_name: String,
    pub parser: &'a mut crate::parser::Parser<'b>,
    pub token: Option<Token>,
    pub break_on_token_text: Option<String>,
}

/// Handler function signature.
pub type FunctionHandler =
    fn(ctx: &mut FunctionContext, args: Vec<ParseNode>, opt_args: Vec<Option<ParseNode>>) -> ParseResult<ParseNode>;

/// Argument types for function parameters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgType {
    Color,
    Size,
    Url,
    Math,
    Text,
    HBox,
    Raw,
    Primitive,
    Original,
}

/// Specification for a registered function.
pub struct FunctionSpec {
    pub node_type: &'static str,
    pub num_args: usize,
    pub num_optional_args: usize,
    pub arg_types: Option<Vec<ArgType>>,
    pub allowed_in_argument: bool,
    pub allowed_in_text: bool,
    pub allowed_in_math: bool,
    pub infix: bool,
    pub primitive: bool,
    pub handler: FunctionHandler,
}

/// Get the global function registry.
pub static FUNCTIONS: std::sync::LazyLock<HashMap<&'static str, FunctionSpec>> =
    std::sync::LazyLock::new(|| {
        let mut map = HashMap::new();
        genfrac::register(&mut map);
        sqrt::register(&mut map);
        op::register(&mut map);
        accent::register(&mut map);
        font::register(&mut map);
        color::register(&mut map);
        sizing::register(&mut map);
        delimsizing::register(&mut map);
        left_right::register(&mut map);
        spacing::register(&mut map);
        styling::register(&mut map);
        overline::register(&mut map);
        kern::register(&mut map);
        phantom::register(&mut map);
        text::register(&mut map);
        cr::register(&mut map);
        relax::register(&mut map);
        verb::register(&mut map);
        symbols_cmd::register(&mut map);
        environment::register(&mut map);
        mclass::register(&mut map);
        operatorname::register(&mut map);
        horiz_brace::register(&mut map);
        arrow::register(&mut map);
        enclose::register(&mut map);
        rule::register(&mut map);
        href::register(&mut map);
        hbox::register(&mut map);
        lap::register(&mut map);
        raisebox::register(&mut map);
        vcenter::register(&mut map);
        pmb::register(&mut map);
        mathchoice::register(&mut map);
        def::register(&mut map);
        htmlmathml::register(&mut map);
        char_cmd::register(&mut map);
        math::register(&mut map);
        tag::register(&mut map);
        label_ref::register(&mut map);
        map
    });

/// Helper to define a function with common defaults.
pub fn define_function(
    map: &mut HashMap<&'static str, FunctionSpec>,
    names: &[&'static str],
    node_type: &'static str,
    num_args: usize,
    handler: FunctionHandler,
) {
    define_function_full(
        map, names, node_type, num_args, 0, None, false, false, true, false, false, handler,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn define_function_full(
    map: &mut HashMap<&'static str, FunctionSpec>,
    names: &[&'static str],
    node_type: &'static str,
    num_args: usize,
    num_optional_args: usize,
    arg_types: Option<Vec<ArgType>>,
    allowed_in_argument: bool,
    allowed_in_text: bool,
    allowed_in_math: bool,
    infix: bool,
    primitive: bool,
    handler: FunctionHandler,
) {
    for &name in names {
        map.insert(
            name,
            FunctionSpec {
                node_type,
                num_args,
                num_optional_args,
                arg_types: arg_types.clone(),
                allowed_in_argument,
                allowed_in_text,
                allowed_in_math,
                infix,
                primitive,
                handler,
            },
        );
    }
}

/// Check if a mode is compatible with a function spec.
pub fn check_mode_compatibility(
    func: &FunctionSpec,
    mode: Mode,
    func_name: &str,
    token: Option<&Token>,
) -> ParseResult<()> {
    if mode == Mode::Text && !func.allowed_in_text {
        return Err(crate::error::ParseError::new(
            format!("Can't use function '{}' in text mode", func_name),
            token,
        ));
    }
    if mode == Mode::Math && !func.allowed_in_math {
        return Err(crate::error::ParseError::new(
            format!("Can't use function '{}' in math mode", func_name),
            token,
        ));
    }
    Ok(())
}
