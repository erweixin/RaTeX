//! mhchem (`\ce`, `\pu`): pure Rust port of KaTeX mhchem 3.3.0.
//!
//! Data (`machines.json`, `patterns_regex.json`) is generated from `tools/mhchem_reference.js`;
//! update workflow: `docs/MHCHEM_DATA.md`.

mod actions;
mod buffer;
mod data;
mod engine;
mod error;
mod json;
mod patterns;
mod texify;

pub use data::data;
pub use error::{MhchemError, MhchemResult};

use crate::mhchem::data::MhchemData;
use serde_json::Value;

/// Context for recursive `go` (used by actions).
pub struct ParserCtx<'a> {
    pub data: &'a MhchemData,
}

impl ParserCtx<'_> {
    pub fn go(&self, input: &str, machine: &str) -> MhchemResult<Vec<Value>> {
        engine::go_machine(self, input, machine)
    }
}

/// Parse `\ce` / `\pu` argument to TeX fragment (wrap `\mathrm` etc. is done here).
pub fn chem_parse_str(input: &str, mode: &str) -> MhchemResult<String> {
    let d = data();
    let ctx = ParserCtx { data: d };
    let sm = match mode {
        "ce" => "ce",
        "pu" => "pu",
        _ => {
            return Err(MhchemError::msg(format!(
                "unknown mhchem mode (expected ce|pu): {mode}"
            )));
        }
    };
    let ast = ctx.go(input.trim(), sm)?;
    texify::go(&ast, false)
}

/// Rebuild a macro argument string from tokens ([KaTeX `chemParse`]).
pub fn mhchem_arg_tokens_to_string(tokens: &[ratex_lexer::token::Token]) -> String {
    if tokens.is_empty() {
        return String::new();
    }
    let mut expected_loc = tokens.last().unwrap().loc.start;
    let mut out = String::new();
    for i in (0..tokens.len()).rev() {
        let t = &tokens[i];
        if t.loc.start > expected_loc {
            out.push(' ');
            expected_loc = t.loc.start;
        }
        out.push_str(&t.text);
        expected_loc += t.text.len();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn h2o_ce() {
        let t = chem_parse_str("H2O", "ce").expect("mhchem");
        assert!(!t.is_empty());
        assert!(t.contains('H'));
    }

    #[test]
    fn reaction_arrow() {
        let t = chem_parse_str("2H + O -> H2O", "ce").expect("mhchem");
        assert!(
            t.contains("rightarrow") || t.contains("->"),
            "{}",
            t
        );
    }

    #[test]
    fn pu_simple() {
        let t = chem_parse_str("123 kJ/mol", "pu").expect("mhchem");
        assert!(!t.is_empty());
    }

    #[test]
    fn pu_scientific_lowercase_e_cdot_uppercase_e_times() {
        for src in ["1.2e3 kJ", "1,2e3 kJ"] {
            let t = chem_parse_str(src, "pu").expect("mhchem");
            assert!(
                t.contains("\\cdot") && t.contains("10^{3}") && !t.contains("\\times"),
                "expected \\cdot for lowercase e: {src:?} → {t:?}"
            );
        }
        for src in ["1.2E3 kJ", "1,2E3 kJ"] {
            let t = chem_parse_str(src, "pu").expect("mhchem");
            assert!(
                t.contains("\\times") && t.contains("10^{3}") && !t.contains("\\cdot"),
                "expected \\times for uppercase E: {src:?} → {t:?}"
            );
        }
    }

    #[test]
    fn dollar_underset_inner_ce_tex_is_valid_latex() {
        let inner = r"$\underset{\mathrm{red}}{\ce{HgI2}}$";
        let tex = chem_parse_str(inner, "ce").expect("mhchem");
        crate::parser::parse(&tex).expect("mhchem TeX should parse");
    }

}
