use ratex_lexer::token::{SourceLocation, Token};

/// Error type for the parser, modeled after KaTeX's ParseError.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub loc: Option<SourceLocation>,
}

impl ParseError {
    pub fn new(message: impl Into<String>, token: Option<&Token>) -> Self {
        Self {
            message: message.into(),
            loc: token.map(|t| t.loc.clone()),
        }
    }

    pub fn msg(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            loc: None,
        }
    }

    pub fn at(message: impl Into<String>, loc: SourceLocation) -> Self {
        Self {
            message: message.into(),
            loc: Some(loc),
        }
    }

    pub fn recursion_limit_exceeded() -> Self {
        Self::msg("Recursion limit exceeded")
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref loc) = self.loc {
            write!(f, "ParseError at position {}: {}", loc.start, self.message)
        } else {
            write!(f, "ParseError: {}", self.message)
        }
    }
}

impl std::error::Error for ParseError {}

pub type ParseResult<T> = Result<T, ParseError>;
