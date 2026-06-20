use std::collections::HashMap;
use std::sync::OnceLock;

use crate::data::symbols_data;

/// A symbol's mode in TeX.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    Math,
    Text,
}

/// The font family for a symbol.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolFont {
    Main,
    Ams,
}

/// The group/atom type of a symbol, determining spacing behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Group {
    Bin,
    Close,
    Inner,
    Open,
    Punct,
    Rel,
    AccentToken,
    MathOrd,
    OpToken,
    Spacing,
    TextOrd,
}

impl Group {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "bin" => Some(Self::Bin),
            "close" => Some(Self::Close),
            "inner" => Some(Self::Inner),
            "open" => Some(Self::Open),
            "punct" => Some(Self::Punct),
            "rel" => Some(Self::Rel),
            "accent-token" => Some(Self::AccentToken),
            "mathord" => Some(Self::MathOrd),
            "op-token" => Some(Self::OpToken),
            "spacing" => Some(Self::Spacing),
            "textord" => Some(Self::TextOrd),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bin => "bin",
            Self::Close => "close",
            Self::Inner => "inner",
            Self::Open => "open",
            Self::Punct => "punct",
            Self::Rel => "rel",
            Self::AccentToken => "accent-token",
            Self::MathOrd => "mathord",
            Self::OpToken => "op-token",
            Self::Spacing => "spacing",
            Self::TextOrd => "textord",
        }
    }

    pub fn is_atom(self) -> bool {
        matches!(
            self,
            Self::Bin | Self::Close | Self::Inner | Self::Open | Self::Punct | Self::Rel
        )
    }
}

/// Information about a resolved symbol.
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    pub name: &'static str,
    pub mode: Mode,
    pub font: SymbolFont,
    pub group: Group,
    pub codepoint: Option<char>,
}

/// (mode, name) → index into SYMBOLS array
type SymbolIndex = HashMap<(u8, &'static str), usize>;
/// (mode, codepoint) → index into SYMBOLS array
type CodepointIndex = HashMap<(u8, char), usize>;

struct SymbolMaps {
    by_name: SymbolIndex,
    by_codepoint: CodepointIndex,
}

fn symbol_maps() -> &'static SymbolMaps {
    static MAPS: OnceLock<SymbolMaps> = OnceLock::new();
    MAPS.get_or_init(|| {
        let mut by_name = HashMap::with_capacity(symbols_data::SYMBOLS.len());
        let mut by_codepoint = HashMap::with_capacity(symbols_data::SYMBOLS.len());
        for (i, &(name, mode, _, _, cp)) in symbols_data::SYMBOLS.iter().enumerate() {
            by_name.entry((mode, name)).or_insert(i);
            if let Some(ch) = cp {
                by_codepoint.entry((mode, ch)).or_insert(i);
            }
        }
        SymbolMaps {
            by_name,
            by_codepoint,
        }
    })
}

fn entry_to_info(idx: usize, mode: Mode) -> SymbolInfo {
    let (n, _, f, g, cp) = symbols_data::SYMBOLS[idx];
    SymbolInfo {
        name: n,
        mode,
        font: if f == 0 {
            SymbolFont::Main
        } else {
            SymbolFont::Ams
        },
        group: Group::parse(g).unwrap_or(Group::MathOrd),
        codepoint: cp,
    }
}

/// Look up a symbol by name in a given mode. O(1) via HashMap.
///
/// When `name` is a single Unicode character, this also tries codepoint lookup,
/// matching KaTeX's `acceptUnicodeChar` behavior where both `name` and `replace`
/// are valid keys (e.g. both `\alpha` and `α` resolve to the same symbol).
pub fn get_symbol(name: &str, mode: Mode) -> Option<SymbolInfo> {
    let mode_val: u8 = match mode {
        Mode::Math => 0,
        Mode::Text => 1,
    };
    // Direct name lookup (command name like "\\alpha")
    if let Some(&idx) = symbol_maps().by_name.get(&(mode_val, name)) {
        return Some(entry_to_info(idx, mode));
    }
    // KaTeX acceptUnicodeChar: replace string is also a key. Single-char name → try by codepoint.
    let mut chars = name.chars();
    if let (Some(ch), None) = (chars.next(), chars.next()) {
        if let Some(info) = get_symbol_by_codepoint(ch, mode) {
            return Some(info);
        }
    }
    None
}

/// Look up a math-mode symbol by name (convenience function).
pub fn get_math_symbol(name: &str) -> Option<SymbolInfo> {
    get_symbol(name, Mode::Math)
}

/// Look up a text-mode symbol by name (convenience function).
pub fn get_text_symbol(name: &str) -> Option<SymbolInfo> {
    get_symbol(name, Mode::Text)
}

/// Look up a symbol by its Unicode codepoint in a given mode. O(1) via HashMap.
pub fn get_symbol_by_codepoint(ch: char, mode: Mode) -> Option<SymbolInfo> {
    let mode_val: u8 = match mode {
        Mode::Math => 0,
        Mode::Text => 1,
    };
    symbol_maps()
        .by_codepoint
        .get(&(mode_val, ch))
        .map(|&idx| entry_to_info(idx, mode))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_equiv() {
        let sym = get_math_symbol("\\equiv").unwrap();
        assert_eq!(sym.group, Group::Rel);
        assert_eq!(sym.font, SymbolFont::Main);
        assert_eq!(sym.codepoint, Some('\u{2261}'));
    }

    #[test]
    fn test_get_alpha() {
        let sym = get_math_symbol("\\alpha").unwrap();
        assert_eq!(sym.group, Group::MathOrd);
        assert_eq!(sym.codepoint, Some('\u{03B1}'));
    }

    #[test]
    fn test_get_plus() {
        let sym = get_math_symbol("+").unwrap();
        assert_eq!(sym.group, Group::Bin);
    }

    #[test]
    fn test_get_lparen() {
        let sym = get_math_symbol("\\lparen").unwrap();
        assert_eq!(sym.group, Group::Open);
        assert_eq!(sym.codepoint, Some('('));
    }

    #[test]
    fn test_get_rparen() {
        let sym = get_math_symbol("\\rparen").unwrap();
        assert_eq!(sym.group, Group::Close);
        assert_eq!(sym.codepoint, Some(')'));
    }

    #[test]
    fn test_get_sum() {
        let sym = get_math_symbol("\\sum").unwrap();
        assert_eq!(sym.group, Group::OpToken);
    }

    #[test]
    fn test_get_int() {
        let sym = get_math_symbol("\\int").unwrap();
        assert_eq!(sym.group, Group::OpToken);
    }

    #[test]
    fn test_get_frac_not_a_symbol() {
        // \frac is a command, not a symbol
        assert!(get_math_symbol("\\frac").is_none());
    }

    #[test]
    fn test_text_mode_hash() {
        let sym = get_text_symbol("\\#").unwrap();
        assert_eq!(sym.group, Group::TextOrd);
    }

    #[test]
    fn test_math_forall() {
        let sym = get_math_symbol("\\forall").unwrap();
        assert_eq!(sym.codepoint, Some('\u{2200}'));
    }

    #[test]
    fn test_ams_symbol() {
        let sym = get_math_symbol("\\beth").unwrap();
        assert_eq!(sym.font, SymbolFont::Ams);
        assert_eq!(sym.codepoint, Some('\u{2136}'));
    }

    #[test]
    fn test_equals_is_rel() {
        let sym = get_math_symbol("=").unwrap();
        assert_eq!(sym.group, Group::Rel);
    }

    #[test]
    fn test_by_codepoint() {
        let sym = get_symbol_by_codepoint('\u{2261}', Mode::Math).unwrap();
        assert_eq!(sym.name, "\\equiv");
    }

    #[test]
    fn test_nonexistent() {
        assert!(get_math_symbol("\\nonexistent_command_xyz").is_none());
    }

    #[test]
    fn test_digit_is_textord() {
        // In KaTeX, digits in math mode are classified as "textord"
        for d in '0'..='9' {
            let name = d.to_string();
            let sym = get_math_symbol(&name).unwrap();
            assert_eq!(sym.group, Group::TextOrd, "digit {} should be textord", d);
        }
    }

    #[test]
    fn test_letters_are_mathord() {
        for ch in 'a'..='z' {
            let name = ch.to_string();
            let sym = get_math_symbol(&name).unwrap();
            assert_eq!(sym.group, Group::MathOrd, "letter {} should be mathord", ch);
        }
    }

    #[test]
    fn test_accept_unicode_char_by_name() {
        // KaTeX acceptUnicodeChar: symbol can be looked up by replace (Unicode char) as name
        let sym = get_symbol("α", Mode::Math).unwrap();
        assert_eq!(sym.name, "\\alpha");
        assert_eq!(sym.group, Group::MathOrd);
        assert_eq!(sym.codepoint, Some('α'));
    }
}
