use ratex_lexer::token::{SourceLocation, Token};
use unicode_normalization::UnicodeNormalization;

use crate::error::{ParseError, ParseResult};
use crate::functions::{self, ArgType, FunctionContext, FUNCTIONS};
use crate::macro_expander::{MacroExpander, IMPLICIT_COMMANDS};
use crate::parse_node::{AtomFamily, Mode, ParseNode};

/// End-of-expression tokens.
static END_OF_EXPRESSION: &[&str] = &["}", "\\endgroup", "\\end", "\\right", "&"];

/// The LaTeX parser. Converts a token stream into a ParseNode AST.
///
/// Follows KaTeX's Parser.ts closely:
/// - `parse()` → parse full expression
/// - `parseExpression()` → parse a list of atoms
/// - `parseAtom()` → parse one atom with optional super/subscripts
/// - `parseGroup()` → parse a group (braced or single token)
/// - `parseFunction()` → parse a function call with arguments
/// - `parseSymbol()` → parse a single symbol
pub struct Parser<'a> {
    pub mode: Mode,
    pub gullet: MacroExpander<'a>,
    pub leftright_depth: i32,
    next_token: Option<Token>,
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            mode: Mode::Math,
            gullet: MacroExpander::new(input, Mode::Math),
            leftright_depth: 0,
            next_token: None,
        }
    }

    // ── Token management ────────────────────────────────────────────────

    /// Return the current lookahead token (fetching from gullet if needed).
    pub fn fetch(&mut self) -> ParseResult<Token> {
        if self.next_token.is_none() {
            self.next_token = Some(self.gullet.expand_next_token()?);
        }
        Ok(self.next_token.clone().unwrap())
    }

    /// Discard the current lookahead token.
    pub fn consume(&mut self) {
        self.next_token = None;
    }

    /// Expect the next token to have the given text, consuming it.
    pub fn expect(&mut self, text: &str, do_consume: bool) -> ParseResult<()> {
        let tok = self.fetch()?;
        if tok.text != text {
            return Err(ParseError::new(
                format!("Expected '{}', got '{}'", text, tok.text),
                Some(&tok),
            ));
        }
        if do_consume {
            self.consume();
        }
        Ok(())
    }

    /// Consume spaces in math mode.
    pub fn consume_spaces(&mut self) -> ParseResult<()> {
        loop {
            let tok = self.fetch()?;
            if tok.text == " " {
                self.consume();
            } else {
                break;
            }
        }
        Ok(())
    }

    /// Switch between "math" and "text" modes.
    pub fn switch_mode(&mut self, new_mode: Mode) {
        self.mode = new_mode;
        self.gullet.switch_mode(new_mode);
    }

    // ── Main parse entry ────────────────────────────────────────────────

    /// Parse the entire input and return the AST.
    pub fn parse(&mut self) -> ParseResult<Vec<ParseNode>> {
        self.gullet.begin_group();

        let result = self.parse_expression(false, None);

        match result {
            Ok(parse) => {
                self.expect("EOF", true)?;
                self.gullet.end_group();
                Ok(parse)
            }
            Err(e) => {
                self.gullet.end_groups();
                Err(e)
            }
        }
    }

    // ── Expression parsing ──────────────────────────────────────────────

    /// Parse an expression: a list of atoms.
    pub fn parse_expression(
        &mut self,
        break_on_infix: bool,
        break_on_token_text: Option<&str>,
    ) -> ParseResult<Vec<ParseNode>> {
        let mut body = Vec::new();

        loop {
            if self.mode == Mode::Math {
                self.consume_spaces()?;
            }

            let lex = self.fetch()?;

            if END_OF_EXPRESSION.contains(&lex.text.as_str()) {
                break;
            }
            if let Some(break_text) = break_on_token_text {
                if lex.text == break_text {
                    break;
                }
            }
            if break_on_infix {
                if let Some(func) = FUNCTIONS.get(lex.text.as_str()) {
                    if func.infix {
                        break;
                    }
                }
            }

            let atom = self.parse_atom(break_on_token_text)?;

            match atom {
                None => break,
                Some(node) if node.type_name() == "internal" => continue,
                Some(node) => body.push(node),
            }
        }

        if self.mode == Mode::Text {
            self.form_ligatures(&mut body);
        }

        self.handle_infix_nodes(body)
    }

    /// Rewrite infix operators (e.g. \over → \frac).
    fn handle_infix_nodes(&mut self, body: Vec<ParseNode>) -> ParseResult<Vec<ParseNode>> {
        let mut over_index: Option<usize> = None;
        let mut func_name: Option<String> = None;

        for (i, node) in body.iter().enumerate() {
            if let ParseNode::Infix { replace_with, .. } = node {
                if over_index.is_some() {
                    return Err(ParseError::msg("only one infix operator per group"));
                }
                over_index = Some(i);
                func_name = Some(replace_with.clone());
            }
        }

        if let (Some(idx), Some(fname)) = (over_index, func_name) {
            let numer_body: Vec<ParseNode> = body[..idx].to_vec();
            let denom_body: Vec<ParseNode> = body[idx + 1..].to_vec();

            let numer = if numer_body.len() == 1 {
                if let ParseNode::OrdGroup { .. } = &numer_body[0] {
                    numer_body.into_iter().next().unwrap()
                } else {
                    ParseNode::OrdGroup {
                        mode: self.mode,
                        body: numer_body,
                        semisimple: None,
                        loc: None,
                    }
                }
            } else {
                ParseNode::OrdGroup {
                    mode: self.mode,
                    body: numer_body,
                    semisimple: None,
                    loc: None,
                }
            };

            let denom = if denom_body.len() == 1 {
                if let ParseNode::OrdGroup { .. } = &denom_body[0] {
                    denom_body.into_iter().next().unwrap()
                } else {
                    ParseNode::OrdGroup {
                        mode: self.mode,
                        body: denom_body,
                        semisimple: None,
                        loc: None,
                    }
                }
            } else {
                ParseNode::OrdGroup {
                    mode: self.mode,
                    body: denom_body,
                    semisimple: None,
                    loc: None,
                }
            };

            let node = if fname == "\\\\abovefrac" {
                // \above passes the infix node (with bar size) as the middle argument
                let infix_node = body[idx].clone();
                self.call_function(&fname, vec![numer, infix_node, denom], vec![], None, None)?
            } else {
                self.call_function(&fname, vec![numer, denom], vec![], None, None)?
            };
            Ok(vec![node])
        } else {
            Ok(body)
        }
    }

    /// Form ligatures in text mode (e.g. -- → –, --- → —).
    fn form_ligatures(&self, group: &mut Vec<ParseNode>) {
        let mut i = 0;
        while i + 1 < group.len() {
            let a_text = group[i].symbol_text().map(|s| s.to_string());
            let b_text = group[i + 1].symbol_text().map(|s| s.to_string());

            if let (Some(a), Some(b)) = (a_text, b_text) {
                if group[i].type_name() == "textord" && group[i + 1].type_name() == "textord" {
                    if a == "-" && b == "-" {
                        if i + 2 < group.len() {
                            if let Some(c) = group[i + 2].symbol_text() {
                                if c == "-" && group[i + 2].type_name() == "textord" {
                                    group[i] = ParseNode::TextOrd {
                                        mode: Mode::Text,
                                        text: "---".to_string(),
                                        loc: None,
                                    };
                                    group.remove(i + 2);
                                    group.remove(i + 1);
                                    continue;
                                }
                            }
                        }
                        group[i] = ParseNode::TextOrd {
                            mode: Mode::Text,
                            text: "--".to_string(),
                            loc: None,
                        };
                        group.remove(i + 1);
                        continue;
                    }
                    if (a == "'" || a == "`") && b == a {
                        group[i] = ParseNode::TextOrd {
                            mode: Mode::Text,
                            text: format!("{}{}", a, a),
                            loc: None,
                        };
                        group.remove(i + 1);
                        continue;
                    }
                }
            }
            i += 1;
        }
    }

    // ── Atom parsing ────────────────────────────────────────────────────

    /// Parse a single atom with optional super/subscripts.
    pub fn parse_atom(
        &mut self,
        break_on_token_text: Option<&str>,
    ) -> ParseResult<Option<ParseNode>> {
        let mut base = self.parse_group("atom", break_on_token_text)?;

        if let Some(ref b) = base {
            if b.type_name() == "internal" {
                return Ok(base);
            }
        }

        if self.mode == Mode::Text {
            return Ok(base);
        }

        let mut superscript: Option<ParseNode> = None;
        let mut subscript: Option<ParseNode> = None;

        loop {
            self.consume_spaces()?;
            let lex = self.fetch()?;

            if lex.text == "\\limits" || lex.text == "\\nolimits" {
                let is_limits = lex.text == "\\limits";
                self.consume();
                if let Some(
                    ParseNode::Op { limits, .. }
                    | ParseNode::OperatorName { limits, .. },
                ) = base.as_mut()
                {
                    *limits = is_limits;
                }
            } else if lex.text == "^" {
                if superscript.is_some() {
                    return Err(ParseError::new("Double superscript", Some(&lex)));
                }
                superscript = Some(self.handle_sup_subscript("superscript")?);
            } else if lex.text == "_" {
                if subscript.is_some() {
                    return Err(ParseError::new("Double subscript", Some(&lex)));
                }
                subscript = Some(self.handle_sup_subscript("subscript")?);
            } else if lex.text == "'" {
                if superscript.is_some() {
                    return Err(ParseError::new("Double superscript", Some(&lex)));
                }
                let prime = ParseNode::TextOrd {
                    mode: self.mode,
                    text: "\\prime".to_string(),
                    loc: None,
                };
                let mut primes = vec![prime.clone()];
                self.consume();
                while self.fetch()?.text == "'" {
                    primes.push(prime.clone());
                    self.consume();
                }
                if self.fetch()?.text == "^" {
                    primes.push(self.handle_sup_subscript("superscript")?);
                }
                superscript = Some(ParseNode::OrdGroup {
                    mode: self.mode,
                    body: primes,
                    semisimple: None,
                    loc: None,
                });
            } else if let Some((mapped, is_sub)) = lex
                .text
                .chars()
                .next()
                .and_then(crate::unicode_sup_sub::unicode_sub_sup)
            {
                if is_sub && subscript.is_some() {
                    return Err(ParseError::new("Double subscript", Some(&lex)));
                }
                if !is_sub && superscript.is_some() {
                    return Err(ParseError::new("Double superscript", Some(&lex)));
                }
                // Collect consecutive Unicode sup/sub chars of the same kind
                let mut subsup_tokens = vec![Token::new(mapped, 0, 0)];
                self.consume();
                loop {
                    let tok = self.fetch()?;
                    match tok
                        .text
                        .chars()
                        .next()
                        .and_then(crate::unicode_sup_sub::unicode_sub_sup)
                    {
                        Some((m, sub)) if sub == is_sub => {
                            subsup_tokens.insert(0, Token::new(m, 0, 0));
                            self.consume();
                        }
                        _ => break,
                    }
                }
                let body = self.subparse(subsup_tokens)?;
                let group = ParseNode::OrdGroup {
                    mode: Mode::Math,
                    body,
                    semisimple: None,
                    loc: None,
                };
                if is_sub {
                    subscript = Some(group);
                } else {
                    superscript = Some(group);
                }
            } else {
                break;
            }
        }

        if superscript.is_some() || subscript.is_some() {
            Ok(Some(ParseNode::SupSub {
                mode: self.mode,
                base: base.map(Box::new),
                sup: superscript.map(Box::new),
                sub: subscript.map(Box::new),
                loc: None,
            }))
        } else {
            Ok(base)
        }
    }

    /// Handle a subscript or superscript.
    fn handle_sup_subscript(&mut self, name: &str) -> ParseResult<ParseNode> {
        let symbol_token = self.fetch()?;
        self.consume();
        self.consume_spaces()?;

        let group = self.parse_group(name, None)?;
        match group {
            Some(g) if g.type_name() != "internal" => Ok(g),
            Some(_) => {
                // Skip internal nodes, try again
                let g2 = self.parse_group(name, None)?;
                g2.ok_or_else(|| {
                    ParseError::new(
                        format!("Expected group after '{}'", symbol_token.text),
                        Some(&symbol_token),
                    )
                })
            }
            None => Err(ParseError::new(
                format!("Expected group after '{}'", symbol_token.text),
                Some(&symbol_token),
            )),
        }
    }

    // ── Group parsing ───────────────────────────────────────────────────

    /// Parse a group: braced expression, function call, or single symbol.
    pub fn parse_group(
        &mut self,
        name: &str,
        break_on_token_text: Option<&str>,
    ) -> ParseResult<Option<ParseNode>> {
        let first_token = self.fetch()?;
        let text = first_token.text.clone();

        if text == "{" || text == "\\begingroup" {
            self.consume();
            let group_end = if text == "{" { "}" } else { "\\endgroup" };

            self.gullet.begin_group();
            let expression = self.parse_expression(false, Some(group_end))?;
            let last_token = self.fetch()?;
            self.expect(group_end, true)?;
            self.gullet.end_group();

            let loc = Some(SourceLocation::range(&first_token.loc, &last_token.loc));
            let semisimple = if text == "\\begingroup" {
                Some(true)
            } else {
                None
            };

            Ok(Some(ParseNode::OrdGroup {
                mode: self.mode,
                body: expression,
                semisimple,
                loc,
            }))
        } else {
            let result = self
                .parse_function(break_on_token_text, Some(name))?
                .or_else(|| self.parse_symbol_inner().ok().flatten());

            if result.is_none()
                && text.starts_with('\\')
                && !IMPLICIT_COMMANDS.contains(&text.as_str())
            {
                return Err(ParseError::new(
                    format!("Undefined control sequence: {}", text),
                    Some(&first_token),
                ));
            }

            Ok(result)
        }
    }

    // ── Function parsing ────────────────────────────────────────────────

    /// Try to parse a function call. Returns None if not a function.
    pub fn parse_function(
        &mut self,
        break_on_token_text: Option<&str>,
        name: Option<&str>,
    ) -> ParseResult<Option<ParseNode>> {
        let token = self.fetch()?;
        let func = token.text.clone();

        let func_data = match FUNCTIONS.get(func.as_str()) {
            Some(f) => f,
            None => return Ok(None),
        };

        self.consume();

        if let Some(n) = name {
            if n != "atom" && !func_data.allowed_in_argument {
                return Err(ParseError::new(
                    format!("Got function '{}' with no arguments as {}", func, n),
                    Some(&token),
                ));
            }
        }

        functions::check_mode_compatibility(func_data, self.mode, &func, Some(&token))?;

        let (args, opt_args) = self.parse_arguments(&func, func_data)?;

        self.call_function(
            &func,
            args,
            opt_args,
            Some(token),
            break_on_token_text.map(|s| s.to_string()).as_deref(),
        )
        .map(Some)
    }

    /// Call a function handler.
    pub fn call_function(
        &mut self,
        name: &str,
        args: Vec<ParseNode>,
        opt_args: Vec<Option<ParseNode>>,
        token: Option<Token>,
        break_on_token_text: Option<&str>,
    ) -> ParseResult<ParseNode> {
        let func = FUNCTIONS.get(name).ok_or_else(|| {
            ParseError::msg(format!("No function handler for {}", name))
        })?;

        let mut ctx = FunctionContext {
            func_name: name.to_string(),
            parser: self,
            token: token.clone(),
            break_on_token_text: break_on_token_text.map(|s| s.to_string()),
        };

        (func.handler)(&mut ctx, args, opt_args)
    }

    /// Parse the arguments for a function.
    pub fn parse_arguments(
        &mut self,
        func: &str,
        func_data: &functions::FunctionSpec,
    ) -> ParseResult<(Vec<ParseNode>, Vec<Option<ParseNode>>)> {
        let total_args = func_data.num_args + func_data.num_optional_args;
        if total_args == 0 {
            return Ok((Vec::new(), Vec::new()));
        }

        let mut args = Vec::new();
        let mut opt_args = Vec::new();

        for i in 0..total_args {
            let arg_type = func_data
                .arg_types
                .as_ref()
                .and_then(|types| types.get(i).copied());
            let is_optional = i < func_data.num_optional_args;

            let effective_type = if (func_data.primitive && arg_type.is_none())
                || (func_data.node_type == "sqrt" && i == 1
                    && opt_args.first().is_some_and(|o: &Option<ParseNode>| o.is_none()))
            {
                Some(ArgType::Primitive)
            } else {
                arg_type
            };

            let arg = self.parse_group_of_type(
                &format!("argument to '{}'", func),
                effective_type,
                is_optional,
            )?;

            if is_optional {
                opt_args.push(arg);
            } else if let Some(a) = arg {
                args.push(a);
            } else {
                return Err(ParseError::msg("Null argument, please report this as a bug"));
            }
        }

        Ok((args, opt_args))
    }

    /// Parse a group with a specific type.
    fn parse_group_of_type(
        &mut self,
        name: &str,
        arg_type: Option<ArgType>,
        optional: bool,
    ) -> ParseResult<Option<ParseNode>> {
        match arg_type {
            Some(ArgType::Color) => self.parse_color_group(optional),
            Some(ArgType::Size) => self.parse_size_group(optional),
            Some(ArgType::Primitive) => {
                if optional {
                    return Err(ParseError::msg("A primitive argument cannot be optional"));
                }
                let group = self.parse_group(name, None)?;
                match group {
                    Some(g) => Ok(Some(g)),
                    None => Err(ParseError::new(
                        format!("Expected group as {}", name),
                        None,
                    )),
                }
            }
            Some(ArgType::Math) | Some(ArgType::Text) => {
                let mode = match arg_type {
                    Some(ArgType::Math) => Some(Mode::Math),
                    Some(ArgType::Text) => Some(Mode::Text),
                    _ => None,
                };
                self.parse_argument_group(optional, mode)
            }
            Some(ArgType::HBox) => {
                let group = self.parse_argument_group(optional, Some(Mode::Text))?;
                match group {
                    Some(g) => Ok(Some(ParseNode::Styling {
                        mode: g.mode(),
                        style: crate::parse_node::StyleStr::Text,
                        body: vec![g],
                        loc: None,
                    })),
                    None => Ok(None),
                }
            }
            Some(ArgType::Raw) => {
                let token = self.parse_string_group("raw", optional)?;
                match token {
                    Some(t) => Ok(Some(ParseNode::Raw {
                        mode: Mode::Text,
                        string: t.text,
                        loc: None,
                    })),
                    None => Ok(None),
                }
            }
            Some(ArgType::Url) => self.parse_url_group(optional),
            None | Some(ArgType::Original) => self.parse_argument_group(optional, None),
        }
    }

    /// Parse a color group.
    fn parse_color_group(&mut self, optional: bool) -> ParseResult<Option<ParseNode>> {
        let res = self.parse_string_group("color", optional)?;
        match res {
            None => Ok(None),
            Some(token) => {
                let text = token.text.trim().to_string();
                let re = regex_lite::Regex::new(
                    r"^(#[a-fA-F0-9]{3,4}|#[a-fA-F0-9]{6}|#[a-fA-F0-9]{8}|[a-fA-F0-9]{6}|[a-zA-Z]+|\d+(\.\d+)?(,\d+(\.\d+)?)*)$",
                )
                .unwrap();

                if !re.is_match(&text) {
                    return Err(ParseError::new(
                        format!("Invalid color: '{}'", text),
                        Some(&token),
                    ));
                }
                let mut color = text;
                if regex_lite::Regex::new(r"^[0-9a-fA-F]{6}$")
                    .unwrap()
                    .is_match(&color)
                {
                    color = format!("#{}", color);
                }

                Ok(Some(ParseNode::ColorToken {
                    mode: self.mode,
                    color,
                    loc: None,
                }))
            }
        }
    }

    /// Parse a size group (e.g., "3pt", "1em").
    pub fn parse_size_group(&mut self, optional: bool) -> ParseResult<Option<ParseNode>> {
        let mut is_blank = false;

        self.gullet.consume_spaces();
        let res = if !optional && self.gullet.future().text != "{" {
            Some(self.parse_regex_group(
                &regex_lite::Regex::new(r"^[-+]? *(?:$|\d+|\d+\.\d*|\.\d*) *[a-z]{0,2} *$")
                    .unwrap(),
                "size",
            )?)
        } else {
            self.parse_string_group("size", optional)?
        };

        let res = match res {
            Some(r) => r,
            None => return Ok(None),
        };

        let mut text = res.text.clone();
        if !optional && text.is_empty() {
            text = "0pt".to_string();
            is_blank = true;
        }

        let size_re =
            regex_lite::Regex::new(r"([-+]?) *(\d+(?:\.\d*)?|\.\d+) *([a-z]{2})").unwrap();
        let m = size_re.captures(&text).ok_or_else(|| {
            ParseError::new(format!("Invalid size: '{}'", text), Some(&res))
        })?;

        let sign = m.get(1).map_or("", |m| m.as_str());
        let magnitude = m.get(2).map_or("", |m| m.as_str());
        let unit = m.get(3).map_or("", |m| m.as_str());

        let number: f64 = format!("{}{}", sign, magnitude).parse().unwrap_or(0.0);

        if !is_valid_unit(unit) {
            return Err(ParseError::new(
                format!("Invalid unit: '{}'", unit),
                Some(&res),
            ));
        }

        Ok(Some(ParseNode::Size {
            mode: self.mode,
            value: crate::parse_node::Measurement {
                number,
                unit: unit.to_string(),
            },
            is_blank,
            loc: None,
        }))
    }

    /// Parse a URL group.
    /// Temporarily disables `%` as comment character to allow `%20` etc. in URLs.
    fn parse_url_group(&mut self, optional: bool) -> ParseResult<Option<ParseNode>> {
        self.gullet.lexer.set_catcode('%', 13);
        self.gullet.lexer.set_catcode('~', 12);
        let res = self.parse_string_group("url", optional);
        self.gullet.lexer.set_catcode('%', 14);
        self.gullet.lexer.set_catcode('~', 13);
        let res = res?;
        match res {
            None => Ok(None),
            Some(token) => {
                let url = token.text;
                Ok(Some(ParseNode::Url {
                    mode: self.mode,
                    url,
                    loc: None,
                }))
            }
        }
    }

    /// Parse a string group (brace-enclosed string).
    fn parse_string_group(
        &mut self,
        _mode_name: &str,
        optional: bool,
    ) -> ParseResult<Option<Token>> {
        let arg_token = self.gullet.scan_argument(optional)?;
        let arg_token = match arg_token {
            Some(t) => t,
            None => return Ok(None),
        };

        let mut s = String::new();
        loop {
            let next = self.fetch()?;
            if next.text == "EOF" {
                break;
            }
            s.push_str(&next.text);
            self.consume();
        }
        self.consume(); // consume EOF

        let mut result = arg_token;
        result.text = s;
        Ok(Some(result))
    }

    /// Parse a regex-delimited group.
    fn parse_regex_group(
        &mut self,
        regex: &regex_lite::Regex,
        mode_name: &str,
    ) -> ParseResult<Token> {
        let first_token = self.fetch()?;
        let mut last_token = first_token.clone();
        let mut s = String::new();

        loop {
            let next = self.fetch()?;
            if next.text == "EOF" {
                break;
            }
            let candidate = format!("{}{}", s, next.text);
            if regex.is_match(&candidate) {
                last_token = next;
                s = candidate;
                self.consume();
            } else {
                break;
            }
        }

        if s.is_empty() {
            return Err(ParseError::new(
                format!("Invalid {}: '{}'", mode_name, first_token.text),
                Some(&first_token),
            ));
        }

        Ok(first_token.range(&last_token, s))
    }

    /// Parse an argument group (with optional mode switch).
    pub fn parse_argument_group(
        &mut self,
        optional: bool,
        mode: Option<Mode>,
    ) -> ParseResult<Option<ParseNode>> {
        let arg_token = self.gullet.scan_argument(optional)?;
        let arg_token = match arg_token {
            Some(t) => t,
            None => return Ok(None),
        };

        let outer_mode = self.mode;
        if let Some(m) = mode {
            self.switch_mode(m);
        }

        self.gullet.begin_group();
        let expression = self.parse_expression(false, Some("EOF"))?;
        self.expect("EOF", true)?;
        self.gullet.end_group();

        let result = ParseNode::OrdGroup {
            mode: self.mode,
            loc: Some(arg_token.loc.clone()),
            body: expression,
            semisimple: None,
        };

        if mode.is_some() {
            self.switch_mode(outer_mode);
        }

        Ok(Some(result))
    }

    // ── Symbol parsing ──────────────────────────────────────────────────

    /// Parse a single symbol (internal version that returns Result).
    fn parse_symbol_inner(&mut self) -> ParseResult<Option<ParseNode>> {
        let nucleus = self.fetch()?;
        let text = nucleus.text.clone();

        if let Some(stripped) = text.strip_prefix("\\verb") {
            self.consume();
            let arg = stripped.to_string();
            let star = arg.starts_with('*');
            let arg = if star { &arg[1..] } else { &arg };

            if arg.len() < 2 {
                return Err(ParseError::new("\\verb assertion failed", Some(&nucleus)));
            }
            let body = arg[1..arg.len() - 1].to_string();
            return Ok(Some(ParseNode::Verb {
                mode: Mode::Text,
                body,
                star,
                loc: Some(nucleus.loc.clone()),
            }));
        }

        let font_mode = match self.mode {
            Mode::Math => ratex_font::symbols::Mode::Math,
            Mode::Text => ratex_font::symbols::Mode::Text,
        };

        // ^ and _ are handled by parse_atom for sup/sub, not as symbol nodes
        if text == "^" || text == "_" {
            return Ok(None);
        }

        // Bare backslash (incomplete control sequence) → not a valid symbol
        if text == "\\" {
            return Ok(None);
        }

        if let Some(sym_info) = ratex_font::symbols::get_symbol(&text, font_mode) {
            let loc = Some(SourceLocation::range(&nucleus.loc, &nucleus.loc));
            let group = sym_info.group;

            let node = if group.is_atom() {
                let family = match group {
                    ratex_font::symbols::Group::Bin => AtomFamily::Bin,
                    ratex_font::symbols::Group::Close => AtomFamily::Close,
                    ratex_font::symbols::Group::Inner => AtomFamily::Inner,
                    ratex_font::symbols::Group::Open => AtomFamily::Open,
                    ratex_font::symbols::Group::Punct => AtomFamily::Punct,
                    ratex_font::symbols::Group::Rel => AtomFamily::Rel,
                    _ => unreachable!(),
                };
                ParseNode::Atom {
                    mode: self.mode,
                    family,
                    text: text.clone(),
                    loc,
                }
            } else {
                match group {
                    ratex_font::symbols::Group::MathOrd => ParseNode::MathOrd {
                        mode: self.mode,
                        text: text.clone(),
                        loc,
                    },
                    ratex_font::symbols::Group::TextOrd => ParseNode::TextOrd {
                        mode: self.mode,
                        text: text.clone(),
                        loc,
                    },
                    ratex_font::symbols::Group::OpToken => ParseNode::OpToken {
                        mode: self.mode,
                        text: text.clone(),
                        loc,
                    },
                    ratex_font::symbols::Group::AccentToken => ParseNode::AccentToken {
                        mode: self.mode,
                        text: text.clone(),
                        loc,
                    },
                    ratex_font::symbols::Group::Spacing => ParseNode::SpacingNode {
                        mode: self.mode,
                        text: text.clone(),
                        loc,
                    },
                    _ => ParseNode::MathOrd {
                        mode: self.mode,
                        text: text.clone(),
                        loc,
                    },
                }
            };

            self.consume();
            return Ok(Some(node));
        }

        // Unicode accented characters → decompose into accent nodes
        // Handles both precomposed (á U+00E1) and combining forms (a + U+0301)
        if let Some(node) = self.try_parse_unicode_accent(&text, &nucleus)? {
            self.consume();
            return Ok(Some(node));
        }

        // Non-ASCII characters without accent decomposition → treat as textord
        // KaTeX always uses mode="text" for these, regardless of current mode
        let first_char = text.chars().next();
        if let Some(ch) = first_char {
            if ch as u32 >= 0x80 {
                let node = ParseNode::TextOrd {
                    mode: Mode::Text,
                    text: text.clone(),
                    loc: Some(SourceLocation::range(&nucleus.loc, &nucleus.loc)),
                };
                self.consume();
                return Ok(Some(node));
            }
        }

        Ok(None)
    }

    /// Try to decompose a Unicode accented character into accent nodes.
    /// Returns None if no decomposition is available.
    /// Only decomposes Latin-script characters, matching KaTeX behavior.
    fn try_parse_unicode_accent(
        &self,
        text: &str,
        nucleus: &Token,
    ) -> ParseResult<Option<ParseNode>> {
        let nfd: String = text.nfd().collect();
        let chars: Vec<char> = nfd.chars().collect();

        if chars.len() < 2 {
            return Ok(None);
        }

        // Build from the base up through each combining mark
        let mut split_idx = chars.len() - 1;
        while split_idx > 0 && is_supported_combining_accent(chars[split_idx]) {
            split_idx -= 1;
        }

        // Verify ALL trailing chars are supported combining accents
        if split_idx == chars.len() - 1 {
            return Ok(None);
        }

        // Only decompose Latin-script base characters
        let base_char = chars[0];
        if !is_latin_base_char(base_char) {
            return Ok(None);
        }

        let loc = Some(SourceLocation::range(&nucleus.loc, &nucleus.loc));

        // Base: everything before the combining marks
        let mut base_str: String = chars[..split_idx + 1].iter().collect();

        // Accented i→ı and j→ȷ (dotless variants), matching KaTeX behavior
        if base_str.len() == 1 {
            match base_str.as_str() {
                "i" => base_str = "\u{0131}".to_string(), // ı
                "j" => base_str = "\u{0237}".to_string(), // ȷ
                _ => {}
            }
        }

        let font_mode = match self.mode {
            Mode::Math => ratex_font::symbols::Mode::Math,
            Mode::Text => ratex_font::symbols::Mode::Text,
        };

        let mut node = if base_str.chars().count() == 1 {
            let ch = base_str.chars().next().unwrap();
            if let Some(sym) = ratex_font::symbols::get_symbol(&base_str, font_mode) {
                match sym.group {
                    ratex_font::symbols::Group::TextOrd => ParseNode::TextOrd {
                        mode: self.mode,
                        text: base_str.clone(),
                        loc: loc.clone(),
                    },
                    _ => ParseNode::MathOrd {
                        mode: self.mode,
                        text: base_str.clone(),
                        loc: loc.clone(),
                    },
                }
            } else if (ch as u32) >= 0x80 {
                // Non-ASCII base chars always text mode (KaTeX compat)
                ParseNode::TextOrd {
                    mode: Mode::Text,
                    text: base_str.clone(),
                    loc: loc.clone(),
                }
            } else {
                ParseNode::MathOrd {
                    mode: self.mode,
                    text: base_str.clone(),
                    loc: loc.clone(),
                }
            }
        } else {
            return self.try_parse_unicode_accent(&base_str, nucleus).map(|opt| {
                opt.or_else(|| {
                    Some(ParseNode::TextOrd {
                        mode: Mode::Text,
                        text: base_str.clone(),
                        loc: loc.clone(),
                    })
                })
            });
        };

        // Wrap in accent nodes from innermost to outermost
        for &combining in &chars[split_idx + 1..] {
            let label = combining_to_accent_label(combining, self.mode);
            node = ParseNode::Accent {
                mode: self.mode,
                label,
                is_stretchy: Some(false),
                is_shifty: Some(true),
                base: Box::new(node),
                loc: loc.clone(),
            };
        }

        Ok(Some(node))
    }

    /// Parse a sub-expression from the given tokens.
    pub fn subparse(&mut self, tokens: Vec<Token>) -> ParseResult<Vec<ParseNode>> {
        let old_token = self.next_token.take();

        self.gullet
            .push_token(Token::new("}", 0, 0));
        self.gullet.push_tokens(tokens);
        let parse = self.parse_expression(false, None)?;
        self.expect("}", true)?;

        self.next_token = old_token;
        Ok(parse)
    }
}

fn is_latin_base_char(ch: char) -> bool {
    matches!(ch,
        'A'..='Z' | 'a'..='z'
        | '\u{0131}' // ı (dotless i)
        | '\u{0237}' // ȷ (dotless j)
        | '\u{00C6}' // Æ
        | '\u{00D0}' // Ð
        | '\u{00D8}' // Ø
        | '\u{00DE}' // Þ
        | '\u{00DF}' // ß
        | '\u{00E6}' // æ
        | '\u{00F0}' // ð
        | '\u{00F8}' // ø
        | '\u{00FE}' // þ
    )
}

fn is_supported_combining_accent(ch: char) -> bool {
    matches!(
        ch,
        '\u{0300}' | '\u{0301}' | '\u{0302}' | '\u{0303}' | '\u{0304}'
        | '\u{0306}' | '\u{0307}' | '\u{0308}' | '\u{030A}' | '\u{030B}' | '\u{030C}'
        | '\u{0327}'
    )
}

fn combining_to_accent_label(ch: char, mode: Mode) -> String {
    match mode {
        Mode::Math => match ch {
            '\u{0300}' => "\\grave".to_string(),
            '\u{0301}' => "\\acute".to_string(),
            '\u{0302}' => "\\hat".to_string(),
            '\u{0303}' => "\\tilde".to_string(),
            '\u{0304}' => "\\bar".to_string(),
            '\u{0306}' => "\\breve".to_string(),
            '\u{0307}' => "\\dot".to_string(),
            '\u{0308}' => "\\ddot".to_string(),
            '\u{030A}' => "\\mathring".to_string(),
            '\u{030B}' => "\\H".to_string(),
            '\u{030C}' => "\\check".to_string(),
            '\u{0327}' => "\\c".to_string(),
            _ => format!("\\char\"{:X}", ch as u32),
        },
        Mode::Text => match ch {
            '\u{0300}' => "\\`".to_string(),
            '\u{0301}' => "\\'".to_string(),
            '\u{0302}' => "\\^".to_string(),
            '\u{0303}' => "\\~".to_string(),
            '\u{0304}' => "\\=".to_string(),
            '\u{0306}' => "\\u".to_string(),
            '\u{0307}' => "\\.".to_string(),
            '\u{0308}' => "\\\"".to_string(),
            '\u{030A}' => "\\r".to_string(),
            '\u{030B}' => "\\H".to_string(),
            '\u{030C}' => "\\v".to_string(),
            '\u{0327}' => "\\c".to_string(),
            _ => format!("\\char\"{:X}", ch as u32),
        },
    }
}

fn is_valid_unit(unit: &str) -> bool {
    matches!(
        unit,
        "pt" | "mm" | "cm" | "in" | "bp" | "pc" | "dd" | "cc" | "nd" | "nc" | "sp" | "px"
            | "ex" | "em" | "mu"
    )
}

/// If the whole expression is wrapped in TeX inline/display math delimiters, parse the inside only.
/// The parser already runs in math mode; a leading `$` would otherwise hit the `$` / `\\(` "switch to math"
/// handler, which is disallowed in math mode (see `functions::math`).
fn strip_outer_math_delimiters(input: &str) -> &str {
    let s = input.trim();
    if s.len() >= 4 && s.starts_with("$$") && s.ends_with("$$") {
        return s[2..s.len() - 2].trim();
    }
    if s.len() >= 2 && s.starts_with('$') && s.ends_with('$') {
        return s[1..s.len() - 1].trim();
    }
    s
}

/// Convenience function: parse a LaTeX string and return the AST.
pub fn parse(input: &str) -> ParseResult<Vec<ParseNode>> {
    Parser::new(strip_outer_math_delimiters(input)).parse()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_char() {
        let result = parse("x").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].type_name(), "mathord");
    }

    #[test]
    fn test_parse_strips_outer_dollar_inline_math() {
        let inner = r"C_p[\ce{H2O(l)}] = \pu{75.3 J // mol K}";
        let wrapped = format!("${inner}$");
        let a = parse(&wrapped).expect("wrapped");
        let b = parse(inner).expect("inner");
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.type_name(), y.type_name());
        }
    }

    #[test]
    fn test_parse_addition() {
        let result = parse("a+b").unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].type_name(), "mathord"); // a
        assert_eq!(result[1].type_name(), "atom"); // +
        assert_eq!(result[2].type_name(), "mathord"); // b
    }

    #[test]
    fn test_parse_superscript() {
        let result = parse("x^2").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].type_name(), "supsub");
    }

    #[test]
    fn test_parse_subscript() {
        let result = parse("a_i").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].type_name(), "supsub");
    }

    #[test]
    fn test_parse_supsub() {
        let result = parse("x^2_i").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].type_name(), "supsub");
        if let ParseNode::SupSub { sup, sub, .. } = &result[0] {
            assert!(sup.is_some());
            assert!(sub.is_some());
        } else {
            panic!("Expected SupSub");
        }
    }

    #[test]
    fn test_parse_group() {
        let result = parse("{a+b}").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].type_name(), "ordgroup");
    }

    #[test]
    fn test_parse_frac() {
        let result = parse("\\frac{a}{b}").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].type_name(), "genfrac");
    }

    #[test]
    fn test_parse_sqrt() {
        let result = parse("\\sqrt{x}").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].type_name(), "sqrt");
    }

    #[test]
    fn test_parse_sqrt_optional() {
        let result = parse("\\sqrt[3]{x}").unwrap();
        assert_eq!(result.len(), 1);
        if let ParseNode::Sqrt { index, .. } = &result[0] {
            assert!(index.is_some());
        } else {
            panic!("Expected Sqrt");
        }
    }

    #[test]
    fn test_parse_nested() {
        let result = parse("\\frac{\\sqrt{a^2+b^2}}{c}").unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].type_name(), "genfrac");
    }

    #[test]
    fn test_parse_empty() {
        let result = parse("").unwrap();
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_parse_double_superscript_error() {
        let result = parse("x^2^3");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_unclosed_brace_error() {
        let result = parse("{x");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_json_output() {
        let result = parse("x^2").unwrap();
        let json = serde_json::to_string_pretty(&result).unwrap();
        assert!(json.contains("supsub"));
    }
}
