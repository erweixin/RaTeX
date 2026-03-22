#!/usr/bin/env node
/**
 * Reads golden LaTeX lists and writes website/public/data/*.json for the gallery pages.
 *
 * Usage: node tools/generate_website_formulas.mjs
 * Run from repository root.
 */

import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.resolve(__dirname, "..");

const MATH_FILE = path.join(ROOT, "tests/golden/test_cases.txt");
const CE_FILE = path.join(ROOT, "tests/golden/test_case_ce.txt");
const OUT_DIR = path.join(ROOT, "website", "public", "data");

/** Extra physics-flavored formulas (not in mhchem golden); shown after \\pu examples. */
const PHYSICS_CURATED = [
  String.raw`i\hbar\frac{\partial}{\partial t}\Psi = \hat{H}\Psi`,
  String.raw`\nabla \times \mathbf{E} = -\frac{\partial \mathbf{B}}{\partial t}`,
  String.raw`G_{\mu\nu} + \Lambda g_{\mu\nu} = \frac{8\pi G}{c^4} T_{\mu\nu}`,
  String.raw`\oint_{\partial \Sigma} \mathbf{F}\cdot d\mathbf{r} = \iint_{\Sigma}(\nabla\times\mathbf{F})\cdot d\mathbf{S}`,
  String.raw`F = m \frac{d^2 x}{dt^2}`,
  String.raw`E = mc^2`,
  String.raw`E = h\nu`,
  String.raw`pV = nRT`,
  String.raw`\frac{1}{2}mv^2 + \frac{1}{2}kx^2 = \text{const}`,
];

function readLinesNonEmpty(file) {
  const raw = fs.readFileSync(file, "utf8");
  return raw
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l.length > 0 && !l.startsWith("#"));
}

function uniquePush(arr, item) {
  if (!arr.includes(item)) arr.push(item);
}

/**
 * Roughly mirrors the section order on https://katex.org/docs/supported.html
 * (first matching rule wins). Used only for gallery grouping, not semantics.
 */
const SECTION_ORDER = [
  "accents",
  "delimiters",
  "environments",
  "html",
  "letters_unicode",
  "layout",
  "logic_sets",
  "macros",
  "operators",
  "relations",
  "negated_relations",
  "arrows",
  "extensible_arrows",
  "special_notation",
  "style_font",
  "symbols_punctuation",
  "spacing",
  "other",
];

const SECTION_META = {
  accents: { title: "重音与上下划线", titleEn: "Accents" },
  delimiters: { title: "定界符", titleEn: "Delimiters" },
  environments: { title: "环境", titleEn: "Environments" },
  html: { title: "HTML", titleEn: "HTML" },
  letters_unicode: { title: "字母与 Unicode", titleEn: "Letters and Unicode" },
  layout: { title: "版式", titleEn: "Layout" },
  logic_sets: { title: "逻辑与集合", titleEn: "Logic and Set Theory" },
  macros: { title: "宏", titleEn: "Macros" },
  operators: { title: "运算符（含分式、根式、大型算子）", titleEn: "Operators" },
  relations: { title: "关系", titleEn: "Relations" },
  negated_relations: { title: "否定关系", titleEn: "Negated Relations" },
  arrows: { title: "箭头", titleEn: "Arrows" },
  extensible_arrows: { title: "可伸缩箭头", titleEn: "Extensible Arrows" },
  special_notation: { title: "特殊记号（bra-ket）", titleEn: "Special Notation" },
  style_font: { title: "样式、颜色与字体", titleEn: "Style, Color, Size, and Font" },
  symbols_punctuation: { title: "符号与标点", titleEn: "Symbols and Punctuation" },
  spacing: { title: "间距", titleEn: "Spacing" },
  other: { title: "其他", titleEn: "Other" },
};

function categorizeMath(latex) {
  const s = latex;

  const rules = [
    [/\\(?:href|url|includegraphics|html(?:Class|Data|Id|Style))\b/, "html"],
    [/\\begin\{/, "environments"],
    [
      /\\(?:gdef|def|edef|xdef|let|providecommand|renewcommand|newcommand|global\s*\\(?:def|let|edef))\b/,
      "macros",
    ],
    [
      /\\(?:left|right|middle|bigl?|bigr?|Bigl?|Bigr?|biggl?|biggr?|Biggl?|Biggr?|bigm|Bigm|biggm|Biggm)\b/,
      "delimiters",
    ],
    [
      /\\(?:lbrace|rbrace|lbrack|rbrack|langle|rangle|lceil|rceil|lfloor|rfloor|lvert|rvert|lVert|rVert|lgroup|rgroup|lmoustache|rmoustache|ulcorner|urcorner|llcorner|lrcorner|llbracket|rrbracket|lBrace|rBrace|lang|rang|lparen|rparen|vert|Vert)\b/,
      "delimiters",
    ],
    [
      /\\x(?:left|right|Left|Right|hook|mapsto|longequal|leftrightarrow|Leftrightarrow|leftharpoon|rightharpoon|twohead|tofrom|leftrightharpoons|rightleftharpoons|Rightarrow|Leftarrow)\b/,
      "extensible_arrows",
    ],
    [
      /\\(?:n(?:eq|leq|geq|geqq|leqq|less|gtr|mid|parallel|in|i|subset|supset|succ|prec|preceq|succeq|VDash|Vdash|vDash|vdash|exists|Rightarrow|rightarrow|Leftarrow|leftarrow|leftrightarrow|Leftrightarrow|sim|cong|triangle|subseteq|supseteq|shortmid|shortparallel|gtrless|leqslant|geqslant|leqq|geqq|triangleleft|triangleright|trianglelefteq|trianglerighteq|prec|succ|preceq|succeq|vdash|Vdash|vDash|VDash)|notin|neq|ne)\b|\\not\s|=|\\not=/,
      "negated_relations",
    ],
    [
      /\\(?:leftarrow|rightarrow|Leftarrow|Rightarrow|leftrightarrow|Leftrightarrow|mapsto|longmapsto|longleftarrow|longrightarrow|implies|iff|impliedby|uparrow|Uparrow|downarrow|Downarrow|Updownarrow|updownarrow|nearrow|nwarrow|searrow|swarrow|hookleftarrow|hookrightarrow|twoheadrightarrow|twoheadleftarrow|rightharpoonup|rightharpoondown|leftharpoonup|leftharpoondown|rightleftharpoons|leftrightharpoons|rightsquigarrow|leadsto|restriction|upharpoonleft|upharpoonright|downharpoonleft|downharpoonright|upuparrows|downdownarrows|Lleftarrow|RRightarrow|dashleftarrow|dashrightarrow|circlearrowleft|circlearrowright|curvearrowleft|curvearrowright|leftrightarrows|leftleftarrows|rightrightarrows|looparrowleft|looparrowright|leftrightsquigarrow|Lsh|Rsh|to|gets|Harr|hArr|harr|Larr|lArr|larr|Rarr|rArr|rarr|lrArr|lrarr|Lrarr)\b/,
      "arrows",
    ],
    [
      /\\(?:acute|bar|breve|check|ddot|dddot|ddddot|dot|grave|hat|mathring|tilde|vec|widehat|widetilde|overbrace|underbrace|overline|underline|overrightarrow|overleftarrow|Overrightarrow|underrightarrow|underleftarrow|overleftrightarrow|underleftrightarrow|overgroup|undergroup|overlinesegment|underlinesegment|utilde|widecheck|underbar)\b/,
      "accents",
    ],
    [/\\(?:bra|ket|Bra|Ket|Braket|braket)\b/, "special_notation"],
    [
      /\\(?:forall|exists|exist|nexists|land|lor|lnot|neg|complement|therefore|because|backepsilon)\b/,
      "logic_sets",
    ],
    [/\\Set\b|\\set\b/, "logic_sets"],
    [
      /\\(?:approx|approxeq|equiv|leq|geq|leqq|geqq|leqslant|geqslant|subset|supset|subseteq|supseteq|Subset|Supset|subseteqq|supseteqq|in|isin|sim|simeq|models|perp|parallel|shortparallel|propto|cong|doteq|Doteq|doteqdot|ll|gg|ggg|gggtr|prec|succ|preceq|succeq|bowtie|Join|between|asymp|bumpeq|Bumpeq|circeq|curlyeqprec|curlyeqsucc|eqcirc|eqcolon|Eqcolon|eqsim|fallingdotseq|risingdotseq|triangle|vDash|vdash|dashv|smile|frown|smallsmile|smallfrown|thicksim|thickapprox|lessdot|gtrdot|lessgtr|lesseqgtr|gtreqless|vartriangle|sqsubset|sqsupset|sqsubseteq|sqsupseteq|coloneq|Coloneq|coloneqq|Coloneqq|eqqcolon|Eqqcolon|colonapprox|Colonapprox|colonsim|Colonsim|dblcolon|eqslantgtr|eqslantless|gtreqqless|lesseqqgtr|origof|imageof|multimap|owns|ni|backepsilon|mid|shortmid)\b/,
      "relations",
    ],
    [
      /\\(?:sqrt|sum|prod|coprod|int|iint|iiint|oint|oiint|oiiint|bigcup|bigcap|bigsqcup|bigvee|bigwedge|bigodot|bigoplus|bigotimes|biguplus|smallint|intop)\b/,
      "operators",
    ],
    [
      /\\(?:frac|tfrac|dfrac|cfrac|binom|dbinom|tbinom|choose|genfrac|over|above|brace|brack)\b/,
      "operators",
    ],
    [
      /\\(?:arcsin|arccos|arctan|arctg|arcctg|sin|cos|tan|cot|sec|csc|sinh|cosh|tanh|coth|ln|log|exp|lim|liminf|limsup|sup|inf|max|min|det|gcd|hom|ker|dim|Pr|arg|deg|lg|cosec|cotg|ctg|cth|sh|th|tg|injlim|projlim|plim|varinjlim|varprojlim|varliminf|varlimsup|operatorname|operatornamewithlimits)\b/,
      "operators",
    ],
    [
      /\\(?:cdot|cdotp|times|div|pm|mp|ast|star|circ|bullet|cap|cup|vee|wedge|oplus|ominus|otimes|oslash|odot|dagger|ddagger|amalg|diamond|bigtriangleup|bigtriangledown|triangleleft|triangleright|unlhd|unrhd|lhd|rhd|setminus|smallsetminus|wr|And|doublecap|doublecup|barwedge|veebar|curlyvee|curlywedge|centerdot|intercal|leftthreetimes|rightthreetimes|ltimes|rtimes|ast|circledast|circledcirc|circleddash|bigcirc|uplus|sqcap|sqcup|ominus|oslash|odot|mp|pm|cdot|div|times)\b/,
      "operators",
    ],
    [
      /\\(?:stackrel|overset|underset|atop|raisebox|vcenter|mathclap|mathllap|mathrlap|llap|rlap|clap|cancel|bcancel|xcancel|sout|boxed|phase|tag|substack|mathchoice|operatorname\*)\b/,
      "layout",
    ],
    [
      /\\[!,;:]|\b\\quad\b|\b\\qquad\b|\b\\enspace\b|\b\\kern\b|\b\\mkern\b|\b\\mskip\b|\b\\hskip\b|\b\\hspace\b|\b\\phantom\b|\b\\hphantom\b|\b\\vphantom\b|\b\\mathstrut\b|\b\\thinspace\b|\b\\medspace\b|\b\\thickspace\b|\b\\nobreakspace\b|\b\\space\b|\b\\negthinspace\b|\b\\negmedspace\b|\b\\negthickspace\b/,
      "spacing",
    ],
    [
      /\\(?:color|textcolor|colorbox|fcolorbox|mathbf|bf|mathrm|mathsf|textsf|mathtt|texttt|mathcal|mathbb|mathfrak|frak|mathscr|boldsymbol|bold|bm|mathit|it|rm|sf|tt|text|textit|textrm|textsf|texttt|textup|emph|Huge|huge|LARGE|Large|large|normalsize|small|footnotesize|scriptsize|tiny|scriptscriptstyle|scriptstyle|textstyle|displaystyle|mathnormal|textnormal|mathsfit|mathbin|mathclose|mathinner|mathopen|mathop|mathord|mathpunct|mathrel|verb|TextOrMath|fbox)\b/,
      "style_font",
    ],
    [
      /\\(?:cdots|ldots|dots|dotsb|dotsc|dotsm|dotsi|dotso|mathellipsis|infty|infin|angle|measuredangle|sphericalangle|degree|prime|backprime|nabla|partial|ell|hbar|hslash|wp|weierp|Im|Re|image|real|aleph|beth|gimel|daleth|Game|Finv|empty|emptyset|varnothing|Box|square|blacksquare|triangle|triangledown|diamond|Diamond|lozenge|blacklozenge|clubsuit|diamondsuit|heartsuit|spadesuit|clubs|diamonds|hearts|spades|flat|natural|sharp|checkmark|surd|top|bot|vdots|ddots|ddag|dag|maltese|yen|pounds|mathsterling|copyright|circledR|circledS|minuso|diagup|diagdown|minuscolon|minuscoloncolon|ratio|vcentcolon|colon|Colon|colonsim|Colonapprox|cdots|ldots|vdots|ddots|TeX|LaTeX|KaTeX)\b/,
      "symbols_punctuation",
    ],
    [
      /\\(?:alpha|beta|gamma|delta|epsilon|varepsilon|zeta|eta|theta|vartheta|vartheta|iota|kappa|varkappa|lambda|mu|nu|xi|pi|varpi|rho|varrho|sigma|varsigma|tau|upsilon|phi|varphi|chi|psi|omega|Gamma|Delta|Theta|Lambda|Xi|Pi|Sigma|Upsilon|Phi|Psi|Omega|Alpha|Beta|Epsilon|Zeta|Eta|Iota|Kappa|Mu|Nu|Omicron|Rho|Tau|Chi|omicron|digamma|thetasym|varGamma|varDelta|varTheta|varLambda|varXi|varPi|varSigma|varUpsilon|varPhi|varPsi|varOmega|varpi|varrho|varsigma|vartheta|N|Z|R|C|cnums|Reals|reals|Complex|natnums|Bbbk)\b/,
      "letters_unicode",
    ],
    [/[^\x00-\x7F]/, "letters_unicode"],
    [/^[^\\]*[_^][^\\]*$/, "layout"],
  ];

  for (const [re, id] of rules) {
    if (re.test(s)) return id;
  }
  return "other";
}

function buildMathSections(mathLines) {
  /** @type {Record<string, string[]>} */
  const buckets = {};
  for (const id of SECTION_ORDER) buckets[id] = [];

  for (const line of mathLines) {
    const cat = categorizeMath(line);
    (buckets[cat] || buckets.other).push(line);
  }

  return SECTION_ORDER.filter((id) => (buckets[id] || []).length > 0).map((id) => ({
    id,
    title: SECTION_META[id].title,
    titleEn: SECTION_META[id].titleEn,
    formulas: buckets[id],
  }));
}

function main() {
  const math = readLinesNonEmpty(MATH_FILE);

  const ceLines = readLinesNonEmpty(CE_FILE);
  const chemistry = [];
  const physics = [];

  for (const line of ceLines) {
    if (line.includes("\\ce")) uniquePush(chemistry, line);
    if (line.includes("\\pu")) uniquePush(physics, line);
  }

  for (const f of PHYSICS_CURATED) uniquePush(physics, f);

  fs.mkdirSync(OUT_DIR, { recursive: true });

  const mathSections = buildMathSections(math);
  fs.writeFileSync(
    path.join(OUT_DIR, "math.json"),
    JSON.stringify(
      { count: math.length, formulas: math, sections: mathSections },
      null,
      0
    ),
    "utf8"
  );
  fs.writeFileSync(
    path.join(OUT_DIR, "chemistry.json"),
    JSON.stringify({ count: chemistry.length, formulas: chemistry }, null, 0),
    "utf8"
  );
  fs.writeFileSync(
    path.join(OUT_DIR, "physics.json"),
    JSON.stringify({ count: physics.length, formulas: physics }, null, 0),
    "utf8"
  );

  console.log(
    `Wrote website/public/data: math=${math.length} (${mathSections.length} sections), chemistry=${chemistry.length}, physics=${physics.length}`
  );
}

main();
