use std::collections::HashMap;

use ratex_lexer::token::{SourceLocation, Token};
use ratex_lexer::Lexer;

use crate::error::{ParseError, ParseResult};
use crate::functions::FUNCTIONS;
use crate::parse_node::Mode;

/// Commands that act like macros but aren't defined as a macro, function, or symbol.
/// Used in `is_defined`.
pub static IMPLICIT_COMMANDS: &[&str] = &["^", "_", "\\limits", "\\nolimits"];

/// Handler type for function-based macros (e.g. \TextOrMath, \@ifstar).
/// Takes the MacroExpander mutably and returns tokens to push onto the stack.
pub type FnMacroHandler = fn(&mut MacroExpander) -> ParseResult<Vec<Token>>;

/// A macro definition: string template, token list, or function.
#[derive(Clone)]
pub enum MacroDefinition {
    /// Simple string expansion (e.g., `\def\foo{bar}` → "bar")
    Text(String),
    /// Pre-tokenized expansion with argument count
    Tokens {
        tokens: Vec<Token>,
        num_args: usize,
    },
    /// Function-based macro (consumes tokens directly, returns expansion)
    Function(FnMacroHandler),
}

impl std::fmt::Debug for MacroDefinition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Text(s) => write!(f, "Text({:?})", s),
            Self::Tokens { tokens, num_args } => {
                write!(f, "Tokens {{ {} tokens, {} args }}", tokens.len(), num_args)
            }
            Self::Function(_) => write!(f, "Function(...)"),
        }
    }
}

/// Result of expanding a macro once.
struct MacroExpansion {
    tokens: Vec<Token>,
    num_args: usize,
    unexpandable: bool,
}

/// The MacroExpander (or "gullet") manages macro expansion.
///
/// It sits between the Lexer (mouth) and the Parser (stomach).
/// Tokens are read from the lexer, pushed onto an internal stack,
/// and macros are expanded until only non-expandable tokens remain.
///
/// Modeled after KaTeX's MacroExpander.ts.
pub struct MacroExpander<'a> {
    pub lexer: Lexer<'a>,
    pub mode: Mode,
    stack: Vec<Token>,
    macros: MacroNamespace,
    expansion_count: usize,
    max_expand: usize,
}

/// Scoped macro namespace supporting group nesting.
struct MacroNamespace {
    current: HashMap<String, MacroDefinition>,
    group_stack: Vec<HashMap<String, Option<MacroDefinition>>>,
}

impl MacroNamespace {
    fn new() -> Self {
        Self {
            current: HashMap::new(),
            group_stack: Vec::new(),
        }
    }

    fn get(&self, name: &str) -> Option<&MacroDefinition> {
        self.current.get(name)
    }

    fn set(&mut self, name: String, def: MacroDefinition) {
        if let Some(undo) = self.group_stack.last_mut() {
            undo.entry(name.clone()).or_insert_with(|| self.current.get(&name).cloned());
        }
        self.current.insert(name, def);
    }

    fn set_global(&mut self, name: String, def: MacroDefinition) {
        self.current.insert(name, def);
    }

    fn has(&self, name: &str) -> bool {
        self.current.contains_key(name)
    }

    fn begin_group(&mut self) {
        self.group_stack.push(HashMap::new());
    }

    fn end_group(&mut self) {
        if let Some(undo) = self.group_stack.pop() {
            for (name, old_val) in undo {
                match old_val {
                    Some(def) => { self.current.insert(name, def); }
                    None => { self.current.remove(&name); }
                }
            }
        }
    }

    fn end_groups(&mut self) {
        while !self.group_stack.is_empty() {
            self.end_group();
        }
    }
}

impl<'a> MacroExpander<'a> {
    pub fn new(input: &'a str, mode: Mode) -> Self {
        let mut me = Self {
            lexer: Lexer::new(input),
            mode,
            stack: Vec::new(),
            macros: MacroNamespace::new(),
            expansion_count: 0,
            max_expand: 1000,
        };
        me.load_builtins();
        me
    }

    fn load_builtins(&mut self) {
        let builtins: &[(&str, &str)] = &[
            // ── Grouping ──
            ("\\bgroup", "{"),
            ("\\egroup", "}"),

            // ── Symbols from latex.ltx ──
            ("\\lq", "`"),
            ("\\rq", "'"),
            // \lbrack and \rbrack are in the symbol table directly
            ("\\aa", "\\r a"),
            ("\\AA", "\\r A"),

            // ── Active characters ──
            ("~", "\\nobreakspace"),

            // ── Phantoms ──
            ("\\hphantom", "\\smash{\\phantom{#1}}"),

            // ── Negated symbols ──
            ("\\not", "\\html@mathml{\\mathrel{\\mathrlap\\@not}\\nobreak}{\\char\"338}"),
            ("\\neq", "\\html@mathml{\\mathrel{\\not=}}{\\mathrel{\\char`≠}}"),
            ("\\ne", "\\neq"),
            ("\u{2260}", "\\neq"),
            ("\\notin", "\\html@mathml{\\mathrel{{\\in}\\mathllap{/\\mskip1mu}}}{\\mathrel{\\char`∉}}"),
            ("\u{2209}", "\\notin"),
            ("\\notni", "\\html@mathml{\\not\\ni}{\\mathrel{\\char`\u{220C}}}"),
            ("\u{220C}", "\\notni"),
            // \le and \ge are in the symbol table directly, not macros

            // ── amsmath iff/implies ──
            ("\\iff", "\\DOTSB\\;\\Longleftrightarrow\\;"),
            ("\\implies", "\\DOTSB\\;\\Longrightarrow\\;"),
            ("\\impliedby", "\\DOTSB\\;\\Longleftarrow\\;"),

            // ── Italic Greek capitals ──
            ("\\varGamma", "\\mathit{\\Gamma}"),
            ("\\varDelta", "\\mathit{\\Delta}"),
            ("\\varTheta", "\\mathit{\\Theta}"),
            ("\\varLambda", "\\mathit{\\Lambda}"),
            ("\\varXi", "\\mathit{\\Xi}"),
            ("\\varPi", "\\mathit{\\Pi}"),
            ("\\varSigma", "\\mathit{\\Sigma}"),
            ("\\varUpsilon", "\\mathit{\\Upsilon}"),
            ("\\varPhi", "\\mathit{\\Phi}"),
            ("\\varPsi", "\\mathit{\\Psi}"),
            ("\\varOmega", "\\mathit{\\Omega}"),

            // ── Spacing (mode-aware via \TextOrMath) ──
            ("\\,", "\\TextOrMath{\\kern{.1667em}}{\\mskip{3mu}}"),
            ("\\thinspace", "\\,"),
            ("\\>", "\\mskip{4mu}"),
            ("\\:", "\\TextOrMath{\\kern{.2222em}}{\\mskip{4mu}}"),
            ("\\medspace", "\\:"),
            ("\\;", "\\TextOrMath{\\kern{.2777em}}{\\mskip{5mu}}"),
            ("\\thickspace", "\\;"),
            ("\\!", "\\TextOrMath{\\kern{-.1667em}}{\\mskip{-3mu}}"),
            ("\\negthinspace", "\\!"),
            ("\\negmedspace", "\\TextOrMath{\\kern{-.2222em}}{\\mskip{-4mu}}"),
            ("\\negthickspace", "\\TextOrMath{\\kern{-.2777em}}{\\mskip{-5mu}}"),
            ("\\enspace", "\\kern.5em "),
            ("\\enskip", "\\hskip.5em\\relax"),
            ("\\quad", "\\hskip1em\\relax"),
            ("\\qquad", "\\hskip2em\\relax"),

            // ── Newline ──
            ("\\newline", "\\\\\\relax"),

            // ── hspace ──
            ("\\@hspace", "\\hskip #1\\relax"),
            ("\\@hspacer", "\\rule{0pt}{0pt}\\hskip #1\\relax"),

            // ── llap / rlap / clap ──
            ("\\llap", "\\mathllap{\\textrm{#1}}"),
            ("\\rlap", "\\mathrlap{\\textrm{#1}}"),
            ("\\clap", "\\mathclap{\\textrm{#1}}"),

            // ── Logos ──
            ("\\TeX", "\\textrm{\\html@mathml{T\\kern-.1667em\\raisebox{-.5ex}{E}\\kern-.125emX}{TeX}}"),
            ("\\LaTeX", "\\textrm{\\html@mathml{L\\kern-.36em\\raisebox{0.21em}{\\scriptstyle A}\\kern-.15em\\TeX}{LaTeX}}"),
            ("\\KaTeX", "\\textrm{\\html@mathml{K\\kern-.17em\\raisebox{0.21em}{\\scriptstyle A}\\kern-.15em\\TeX}{KaTeX}}"),

            // ── imath / jmath ──
            ("\\imath", "\\html@mathml{\\@imath}{\u{0131}}"),
            ("\\jmath", "\\html@mathml{\\@jmath}{\u{0237}}"),

            // ── minuso ──
            ("\\minuso", "\\mathbin{\\html@mathml{{\\mathrlap{\\mathchoice{\\kern{0.145em}}{\\kern{0.145em}}{\\kern{0.1015em}}{\\kern{0.0725em}}\\circ}{-}}}{\\char`\u{29B5}}}"),
            ("\\clap", "\\mathclap{\\textrm{#1}}"),

            // ── mathstrut / underbar ──
            ("\\mathstrut", "\\vphantom{(}"),
            ("\\underbar", "\\underline{\\text{#1}}"),

            // ── Bbbk ──
            ("\\Bbbk", "\\Bbb{k}"),

            // ── substack ──
            ("\\substack", "\\begin{subarray}{c}#1\\end{subarray}"),

            // ── boxed ──
            ("\\boxed", "\\fbox{$\\displaystyle{#1}$}"),

            // ── colon ──
            ("\\colon", "\\nobreak\\mskip2mu\\mathpunct{}\\mathchoice{\\mkern-3mu}{\\mkern-3mu}{}{}{:}\\mskip6mu\\relax"),

            // ── dots (string-based) ──
            ("\\dots", "\\cdots"),
            ("\\cdots", "\\@cdots"),
            ("\\dotsb", "\\cdots"),
            ("\\dotsm", "\\cdots"),
            ("\\dotsi", "\\!\\cdots"),
            ("\\dotsx", "\\ldots\\,"),
            ("\\dotsc", "\\ldots"),  // comma list: x,\dotsc,y
            ("\\dotso", "\\ldots"),  // other
            ("\\DOTSI", "\\relax"),
            ("\\DOTSB", "\\relax"),
            ("\\DOTSX", "\\relax"),

            // ── negated relations / corners (→ symbol table \@xxx) ──
            ("\\gvertneqq", "\\@gvertneqq"),
            ("\\lvertneqq", "\\@lvertneqq"),
            ("\\ngeqq", "\\@ngeqq"),
            ("\\ngeqslant", "\\@ngeqslant"),
            ("\\nleqq", "\\@nleqq"),
            ("\\nleqslant", "\\@nleqslant"),
            ("\\nshortmid", "\\@nshortmid"),
            ("\\nshortparallel", "\\@nshortparallel"),
            ("\\nsubseteqq", "\\@nsubseteqq"),
            ("\\nsupseteqq", "\\@nsupseteqq"),
            ("\\ulcorner", "\\@ulcorner"),
            ("\\urcorner", "\\@urcorner"),
            ("\\llcorner", "\\@llcorner"),
            ("\\lrcorner", "\\@lrcorner"),
            ("\\varsubsetneq", "\\@varsubsetneq"),
            ("\\varsubsetneqq", "\\@varsubsetneqq"),
            ("\\varsupsetneq", "\\@varsupsetneq"),
            ("\\varsupsetneqq", "\\@varsupsetneqq"),

            // ── delimiters / text (compose from existing) ──
            ("\\lBrace", "\\lbrace"),
            ("\\rBrace", "\\rbrace"),
            ("\\llbracket", "\\lbrack\\lbrack"),
            ("\\rrbracket", "\\rbrack\\rbrack"),
            ("\\copyright", "\\text{\u{00A9}}"),
            ("\\textregistered", "\u{00AE}"),

            // ── dddot / ddddot ──
            ("\\dddot", "{\\overset{\\raisebox{-0.1ex}{\\normalsize ...}}{#1}}"),
            ("\\ddddot", "{\\overset{\\raisebox{-0.1ex}{\\normalsize ....}}{#1}}"),

            // ── vdots ──
            ("\\vdots", "{\\varvdots\\rule{0pt}{15pt}}"),
            ("\u{22ee}", "\\vdots"),

            // ── bmod / pod / pmod / mod ──
            ("\\bmod", "\\mathchoice{\\mskip1mu}{\\mskip1mu}{\\mskip5mu}{\\mskip5mu}\\mathbin{\\rm mod}\\mathchoice{\\mskip1mu}{\\mskip1mu}{\\mskip5mu}{\\mskip5mu}"),
            ("\\pod", "\\allowbreak\\mathchoice{\\mkern18mu}{\\mkern8mu}{\\mkern8mu}{\\mkern8mu}(#1)"),
            ("\\pmod", "\\pod{{\\rm mod}\\mkern6mu#1}"),
            ("\\mod", "\\allowbreak\\mathchoice{\\mkern18mu}{\\mkern12mu}{\\mkern12mu}{\\mkern12mu}{\\rm mod}\\,\\,#1"),

            // ── limsup / liminf / etc ──
            ("\\limsup", "\\DOTSB\\operatorname*{lim\\,sup}"),
            ("\\liminf", "\\DOTSB\\operatorname*{lim\\,inf}"),
            ("\\injlim", "\\DOTSB\\operatorname*{inj\\,lim}"),
            ("\\projlim", "\\DOTSB\\operatorname*{proj\\,lim}"),
            ("\\varlimsup", "\\DOTSB\\operatorname*{\\overline{lim}}"),
            ("\\varliminf", "\\DOTSB\\operatorname*{\\underline{lim}}"),
            ("\\varinjlim", "\\DOTSB\\operatorname*{\\underrightarrow{lim}}"),
            ("\\varprojlim", "\\DOTSB\\operatorname*{\\underleftarrow{lim}}"),

            // ── statmath ──
            ("\\argmin", "\\DOTSB\\operatorname*{arg\\,min}"),
            ("\\argmax", "\\DOTSB\\operatorname*{arg\\,max}"),
            ("\\plim", "\\DOTSB\\mathop{\\operatorname{plim}}\\limits"),

            // ── mathtools colon variants ──
            ("\\ordinarycolon", ":"),
            ("\\vcentcolon", "\\mathrel{\\mathop\\ordinarycolon}"),
            ("\\dblcolon", "\\html@mathml{\\mathrel{\\vcentcolon\\mathrel{\\mkern-.9mu}\\vcentcolon}}{\\mathop{\\char\"2237}}"),
            ("\\coloneqq", "\\html@mathml{\\mathrel{\\vcentcolon\\mathrel{\\mkern-1.2mu}=}}{\\mathop{\\char\"2254}}"),
            ("\\Coloneqq", "\\html@mathml{\\mathrel{\\dblcolon\\mathrel{\\mkern-1.2mu}=}}{\\mathop{\\char\"2237\\char\"3d}}"),
            ("\\coloneq", "\\html@mathml{\\mathrel{\\vcentcolon\\mathrel{\\mkern-1.2mu}\\mathrel{-}}}{\\mathop{\\char\"3a\\char\"2212}}"),
            ("\\Coloneq", "\\html@mathml{\\mathrel{\\dblcolon\\mathrel{\\mkern-1.2mu}\\mathrel{-}}}{\\mathop{\\char\"2237\\char\"2212}}"),
            ("\\eqqcolon", "\\html@mathml{\\mathrel{=\\mathrel{\\mkern-1.2mu}\\vcentcolon}}{\\mathop{\\char\"2255}}"),
            ("\\Eqqcolon", "\\html@mathml{\\mathrel{=\\mathrel{\\mkern-1.2mu}\\dblcolon}}{\\mathop{\\char\"3d\\char\"2237}}"),
            ("\\eqcolon", "\\html@mathml{\\mathrel{\\mathrel{-}\\mathrel{\\mkern-1.2mu}\\vcentcolon}}{\\mathop{\\char\"2239}}"),
            ("\\Eqcolon", "\\html@mathml{\\mathrel{\\mathrel{-}\\mathrel{\\mkern-1.2mu}\\dblcolon}}{\\mathop{\\char\"2212\\char\"2237}}"),
            ("\\colonapprox", "\\html@mathml{\\mathrel{\\vcentcolon\\mathrel{\\mkern-1.2mu}\\approx}}{\\mathop{\\char\"3a\\char\"2248}}"),
            ("\\Colonapprox", "\\html@mathml{\\mathrel{\\dblcolon\\mathrel{\\mkern-1.2mu}\\approx}}{\\mathop{\\char\"2237\\char\"2248}}"),
            ("\\colonsim", "\\html@mathml{\\mathrel{\\vcentcolon\\mathrel{\\mkern-1.2mu}\\sim}}{\\mathop{\\char\"3a\\char\"223c}}"),
            ("\\Colonsim", "\\html@mathml{\\mathrel{\\dblcolon\\mathrel{\\mkern-1.2mu}\\sim}}{\\mathop{\\char\"2237\\char\"223c}}"),

            // ── colonequals alternate names ──
            ("\\ratio", "\\vcentcolon"),
            ("\\coloncolon", "\\dblcolon"),
            ("\\colonequals", "\\coloneqq"),
            ("\\coloncolonequals", "\\Coloneqq"),
            ("\\equalscolon", "\\eqqcolon"),
            ("\\equalscoloncolon", "\\Eqqcolon"),
            ("\\colonminus", "\\coloneq"),
            ("\\coloncolonminus", "\\Coloneq"),
            ("\\minuscolon", "\\eqcolon"),
            ("\\minuscoloncolon", "\\Eqcolon"),
            ("\\coloncolonapprox", "\\Colonapprox"),
            ("\\coloncolonsim", "\\Colonsim"),
            ("\\simcolon", "\\mathrel{\\sim\\mathrel{\\mkern-1.2mu}\\vcentcolon}"),
            ("\\simcoloncolon", "\\mathrel{\\sim\\mathrel{\\mkern-1.2mu}\\dblcolon}"),
            ("\\approxcolon", "\\mathrel{\\approx\\mathrel{\\mkern-1.2mu}\\vcentcolon}"),
            ("\\approxcoloncolon", "\\mathrel{\\approx\\mathrel{\\mkern-1.2mu}\\dblcolon}"),

            // ── braket (string-based) ──
            ("\\bra", "\\mathinner{\\langle{#1}|}"),
            ("\\ket", "\\mathinner{|{#1}\\rangle}"),
            ("\\braket", "\\mathinner{\\langle{#1}\\rangle}"),
            ("\\Braket", "\\mathinner{\\langle{#1}\\rangle}"),
            ("\\Bra", "\\left\\langle#1\\right|"),
            ("\\Ket", "\\left|#1\\right\\rangle"),

            // ── texvc (MediaWiki) ──
            ("\\darr", "\\downarrow"),
            ("\\dArr", "\\Downarrow"),
            ("\\Darr", "\\Downarrow"),
            ("\\lang", "\\langle"),
            ("\\rang", "\\rangle"),
            ("\\uarr", "\\uparrow"),
            ("\\uArr", "\\Uparrow"),
            ("\\Uarr", "\\Uparrow"),
            ("\\N", "\\mathbb{N}"),
            ("\\R", "\\mathbb{R}"),
            ("\\Z", "\\mathbb{Z}"),
            ("\\alef", "\\aleph"),
            ("\\alefsym", "\\aleph"),
            ("\\Alpha", "\\mathrm{A}"),
            ("\\Beta", "\\mathrm{B}"),
            ("\\bull", "\\bullet"),
            ("\\Chi", "\\mathrm{X}"),
            ("\\clubs", "\\clubsuit"),
            ("\\cnums", "\\mathbb{C}"),
            ("\\Complex", "\\mathbb{C}"),
            ("\\Dagger", "\\ddagger"),
            ("\\diamonds", "\\diamondsuit"),
            ("\\empty", "\\emptyset"),
            ("\\Epsilon", "\\mathrm{E}"),
            ("\\Eta", "\\mathrm{H}"),
            ("\\exist", "\\exists"),
            ("\\harr", "\\leftrightarrow"),
            ("\\hArr", "\\Leftrightarrow"),
            ("\\Harr", "\\Leftrightarrow"),
            ("\\hearts", "\\heartsuit"),
            ("\\image", "\\Im"),
            ("\\infin", "\\infty"),
            ("\\Iota", "\\mathrm{I}"),
            ("\\isin", "\\in"),
            ("\\Kappa", "\\mathrm{K}"),
            ("\\larr", "\\leftarrow"),
            ("\\lArr", "\\Leftarrow"),
            ("\\Larr", "\\Leftarrow"),
            ("\\lrarr", "\\leftrightarrow"),
            ("\\lrArr", "\\Leftrightarrow"),
            ("\\Lrarr", "\\Leftrightarrow"),
            ("\\Mu", "\\mathrm{M}"),
            ("\\natnums", "\\mathbb{N}"),
            ("\\Nu", "\\mathrm{N}"),
            ("\\Omicron", "\\mathrm{O}"),
            ("\\plusmn", "\\pm"),
            ("\\rarr", "\\rightarrow"),
            ("\\rArr", "\\Rightarrow"),
            ("\\Rarr", "\\Rightarrow"),
            ("\\real", "\\Re"),
            ("\\reals", "\\mathbb{R}"),
            ("\\Reals", "\\mathbb{R}"),
            ("\\Rho", "\\mathrm{P}"),
            ("\\sdot", "\\cdot"),
            ("\\sect", "\\S"),
            ("\\spades", "\\spadesuit"),
            ("\\sub", "\\subset"),
            ("\\sube", "\\subseteq"),
            ("\\supe", "\\supseteq"),
            ("\\Tau", "\\mathrm{T}"),
            ("\\thetasym", "\\vartheta"),
            ("\\weierp", "\\wp"),
            ("\\Zeta", "\\mathrm{Z}"),

            // ── Khan Academy color aliases ──
            ("\\blue", "\\textcolor{##6495ed}{#1}"),
            ("\\orange", "\\textcolor{##ffa500}{#1}"),
            ("\\pink", "\\textcolor{##ff00af}{#1}"),
            ("\\red", "\\textcolor{##df0030}{#1}"),
            ("\\green", "\\textcolor{##28ae7b}{#1}"),
            ("\\gray", "\\textcolor{gray}{#1}"),
            ("\\purple", "\\textcolor{##9d38bd}{#1}"),

            // ── Unicode script letters ──
            ("\u{212C}", "\\mathscr{B}"),
            ("\u{2130}", "\\mathscr{E}"),
            ("\u{2131}", "\\mathscr{F}"),
            ("\u{210B}", "\\mathscr{H}"),
            ("\u{2110}", "\\mathscr{I}"),
            ("\u{2112}", "\\mathscr{L}"),
            ("\u{2133}", "\\mathscr{M}"),
            ("\u{211B}", "\\mathscr{R}"),
            ("\u{212D}", "\\mathfrak{C}"),
            ("\u{210C}", "\\mathfrak{H}"),
            ("\u{2128}", "\\mathfrak{Z}"),

            // ── notni ──
            ("\\notni", "\\html@mathml{\\not\\ni}{\\mathrel{\\char`\u{220C}}}"),

            // ── actuarialangle ──
            ("\\angln", "{\\angl n}"),

            // ── set/Set (braket notation, simplified) ──
            ("\\set", "\\bra@set{\\{\\,}{\\mid}{}{\\,\\}}"),
            ("\\Set", "\\bra@set{\\left\\{\\:}{\\;\\middle\\vert\\;}{\\;\\middle\\Vert\\;}{\\:\\right\\}}"),

            // ── tag ──
            ("\\tag", "\\@ifstar\\tag@literal\\tag@paren"),
            ("\\tag@paren", "\\tag@literal{({#1})}"),
            ("\\tag@literal", "\\gdef\\df@tag{\\text{#1}}"),
            // ── equation numbering (display math; no-op in parser) ──
            ("\\nonumber", "\\relax"),
            ("\\notag", "\\relax"),
        ];

        for &(name, expansion) in builtins {
            self.macros.set(
                name.to_string(),
                MacroDefinition::Text(expansion.to_string()),
            );
        }

        self.load_function_macros();
    }

    fn load_function_macros(&mut self) {
        // \noexpand: mark the next token as non-expandable (only if expandable)
        self.macros.set(
            "\\noexpand".to_string(),
            MacroDefinition::Function(|me: &mut MacroExpander| -> ParseResult<Vec<Token>> {
                let mut tok = me.pop_token();
                if me.is_expandable(&tok.text) {
                    tok.noexpand = true;
                    tok.treat_as_relax = true;
                }
                Ok(vec![tok])
            }),
        );

        // \@firstoftwo{A}{B} → A
        // NOTE: consume_args returns tokens in stack order (reversed).
        // We return them as-is since expand_once does stack.extend(tokens).
        self.macros.set(
            "\\@firstoftwo".to_string(),
            MacroDefinition::Function(|me: &mut MacroExpander| -> ParseResult<Vec<Token>> {
                let args = me.consume_args(2)?;
                Ok(args.into_iter().next().unwrap())
            }),
        );

        // \@secondoftwo{A}{B} → B
        self.macros.set(
            "\\@secondoftwo".to_string(),
            MacroDefinition::Function(|me: &mut MacroExpander| -> ParseResult<Vec<Token>> {
                let args = me.consume_args(2)?;
                Ok(args.into_iter().nth(1).unwrap())
            }),
        );

        // \@ifnextchar{C}{T}{F}: peek; if next non-space == C then T else F
        self.macros.set(
            "\\@ifnextchar".to_string(),
            MacroDefinition::Function(|me: &mut MacroExpander| -> ParseResult<Vec<Token>> {
                let args = me.consume_args(3)?;
                me.consume_spaces();
                let next = me.future().text.clone();
                let char_arg = &args[0];
                // char_arg is reversed; the "first" char in original order is the last element
                let char_text = char_arg.first().map_or("", |t| t.text.as_str());
                if next == char_text {
                    Ok(args[1].clone())
                } else {
                    Ok(args[2].clone())
                }
            }),
        );

        // \@ifstar{with-star}{without-star}: if next is * → consume * and use first arg
        self.macros.set(
            "\\@ifstar".to_string(),
            MacroDefinition::Function(|me: &mut MacroExpander| -> ParseResult<Vec<Token>> {
                let args = me.consume_args(2)?;
                let next = me.future().text.clone();
                if next == "*" {
                    me.pop_token();
                    Ok(args[0].clone())
                } else {
                    Ok(args[1].clone())
                }
            }),
        );

        // \TextOrMath{text-branch}{math-branch}: choose based on mode
        self.macros.set(
            "\\TextOrMath".to_string(),
            MacroDefinition::Function(|me: &mut MacroExpander| -> ParseResult<Vec<Token>> {
                let args = me.consume_args(2)?;
                if me.mode == Mode::Text {
                    Ok(args[0].clone())
                } else {
                    Ok(args[1].clone())
                }
            }),
        );

        // \html@mathml is registered as a function in htmlmathml.rs

        // \newcommand{\name}[nargs]{body}
        self.macros.set(
            "\\newcommand".to_string(),
            MacroDefinition::Function(|me: &mut MacroExpander| -> ParseResult<Vec<Token>> {
                handle_newcommand(me, false, true)
            }),
        );

        // \renewcommand{\name}[nargs]{body}
        self.macros.set(
            "\\renewcommand".to_string(),
            MacroDefinition::Function(|me: &mut MacroExpander| -> ParseResult<Vec<Token>> {
                handle_newcommand(me, true, false)
            }),
        );

        // \providecommand{\name}[nargs]{body}
        self.macros.set(
            "\\providecommand".to_string(),
            MacroDefinition::Function(|me: &mut MacroExpander| -> ParseResult<Vec<Token>> {
                handle_newcommand(me, true, true)
            }),
        );

        // \char: parse decimal/octal/hex/backtick number → \@char{N}
        self.macros.set(
            "\\char".to_string(),
            MacroDefinition::Function(|me: &mut MacroExpander| -> ParseResult<Vec<Token>> {
                let mut tok = me.pop_token();
                let mut number: i64;
                let base: Option<u32>;

                if tok.text == "'" {
                    base = Some(8);
                    tok = me.pop_token();
                } else if tok.text == "\"" {
                    base = Some(16);
                    tok = me.pop_token();
                } else if tok.text == "`" {
                    tok = me.pop_token();
                    if tok.text.starts_with('\\') {
                        number = tok.text.chars().nth(1).map_or(0, |c| c as i64);
                    } else {
                        number = tok.text.chars().next().map_or(0, |c| c as i64);
                    }
                    // Build \@char{N} tokens in reverse (stack order)
                    let s = number.to_string();
                    let loc = tok.loc.clone();
                    let mut result = vec![Token::new("}", loc.start, loc.end)];
                    for ch in s.chars().rev() {
                        result.push(Token::new(ch.to_string(), loc.start, loc.end));
                    }
                    result.push(Token::new("{", loc.start, loc.end));
                    result.push(Token::new("\\@char", loc.start, loc.end));
                    return Ok(result);
                } else {
                    base = Some(10);
                }

                if let Some(b) = base {
                    number = i64::from_str_radix(&tok.text, b).unwrap_or(0);
                    loop {
                        let next = me.future().text.clone();
                        if let Ok(d) = i64::from_str_radix(&next, b) {
                            me.pop_token();
                            number = number * (b as i64) + d;
                        } else {
                            break;
                        }
                    }
                } else {
                    number = 0;
                }

                let s = number.to_string();
                let loc = tok.loc.clone();
                let mut result = vec![Token::new("}", loc.start, loc.end)];
                for ch in s.chars().rev() {
                    result.push(Token::new(ch.to_string(), loc.start, loc.end));
                }
                result.push(Token::new("{", loc.start, loc.end));
                result.push(Token::new("\\@char", loc.start, loc.end));
                Ok(result)
            }),
        );

        // \operatorname: \@ifstar\operatornamewithlimits\operatorname@
        self.macros.set(
            "\\operatorname".to_string(),
            MacroDefinition::Text(
                "\\@ifstar\\operatornamewithlimits\\operatorname@".to_string(),
            ),
        );

        // \message{...}: consume argument and discard (no-op)
        self.macros.set(
            "\\message".to_string(),
            MacroDefinition::Function(|me: &mut MacroExpander| -> ParseResult<Vec<Token>> {
                let _args = me.consume_args(1)?;
                Ok(vec![])
            }),
        );

        // \errmessage{...}: consume argument and discard (no-op)
        self.macros.set(
            "\\errmessage".to_string(),
            MacroDefinition::Function(|me: &mut MacroExpander| -> ParseResult<Vec<Token>> {
                let _args = me.consume_args(1)?;
                Ok(vec![])
            }),
        );

        // KaTeX HTML extensions: no-op (only render content, no HTML attributes).
        // Not standard LaTeX; for compatibility we parse and expand to second argument only.
        for name in &["\\htmlClass", "\\htmlData", "\\htmlId", "\\htmlStyle"] {
            let name = (*name).to_string();
            self.macros.set(
                name.clone(),
                MacroDefinition::Function(|me: &mut MacroExpander| -> ParseResult<Vec<Token>> {
                    let args = me.consume_args(2)?;
                    let content = args[1].iter().cloned().rev().collect::<Vec<_>>();
                    Ok(content)
                }),
            );
        }

        // \bra@set: braket set notation helper
        // Only replaces the FIRST | with middle tokens (one-shot), matching KaTeX
        self.macros.set(
            "\\bra@set".to_string(),
            MacroDefinition::Function(|me: &mut MacroExpander| -> ParseResult<Vec<Token>> {
                let args = me.consume_args(4)?;
                let left = args[0].clone();
                let middle = args[1].clone();
                let middle_double = args[2].clone();
                let right = args[3].clone();

                let content = me.consume_args(1)?;
                let mut content = content.into_iter().next().unwrap();

                // Scan content and replace only the first | at depth 0
                // Content tokens are in reversed order (stack), so iterate from end
                let mut depth: i32 = 0;
                let mut _first_pipe_idx: Option<usize> = None;
                // Tokens are reversed (last token first in vec), scan in logical order
                for i in (0..content.len()).rev() {
                    let t = &content[i];
                    if t.text == "{" { depth += 1; }
                    else if t.text == "}" { depth -= 1; }
                    else if depth == 0 && t.text == "|" {
                        // Check for || (double pipe) → middleDouble
                        if !middle_double.is_empty() && i > 0 && content[i - 1].text == "|" {
                            _first_pipe_idx = Some(i);
                            // Replace || with middleDouble
                            content.remove(i);
                            content.remove(i - 1);
                            let insert_at = if i >= 2 { i - 1 } else { 0 };
                            for (j, tok) in middle_double.iter().enumerate() {
                                content.insert(insert_at + j, tok.clone());
                            }
                            break;
                        }
                        _first_pipe_idx = Some(i);
                        content.remove(i);
                        for (j, tok) in middle.iter().enumerate() {
                            content.insert(i + j, tok.clone());
                        }
                        break;
                    }
                }

                // Build: right + content + left (reversed for stack)
                let mut to_expand = Vec::new();
                to_expand.extend(right);
                to_expand.extend(content);
                to_expand.extend(left);

                me.begin_group();
                let expanded = me.expand_tokens(to_expand)?;
                me.end_group();

                Ok(expanded)
            }),
        );
    }

    pub fn set_macro(&mut self, name: String, def: MacroDefinition) {
        self.macros.set(name, def);
    }

    pub fn set_macro_global(&mut self, name: String, def: MacroDefinition) {
        self.macros.set_global(name, def);
    }

    pub fn set_text_macro(&mut self, name: &str, text: &str) {
        self.macros.set(
            name.to_string(),
            MacroDefinition::Text(text.to_string()),
        );
    }

    pub fn get_macro(&self, name: &str) -> Option<&MacroDefinition> {
        self.macros.get(name)
    }

    /// Expand a list of tokens fully (for \edef/\xdef).
    pub fn expand_tokens(&mut self, tokens: Vec<Token>) -> ParseResult<Vec<Token>> {
        let saved_stack = std::mem::take(&mut self.stack);
        self.stack = tokens;

        let mut result = Vec::new();
        loop {
            if self.stack.is_empty() {
                break;
            }
            let expanded = self.expand_once(false)?;
            if !expanded {
                if let Some(tok) = self.stack.pop() {
                    if tok.is_eof() {
                        break;
                    }
                    result.push(tok);
                }
            }
        }

        self.stack = saved_stack;
        result.reverse();
        Ok(result)
    }

    pub fn switch_mode(&mut self, new_mode: Mode) {
        self.mode = new_mode;
    }

    pub fn begin_group(&mut self) {
        self.macros.begin_group();
    }

    pub fn end_group(&mut self) {
        self.macros.end_group();
    }

    pub fn end_groups(&mut self) {
        self.macros.end_groups();
    }

    /// Returns the topmost token on the stack, without expanding it.
    pub fn future(&mut self) -> &Token {
        if self.stack.is_empty() {
            let tok = self.lexer.lex();
            self.stack.push(tok);
        }
        self.stack.last().unwrap()
    }

    /// Remove and return the next unexpanded token.
    pub fn pop_token(&mut self) -> Token {
        self.future();
        self.stack.pop().unwrap()
    }

    /// Modify the top token's text on the stack (for \global prefix handling).
    pub fn set_top_text(&mut self, text: String) {
        self.future();
        if let Some(tok) = self.stack.last_mut() {
            tok.text = text;
        }
    }

    /// Push a token onto the stack.
    pub fn push_token(&mut self, token: Token) {
        self.stack.push(token);
    }

    /// Push multiple tokens onto the stack.
    pub fn push_tokens(&mut self, tokens: Vec<Token>) {
        self.stack.extend(tokens);
    }

    /// Consume all following space tokens, without expansion.
    pub fn consume_spaces(&mut self) {
        loop {
            let is_space = self.future().text == " ";
            if is_space {
                self.stack.pop();
            } else {
                break;
            }
        }
    }

    /// Expand the next token once if possible.
    /// Returns Ok(true) if expanded, Ok(false) if not expandable.
    fn expand_once(&mut self, expandable_only: bool) -> ParseResult<bool> {
        let top_token = self.pop_token();
        let name = &top_token.text;

        if top_token.noexpand {
            self.push_token(top_token);
            return Ok(false);
        }

        // Check for function-based macro first — always expandable
        if let Some(MacroDefinition::Function(handler)) = self.macros.get(name).cloned() {
            self.count_expansion(1)?;
            let tokens = handler(self)?;
            self.stack.extend(tokens);
            return Ok(true);
        }

        let expansion = self.get_expansion(name);
        match expansion {
            None => {
                if expandable_only && name.starts_with('\\') && !self.is_defined(name) {
                    return Err(ParseError::new(
                        format!("Undefined control sequence: {}", name),
                        Some(&top_token),
                    ));
                }
                self.push_token(top_token);
                Ok(false)
            }
            Some(exp) if expandable_only && exp.unexpandable => {
                self.push_token(top_token);
                Ok(false)
            }
            Some(exp) => {
                self.count_expansion(1)?;
                let mut tokens = exp.tokens;
                if exp.num_args > 0 {
                    let args = self.consume_args(exp.num_args)?;
                    tokens = self.substitute_args(tokens, &args);
                }
                self.stack.extend(tokens);
                Ok(true)
            }
        }
    }

    fn substitute_args(&self, mut tokens: Vec<Token>, args: &[Vec<Token>]) -> Vec<Token> {
        let mut i = tokens.len();
        while i > 0 {
            i -= 1;
            if tokens[i].text == "#" && i > 0 {
                let next = &tokens[i - 1];
                if next.text == "#" {
                    tokens.remove(i);
                    i -= 1;
                } else if let Ok(n) = next.text.parse::<usize>() {
                    if n >= 1 && n <= args.len() {
                        tokens.remove(i);
                        tokens.remove(i - 1);
                        let arg_tokens = &args[n - 1];
                        for (j, t) in arg_tokens.iter().enumerate() {
                            tokens.insert(i - 1 + j, t.clone());
                        }
                        i = i.saturating_sub(1);
                    }
                }
            }
        }
        tokens
    }

    fn get_expansion(&self, name: &str) -> Option<MacroExpansion> {
        let def = self.macros.get(name)?;

        if name.len() == 1 {
            let ch = name.chars().next().unwrap();
            let catcode = self.lexer_catcode(ch);
            if catcode != 0 && catcode != 13 {
                return None;
            }
        }

        match def {
            MacroDefinition::Text(text) => {
                let mut num_args = 0;
                let stripped = text.replace("##", "");
                while stripped.contains(&format!("#{}", num_args + 1)) {
                    num_args += 1;
                }
                let mut body_lexer = Lexer::new(text);
                let mut tokens = Vec::new();
                loop {
                    let tok = body_lexer.lex();
                    if tok.is_eof() {
                        break;
                    }
                    tokens.push(tok);
                }
                tokens.reverse();
                Some(MacroExpansion {
                    tokens,
                    num_args,
                    unexpandable: false,
                })
            }
            MacroDefinition::Tokens { tokens, num_args } => Some(MacroExpansion {
                tokens: tokens.clone(),
                num_args: *num_args,
                unexpandable: false,
            }),
            MacroDefinition::Function(_) => {
                // Signal that this is a function macro; handled in expand_once
                Some(MacroExpansion {
                    tokens: vec![],
                    num_args: 0,
                    unexpandable: false,
                })
            }
        }
    }

    fn lexer_catcode(&self, ch: char) -> u8 {
        self.lexer.get_catcode(ch)
    }

    fn count_expansion(&mut self, amount: usize) -> ParseResult<()> {
        self.expansion_count += amount;
        if self.expansion_count > self.max_expand {
            Err(ParseError::msg(
                "Too many expansions: infinite loop or need to increase maxExpand setting",
            ))
        } else {
            Ok(())
        }
    }

    /// Recursively expand the next token until a non-expandable token is found.
    pub fn expand_next_token(&mut self) -> ParseResult<Token> {
        loop {
            let expanded = self.expand_once(false)?;
            if !expanded {
                let mut token = self.stack.pop().unwrap();
                if token.treat_as_relax {
                    token.text = "\\relax".to_string();
                }
                return Ok(token);
            }
        }
    }

    /// Consume a single argument from the token stream.
    pub fn consume_arg(&mut self, delims: Option<&[&str]>) -> ParseResult<ConsumedArg> {
        let is_delimited = delims.is_some_and(|d| !d.is_empty());
        if !is_delimited {
            self.consume_spaces();
        }

        let start = self.future().clone();
        let mut tokens = Vec::new();
        let mut depth: i32 = 0;
        let mut end_tok;

        loop {
            let tok = self.pop_token();
            end_tok = tok.clone();
            tokens.push(tok.clone());

            if tok.text == "{" {
                depth += 1;
            } else if tok.text == "}" {
                depth -= 1;
                if depth == -1 {
                    return Err(ParseError::new("Extra }", Some(&tok)));
                }
            } else if tok.is_eof() {
                return Err(ParseError::new(
                    "Unexpected end of input in a macro argument",
                    Some(&tok),
                ));
            }

            if depth == 0 && !is_delimited {
                break;
            }

            if let Some(delims) = delims {
                if is_delimited && depth == 0 {
                    if let Some(last) = delims.last() {
                        if tok.text == *last {
                            tokens.pop();
                            break;
                        }
                    }
                }
            }
        }

        if start.text == "{" && tokens.last().is_some_and(|t| t.text == "}") {
            tokens.pop();
            tokens.remove(0);
        }

        tokens.reverse();

        Ok(ConsumedArg {
            tokens,
            start,
            end: end_tok,
        })
    }

    /// Consume N arguments.
    fn consume_args(&mut self, num_args: usize) -> ParseResult<Vec<Vec<Token>>> {
        let mut args = Vec::with_capacity(num_args);
        for _ in 0..num_args {
            let arg = self.consume_arg(None)?;
            args.push(arg.tokens);
        }
        Ok(args)
    }

    /// Scan a function argument (optional or mandatory).
    /// Pushes an EOF token to mark the end, then pushes the argument tokens.
    pub fn scan_argument(&mut self, is_optional: bool) -> ParseResult<Option<Token>> {
        if is_optional {
            self.consume_spaces();
            if self.future().text != "[" {
                return Ok(None);
            }
            let start = self.pop_token();
            let arg = self.consume_arg(Some(&["]"]))?;
            let end = &arg.end;
            let end_loc = end.loc.clone();

            self.push_token(Token::new("EOF", end_loc.start, end_loc.end));
            self.push_tokens(arg.tokens);

            let result = Token {
                text: String::new(),
                loc: SourceLocation::range(&start.loc, &end_loc),
                noexpand: false,
                treat_as_relax: false,
            };
            Ok(Some(result))
        } else {
            let arg = self.consume_arg(None)?;
            let end_loc = arg.end.loc.clone();

            self.push_token(Token::new("EOF", end_loc.start, end_loc.end));
            self.push_tokens(arg.tokens);

            let result = Token {
                text: String::new(),
                loc: SourceLocation::range(&arg.start.loc, &end_loc),
                noexpand: false,
                treat_as_relax: false,
            };
            Ok(Some(result))
        }
    }

    /// Check if a command name is currently defined.
    pub fn is_defined(&self, name: &str) -> bool {
        self.macros.has(name)
            || FUNCTIONS.contains_key(name)
            || is_known_symbol(name)
            || IMPLICIT_COMMANDS.contains(&name)
    }

    /// Check if a command is expandable.
    pub fn is_expandable(&self, name: &str) -> bool {
        if let Some(_def) = self.macros.get(name) {
            return true;
        }
        if let Some(func) = FUNCTIONS.get(name) {
            return !func.primitive;
        }
        false
    }
}

pub struct ConsumedArg {
    pub tokens: Vec<Token>,
    pub start: Token,
    pub end: Token,
}

fn handle_newcommand(
    me: &mut MacroExpander,
    exists_ok: bool,
    nonexists_ok: bool,
) -> ParseResult<Vec<Token>> {
    let name_arg = me.consume_arg(None)?;
    // name_arg.tokens is reversed (stack order); last element = first token in original
    let name = name_arg.tokens.last().map_or_else(String::new, |t| t.text.clone());

    let exists = me.is_defined(&name);
    if exists && !exists_ok {
        return Err(ParseError::msg(format!(
            "\\newcommand{{{}}} attempting to redefine {}; use \\renewcommand",
            name, name
        )));
    }
    if !exists && !nonexists_ok {
        return Err(ParseError::msg(format!(
            "\\renewcommand{{{}}} when command {} does not yet exist; use \\newcommand",
            name, name
        )));
    }

    me.consume_spaces();
    let mut num_args = 0usize;
    if me.future().text == "[" {
        me.pop_token();
        let narg_tok = me.pop_token();
        num_args = narg_tok.text.parse().unwrap_or(0);
        let close = me.pop_token();
        if close.text != "]" {
            return Err(ParseError::msg("Expected ] in \\newcommand"));
        }
    }

    let body_arg = me.consume_arg(None)?;
    let tokens = body_arg.tokens;

    me.set_macro(name, MacroDefinition::Tokens { tokens, num_args });
    Ok(vec![])
}

fn is_known_symbol(name: &str) -> bool {
    use ratex_font::symbols;
    symbols::get_symbol(name, symbols::Mode::Math).is_some()
        || symbols::get_symbol(name, symbols::Mode::Text).is_some()
}
