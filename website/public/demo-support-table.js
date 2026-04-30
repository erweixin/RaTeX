/** WASM entry: flat /platforms/web for /demo/ & /zh/ when base unset; else legacy subdirectory heuristic (see gallery.js). */
function ratexWasmModuleUrl() {
  var g = typeof globalThis !== "undefined" ? globalThis : window;
  function getSiteDirUrl() {
    var u = new URL(location.href);
    var path = u.pathname;
    if (!path.endsWith("/")) {
      var last = path.split("/").pop() || "";
      if (last.indexOf(".") !== -1) {
        path = path.replace(/\/[^/]+$/, "/");
      } else {
        path = path + "/";
      }
    }
    u.pathname = path || "/";
    return u;
  }
  if (typeof g.__RATES_WASM_IMPORT_URL__ === "string" && g.__RATES_WASM_IMPORT_URL__.length > 0) {
    return g.__RATES_WASM_IMPORT_URL__;
  }
  var configured = typeof g.__RATEX_SITE_BASE__ === "string" ? g.__RATEX_SITE_BASE__ : "";
  if (configured.length > 0) {
    var b = configured.endsWith("/") ? configured : configured + "/";
    return new URL("platforms/web/pkg/ratex_wasm.js", new URL(b, location.origin)).href;
  }
  var path = location.pathname || "";
  if (path.indexOf("/website/") !== -1) {
    return new URL("../platforms/web/pkg/ratex_wasm.js", location.href).href;
  }
  if (path.startsWith("/RaTeX/") || path === "/RaTeX") {
    return new URL("platforms/web/pkg/ratex_wasm.js", new URL("/RaTeX/", location.origin)).href;
  }
  if (location.protocol === "file:") {
    return new URL("platforms/web/pkg/ratex_wasm.js", getSiteDirUrl()).href;
  }
  if (/^\/demo(\/|$)/.test(path) || /^\/zh(\/|$)/.test(path)) {
    return new URL("/platforms/web/pkg/ratex_wasm.js", location.origin).href;
  }
  return new URL("platforms/web/pkg/ratex_wasm.js", getSiteDirUrl()).href;
}

const FORMULAS = [
  "n!",
  "a\\!b",
  "\\def\\sqr#1{#1^2} \\sqr{y}",
  "\\#",
  "\\%",
  "\\begin{matrix} a & b \\\\ c & d \\end{matrix}",
  "\\text{\\'{a}}",
  "\\text{\\(\\frac a b\\)}",
  "a\\ b",
  "\\text{\\\"{a}}",
  "a\\,\\,{b}",
  "\\text{\\.{a}}",
  "a\\:\\:{b}",
  "\\;\\;{b}",
  "x_i",
  "\\text{\\={a}}",
  "a\\>\\>{b}",
  "{a}",
  "\\text{no~no~no~breaks}",
  "\\text{\\~{a}}",
  "x^i",
  "\\text{\\^{a}}",
  "\\text{\\textdollar}",
  "\\text{\\`{a}}",
  "\\text{\\AA}",
  "\\text{\\aa}",
  "{a \\above{2pt} b+1}",
  "\\acute e",
  "\\text{\\AE}",
  "\\text{\\ae}",
  "\\alef",
  "\\alefsym",
  "\\aleph",
  "\\begin{align*} a&=b+c \\\\ d+e&=f \\end{align*}",
  "\\begin{align*} a&=b+c \\\\ d+e&=f \\end{align*}",
  "\\begin{aligned} a&=b+c \\\\ d+e&=f \\end{aligned}",
  "\\begin{alignat*}{2} 10&x+ &3&y = 2 \\\\ 3&x+&13&y = 4 \\end{alignat*}",
  "\\begin{alignat*}{2} 10&x+ &3&y = 2 \\\\ 3&x+&13&y = 4 \\end{alignat*}",
  "\\begin{alignedat}{2} 10&x+ &3&y = 2 \\\\ 3&x+&13&y = 4 \\end{alignedat}",
  "\\Alpha",
  "\\alpha",
  "\\amalg",
  "\\And",
  "a_{\\angl n}",
  "\\angln",
  "\\allowbreak",
  "\\angle",
  "\\approx",
  "\\approxeq",
  "\\approxcolon",
  "\\approxcoloncolon",
  "\\arccos",
  "\\arcctg",
  "\\arcsin",
  "\\arctan",
  "\\arctg",
  "\\arg",
  "\\argmax",
  "\\argmin",
  "\\begin{array}{cc} a & b \\\\ c & d \\end{array}",
  "\\def\\arraystretch{1.5} \\begin{array}{cc} a & b \\\\ c & d \\end{array}",
  "\\ast",
  "\\asymp",
  "{a \\atop b}",
  "\\backepsilon",
  "\\backprime",
  "\\backsim",
  "\\backsimeq",
  "\\backslash",
  "\\bar{y}",
  "\\barwedge",
  "\\Bbb{ABC}",
  "\\Bbbk",
  "\\bcancel{5}",
  "\\because",
  "\\Beta",
  "\\beta",
  "\\beth",
  "\\between",
  "\\begingroup a \\endgroup",
  "\\bf AaBb12",
  "\\big(\\big)",
  "\\Big(\\Big)",
  "\\bigcap",
  "\\bigcirc",
  "\\bigcup",
  "\\bigg(\\bigg)",
  "\\Bigg(\\Bigg)",
  "\\biggl(",
  "\\Biggl(",
  "\\biggm\\vert",
  "\\Biggm\\vert",
  "\\biggr)",
  "\\Biggr)",
  "\\bigl(",
  "\\Bigl(",
  "\\bigm\\vert",
  "\\Bigm\\vert",
  "\\bigodot",
  "\\bigoplus",
  "\\bigotimes",
  "\\bigr)",
  "\\Bigr)",
  "\\bigsqcup",
  "\\bigstar",
  "\\bigtriangledown",
  "\\bigtriangleup",
  "\\biguplus",
  "\\bigvee",
  "\\bigwedge",
  "\\binom n k",
  "\\blacklozenge",
  "\\blacksquare",
  "\\blacktriangle",
  "\\blacktriangledown",
  "\\blacktriangleleft",
  "\\blacktriangleright",
  "\\bm{AaBb}",
  "\\begin{Bmatrix} a & b \\\\ c & d \\end{Bmatrix}",
  "\\begin{Bmatrix*}[r] 0 & -1 \\\\ -1 & 0 \\end{Bmatrix*}",
  "\\begin{bmatrix} a & b \\\\ c & d \\end{bmatrix}",
  "\\begin{bmatrix*}[r] 0 & -1 \\\\ -1 & 0 \\end{bmatrix*}",
  "a \\bmod b",
  "\\bold{AaBb123}",
  "\\boldsymbol{AaBb}",
  "\\bot",
  "\\bowtie",
  "\\Box",
  "\\boxdot",
  "\\boxed{ab}",
  "\\boxminus",
  "\\boxplus",
  "\\boxtimes",
  "\\Bra{\\psi}",
  "\\bra{\\psi}",
  "\\braket{\\phi|\\psi}",
  "\\Braket{a|\\frac{\\partial^2}{\\partial t^2}|b}",
  "{n\\brace k}",
  "{n\\brack k}",
  "\\breve{eu}",
  "\\bull",
  "\\bullet",
  "\\Bumpeq",
  "\\bumpeq",
  "\\cal AaBb123",
  "\\cancel{5}",
  "\\Cap",
  "\\cap",
  "\\begin{cases} a &\\text{if } b \\\\ c &\\text{if } d \\end{cases}",
  "\\begin{CD} A @>a>> B \\\\ @VbVV @AAcA \\\\ C @= D \\end{CD}",
  "\\cdot",
  "\\cdotp",
  "\\cdots",
  "\\ce{C6H5-CHO}",
  "a\\centerdot b",
  "\\cfrac{2}{1+\\cfrac{2}{1+\\cfrac{2}{1}}}",
  "\\char\"263a",
  "\\check{oe}",
  "\\ch",
  "\\checkmark",
  "\\Chi",
  "\\chi",
  "{n+1 \\choose k+2}",
  "\\circ",
  "\\circeq",
  "\\circlearrowleft",
  "\\circlearrowright",
  "\\circledast",
  "\\circledcirc",
  "\\circleddash",
  "\\circledR",
  "\\circledS",
  "\\clubs",
  "\\clubsuit",
  "\\cnums",
  "\\colon",
  "\\Colonapprox",
  "\\colonapprox",
  "\\coloncolon",
  "\\coloncolonapprox",
  "\\coloncolonequals",
  "\\coloncolonminus",
  "\\coloncolonsim",
  "\\Coloneq",
  "\\coloneq",
  "\\colonequals",
  "\\Coloneqq",
  "\\coloneqq",
  "\\colonminus",
  "\\Colonsim",
  "\\colonsim",
  "\\color{#0000FF} AaBb123",
  "\\colorbox{red}{Black on red}",
  "\\complement",
  "\\Complex",
  "\\cong",
  "\\coprod",
  "\\copyright",
  "\\cos",
  "\\cosec",
  "\\cosh",
  "\\cot",
  "\\cotg",
  "\\coth",
  "\\begin{matrix} a & b \\cr c & d \\end{matrix}",
  "\\csc",
  "\\ctg",
  "\\cth",
  "\\Cup",
  "\\cup",
  "\\curlyeqprec",
  "\\curlyeqsucc",
  "\\curlyvee",
  "\\curlywedge",
  "\\curvearrowleft",
  "\\curvearrowright",
  "\\dag",
  "\\Dagger",
  "\\dagger",
  "\\daleth",
  "\\Darr",
  "\\dArr",
  "\\darr",
  "\\begin{darray}{cc} a & b \\\\ c & d \\end{darray}",
  "\\dashleftarrow",
  "\\dashrightarrow",
  "\\dashv",
  "\\dbinom n k",
  "\\dblcolon",
  "\\begin{dcases} a &\\text{if } b \\\\ c &\\text{if } d \\end{dcases}",
  "\\ddag",
  "\\ddagger",
  "\\ddddot x",
  "\\dddot x",
  "\\ddot x",
  "\\ddots",
  "\\def\\foo{x^2} \\foo + \\foo",
  "\\deg",
  "\\degree",
  "\\delta",
  "\\Delta",
  "\\det",
  "\\digamma",
  "\\dfrac{a-1}{b-1}",
  "\\diagdown",
  "\\diagup",
  "\\Diamond",
  "\\diamond",
  "\\diamonds",
  "\\diamondsuit",
  "\\dim",
  "\\displaystyle\\sum_0^n",
  "\\div",
  "\\divideontimes",
  "\\dot x",
  "\\Doteq",
  "\\doteq",
  "\\doteqdot",
  "\\dotplus",
  "x_1 + \\dots + x_n",
  "x_1 +\\dotsb + x_n",
  "x,\\dotsc,y",
  "\\int_{A_1}\\int_{A_2}\\dotsi",
  "x_1 x_2 \\dotsm x_n",
  "\\dotso",
  "\\doublebarwedge",
  "\\doublecap",
  "\\doublecup",
  "\\Downarrow",
  "\\downarrow",
  "\\downdownarrows",
  "\\downharpoonleft",
  "\\downharpoonright",
  "\\begin{drcases} a &\\text{if } b \\\\ c &\\text{if } d \\end{drcases}",
  "\\def\\foo{a}\\edef\\fcopy{\\foo}\\def\\foo{}\\fcopy",
  "\\ell",
  "\\emph{nested \\emph{emphasis}}",
  "\\empty",
  "\\emptyset",
  "a\\enspace b",
  "\\Epsilon",
  "\\epsilon",
  "\\eqcirc",
  "\\Eqcolon",
  "\\eqcolon",
  "\\begin{equation*} a = b + c \\end{equation*}",
  "\\begin{equation*} a = b + c \\end{equation*}",
  "\\Eqqcolon",
  "\\eqqcolon",
  "\\eqsim",
  "\\eqslantgtr",
  "\\eqslantless",
  "\\equalscolon",
  "\\equalscoloncolon",
  "\\equiv",
  "\\Eta",
  "\\eta",
  "\\eth",
  "\\exist",
  "\\exists",
  "\\exp",
  "\\fallingdotseq",
  "\\fbox{Hi there!}",
  "\\fcolorbox{red}{aqua}{A}",
  "\\Finv",
  "\\flat",
  "\\footnotesize footnotesize",
  "\\forall",
  "\\frac a b",
  "\\frak{AaBb}",
  "\\frown",
  "\\Game",
  "\\Gamma",
  "\\gamma",
  "\\begin{gather*} a=b \\\\ e=b+c \\end{gather*}",
  "\\begin{gathered} a=b \\\\ e=b+c \\end{gathered}",
  "\\gcd",
  "\\gdef\\sqr#1{#1^2} \\sqr{y} + \\sqr{y}",
  "\\gdef\\VERT{|} \\braket{\\phi\\VERT\\psi}",
  "\\ge",
  "\\genfrac ( ] {2pt}{0}a{a+1}",
  "\\geq",
  "\\geqq",
  "\\geqslant",
  "\\gets",
  "\\gg",
  "\\ggg",
  "\\gggtr",
  "\\gimel",
  "\\global\\def\\add#1#2{#1+#2} \\add 2 3",
  "\\gnapprox",
  "\\gneq",
  "\\gneqq",
  "\\gnsim",
  "\\grave{eu}",
  "a \\gt b",
  "\\gtrdot",
  "\\gtrapprox",
  "\\gtreqless",
  "\\gtreqqless",
  "\\gtrless",
  "\\gtrsim",
  "\\gvertneqq",
  "\\text{\\H{a}}",
  "\\Harr",
  "\\hArr",
  "\\harr",
  "\\hat{\\theta}",
  "\\hbar",
  "\\hbox{$x^2$}",
  "\\begin{matrix} a & b \\\\ \\hdashline c & d \\end{matrix}",
  "\\hearts",
  "\\heartsuit",
  "\\begin{matrix} a & b \\\\ \\hline c & d \\end{matrix}",
  "\\hom",
  "\\hookleftarrow",
  "\\hookrightarrow",
  "a\\hphantom{bc}d",
  "\\href{https://katex.org/}{\\KaTeX}",
  "w\\hskip1em i\\hskip2em d",
  "\\hslash",
  "s\\hspace{7ex} k",
  "\\htmlClass{foo}{x}",
  "\\htmlData{foo=a, bar=b}{x}",
  "\\htmlId{bar}{x}",
  "\\htmlStyle{color: red;}{x}",
  "\\huge huge",
  "\\Huge Huge",
  "\\text{\\i}",
  "A\\iff B",
  "\\iiint",
  "\\iint",
  "\\Im",
  "\\image",
  "\\imageof",
  "\\imath",
  "P\\impliedby Q",
  "P\\implies Q",
  "\\in",
  "\\includegraphics[height=0.8em, totalheight=0.9em, width=0.9em, alt=KA logo]{https://cdn.kastatic.org/images/apple-touch-icon-57x57-precomposed.new.png}",
  "\\inf",
  "\\infin",
  "\\infty",
  "\\injlim",
  "\\int",
  "\\intercal",
  "\\intop",
  "\\Iota",
  "\\iota",
  "\\isin",
  "{\\it AaBb}",
  "\\text{\\j}",
  "\\jmath",
  "\\Join",
  "\\Kappa",
  "\\kappa",
  "\\KaTeX",
  "\\ker",
  "I\\kern-2.5pt R",
  "\\Ket{\\psi}",
  "\\ket{\\psi}",
  "\\Lambda",
  "\\lambda",
  "\\land",
  "\\lang A\\rangle",
  "\\langle A\\rangle",
  "\\Larr",
  "\\lArr",
  "\\larr",
  "\\large large",
  "\\Large Large",
  "\\LARGE LARGE",
  "\\LaTeX",
  "\\lBrace",
  "\\lbrace",
  "\\lbrack",
  "\\lceil",
  "\\ldotp",
  "\\ldots",
  "\\le",
  "\\leadsto",
  "\\left\\lbrace \\dfrac ab \\right.",
  "\\leftarrow",
  "\\Leftarrow",
  "\\leftarrowtail",
  "\\leftharpoondown",
  "\\leftharpoonup",
  "\\leftleftarrows",
  "\\Leftrightarrow",
  "\\leftrightarrow",
  "\\leftrightarrows",
  "\\leftrightharpoons",
  "\\leftrightsquigarrow",
  "\\leftthreetimes",
  "\\leq",
  "\\leqq",
  "\\leqslant",
  "\\lessapprox",
  "\\lessdot",
  "\\lesseqgtr",
  "\\lesseqqgtr",
  "\\lessgtr",
  "\\lesssim",
  "\\lfloor",
  "\\lg",
  "\\lgroup",
  "\\lhd",
  "\\lim",
  "\\liminf",
  "\\lim\\limits_x",
  "\\limsup",
  "\\ll",
  "{=}\\llap{/\\,}",
  "\\llbracket",
  "\\llcorner",
  "\\Lleftarrow",
  "\\lll",
  "\\llless",
  "\\lmoustache",
  "\\ln",
  "\\lnapprox",
  "\\lneq",
  "\\lneqq",
  "\\lnot",
  "\\lnsim",
  "\\log",
  "\\Longleftarrow",
  "\\longleftarrow",
  "\\Longleftrightarrow",
  "\\longleftrightarrow",
  "\\longmapsto",
  "\\Longrightarrow",
  "\\longrightarrow",
  "\\looparrowleft",
  "\\looparrowright",
  "\\lor",
  "\\lozenge",
  "\\lparen",
  "\\Lrarr",
  "\\lrArr",
  "\\lrarr",
  "\\lrcorner",
  "\\lq",
  "\\Lsh",
  "\\lt",
  "\\ltimes",
  "\\lVert",
  "\\lvert",
  "\\lvertneqq",
  "\\maltese",
  "\\mapsto",
  "\\mathbb{AB}",
  "\\mathbf{AaBb123}",
  "a\\mathbin{!}b",
  "\\mathcal{AaBb123}",
  "a\\mathchoice{\\,}{\\,\\,}{\\,\\,\\,}{\\,\\,\\,\\,}b",
  "\\sum_{\\mathclap{1\\le i\\le n}} x_{i}",
  "a + (b\\mathclose\\gt + c",
  "\\mathellipsis",
  "\\mathfrak{AaBb}",
  "ab\\mathinner{\\text{inside}}cd",
  "\\mathit{AaBb}",
  "{=}\\mathllap{/\\,}",
  "\\mathnormal{AaBb}",
  "\\mathop{\\star}_a^b",
  "\\{x\u2208\u211d\\mid x>0\\}",
  "P\\left(A\\middle\\vert B\\right)",
  "\\min",
  "\\minuscolon",
  "\\minuscoloncolon",
  "\\minuso",
  "a\\mkern18mu b",
  "3\\equiv 5 \\mod 2",
  "\\models",
  "\\mp",
  "a\\mskip{10mu}b",
  "\\Mu",
  "\\mu",
  "\\multimap",
  "\\N",
  "\\nabla",
  "\\natnums",
  "\\natural",
  "a\\negmedspace b",
  "\\ncong",
  "\\ne",
  "\\nearrow",
  "\\neg",
  "a\\negthickspace b",
  "a\\negthinspace b",
  "\\neq",
  "\\newcommand\\chk{\\checkmark} \\chk",
  "a\\newline b",
  "\\nexists",
  "\\ngeq",
  "\\ngeqq",
  "\\ngeqslant",
  "\\ngtr",
  "\\ni",
  "\\nleftarrow",
  "\\nLeftarrow",
  "\\nLeftrightarrow",
  "\\nleftrightarrow",
  "\\nleq",
  "\\nleqq",
  "\\nleqslant",
  "\\nless",
  "\\nmid",
  "a\\nobreakspace b",
  "\\lim\\nolimits_x",
  "\\begin{align*} a&=b+c \\\\ d+e&=f \\end{align*}",
  "\\normalsize normalsize",
  "\\not =",
  "\\begin{align*} a&=b+c \\\\ d+e&=f \\end{align*}",
  "\\notin",
  "\\notni",
  "\\nparallel",
  "\\nprec",
  "\\npreceq",
  "\\nRightarrow",
  "\\nrightarrow",
  "\\nshortmid",
  "\\nshortparallel",
  "\\nsim",
  "\\nsubseteq",
  "\\nsubseteqq",
  "\\nsucc",
  "\\nsucceq",
  "\\nsupseteq",
  "\\nsupseteqq",
  "\\ntriangleleft",
  "\\ntrianglelefteq",
  "\\ntriangleright",
  "\\ntrianglerighteq",
  "\\Nu",
  "\\nu",
  "\\nVDash",
  "\\nVdash",
  "\\nvDash",
  "\\nvdash",
  "\\nwarrow",
  "\\text{\\O}",
  "\\text{\\o}",
  "\\odot",
  "\\text{\\OE}",
  "\\text{\\oe}",
  "\\oiiint",
  "\\oiint",
  "\\oint",
  "\\omega",
  "\\Omega",
  "\\Omicron",
  "\\omicron",
  "\\ominus",
  "\\operatorname{asin} x",
  "\\operatorname*{asin}\\limits_y x",
  "\\operatornamewithlimits{asin}\\limits_y x",
  "\\oplus",
  "\\origof",
  "\\oslash",
  "\\otimes",
  "{a+1 \\over b+2}+c",
  "\\overbrace{x+\u22ef+x}^{n\\text{ times}}",
  "\\overgroup{AB}",
  "\\overleftarrow{AB}",
  "\\overleftharpoon{AB}",
  "\\overleftrightarrow{AB}",
  "\\overline{\\text{a long argument}}",
  "\\overlinesegment{AB}",
  "\\Overrightarrow{AB}",
  "\\overrightarrow{AB}",
  "\\overrightharpoon{ac}",
  "\\overset{!}{=}",
  "\\owns",
  "\\text{\\P}",
  "\\parallel",
  "\\partial",
  "\\perp",
  "\\Gamma^{\\phantom{i}j}_{i\\phantom{j}k}",
  "\\phase{-78^\\circ}",
  "\\Phi",
  "\\phi",
  "\\Pi",
  "\\pi",
  "\\pitchfork",
  "\\plim",
  "\\plusmn",
  "\\pm",
  "\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}",
  "\\begin{pmatrix*}[r] 0 & -1 \\\\ -1 & 0 \\end{pmatrix*}",
  "\\pmb{\\mu}",
  "x\\pmod a",
  "x \\pod a",
  "\\pounds",
  "\\Pr",
  "\\prec",
  "\\precapprox",
  "\\preccurlyeq",
  "\\preceq",
  "\\precnapprox",
  "\\precneqq",
  "\\precnsim",
  "\\precsim",
  "\\prime",
  "\\prod",
  "\\projlim",
  "\\propto",
  "\\providecommand\\greet{\\text{Hello}} \\greet",
  "\\psi",
  "\\Psi",
  "\\pu{123 kJ//mol}",
  "a\\qquad\\qquad{b}",
  "a\\quad\\quad{b}",
  "\\R",
  "\\text{\\r{a}}",
  "h\\raisebox{2pt}{ighe}r",
  "\\langle A\\rang",
  "\\Rarr",
  "\\rArr",
  "\\rarr",
  "\\ratio",
  "\\rBrace",
  "\\rbrace",
  "\\rbrack",
  "\\begin{rcases} a &\\text{if } b \\\\ c &\\text{if } d \\end{rcases}",
  "\\rceil",
  "\\Re",
  "\\real",
  "\\Reals",
  "\\reals",
  "\\def\\hail{Hi!} \\renewcommand\\hail{\\text{Ahoy!}} \\hail",
  "\\restriction",
  "\\rfloor",
  "\\rgroup",
  "\\rhd",
  "\\Rho",
  "\\rho",
  "\\left.\\dfrac a b\\right)",
  "\\Rightarrow",
  "\\rightarrow",
  "\\rightarrowtail",
  "\\rightharpoondown",
  "\\rightharpoonup",
  "\\rightleftarrows",
  "\\rightleftharpoons",
  "\\rightrightarrows",
  "\\rightsquigarrow",
  "\\rightthreetimes",
  "\\risingdotseq",
  "\\rlap{\\,/}{=}",
  "\\rm AaBb12",
  "\\rmoustache",
  "\\rparen",
  "\\rq",
  "\\rrbracket",
  "\\Rrightarrow",
  "\\Rsh",
  "\\rtimes",
  "x\\rule[6pt]{2ex}{1ex}x",
  "\\rVert",
  "\\rvert",
  "\\text{\\S}",
  "\\scriptscriptstyle \\frac cd",
  "\\scriptsize scriptsize",
  "\\frac ab + {\\scriptstyle \\frac cd}",
  "\\sdot",
  "\\searrow",
  "\\sec",
  "\\text{\\sect}",
  "\\set{x|x<5}",
  "\\Set{ x | x<\\frac 1 2 }",
  "\\setminus",
  "\\sf AaBb123",
  "\\sharp",
  "\\shortmid",
  "\\shortparallel",
  "\\Sigma",
  "\\sigma",
  "\\sim",
  "\\simcolon",
  "\\simcoloncolon",
  "\\simeq",
  "\\sin",
  "\\sinh",
  "\\sixptsize sixptsize",
  "\\sh",
  "\\small small",
  "\\smallfrown",
  "\\smallint",
  "\\begin{smallmatrix} a & b \\\\ c & d \\end{smallmatrix}",
  "\\smallsetminus",
  "\\smallsmile",
  "\\left(x^{\\smash{2}}\\right)",
  "\\smile",
  "\\sout{abc}",
  "a\\space b",
  "\\spades",
  "\\spadesuit",
  "\\sphericalangle",
  "\\begin{equation*} \\begin{split} a &=b+c\\\\ &=e+f \\end{split} \\end{equation*}",
  "\\sqcap",
  "\\sqcup",
  "\\square",
  "\\sqrt[3]{x}",
  "\\sqsubset",
  "\\sqsubseteq",
  "\\sqsupset",
  "\\sqsupseteq",
  "\\text{\\ss}",
  "\\stackrel{!}{=}",
  "\\star",
  "\\sub",
  "\\sube",
  "\\Subset",
  "\\subset",
  "\\subseteq",
  "\\subseteqq",
  "\\subsetneq",
  "\\subsetneqq",
  "\\sum_{\\substack{0\\le i\\le n \\\\ i\\text{ even}}} x_i",
  "\\text{\\textgreater}",
  "\\textit{AaBb}",
  "\\text{\\textless}",
  "\\textmd{AaBb123}",
  "\\textnormal{AB}",
  "\\text{\\textquotedblleft}",
  "\\text{\\textquotedblright}",
  "\\text{\\textquoteleft}",
  "\\text{\\textquoteright}",
  "\\text{\\textregistered}",
  "\\textrm{AaBb123}",
  "\\textsf{AaBb123}",
  "\\text{\\textsterling}",
  "\\textstyle\\sum_0^n",
  "\\texttt{AaBb123}",
  "\\text{\\textunderscore}",
  "\\textup{AaBb123}",
  "\\tfrac ab",
  "\\tg",
  "\\th",
  "\\therefore",
  "\\Theta",
  "\\theta",
  "\\thetasym",
  "\\thickapprox",
  "\\thicksim",
  "a\\thickspace b",
  "a\\thinspace b",
  "\\tilde M",
  "\\times",
  "\\tiny tiny",
  "\\to",
  "\\top",
  "\\triangle",
  "\\triangledown",
  "\\triangleleft",
  "\\trianglelefteq",
  "\\triangleq",
  "\\triangleright",
  "\\trianglerighteq",
  "{\\tt AaBb123}",
  "\\twoheadleftarrow",
  "\\twoheadrightarrow",
  "\\text{\\u{a}}",
  "\\Uarr",
  "\\uArr",
  "\\uarr",
  "\\ulcorner",
  "\\underbar{X}",
  "\\underbrace{x+\u22ef+x}_{n\\text{ times}}",
  "\\undergroup{AB}",
  "\\underleftarrow{AB}",
  "\\underleftrightarrow{AB}",
  "\\underrightarrow{AB}",
  "\\underline{\\text{a long argument}}",
  "\\underlinesegment{AB}",
  "\\underset{!}{=}",
  "\\unlhd",
  "\\unrhd",
  "\\Uparrow",
  "\\uparrow",
  "\\Updownarrow",
  "\\updownarrow",
  "\\upharpoonleft",
  "\\upharpoonright",
  "\\uplus",
  "\\Upsilon",
  "\\upsilon",
  "\\upuparrows",
  "\\urcorner",
  "\\url{https://katex.org/}",
  "\\utilde{AB}",
  "\\text{\\v{a}}",
  "\\varDelta",
  "\\varepsilon",
  "\\varGamma",
  "\\varinjlim",
  "\\varkappa",
  "\\varLambda",
  "\\varliminf",
  "\\varlimsup",
  "\\varnothing",
  "\\varOmega",
  "\\varPhi",
  "\\varphi",
  "\\varPi",
  "\\varpi",
  "\\varprojlim",
  "\\varpropto",
  "\\varPsi",
  "\\varrho",
  "\\varSigma",
  "\\varsigma",
  "\\varsubsetneq",
  "\\varsubsetneqq",
  "\\varsupsetneq",
  "\\varsupsetneqq",
  "\\varTheta",
  "\\vartheta",
  "\\vartriangle",
  "\\vartriangleleft",
  "\\vartriangleright",
  "\\varUpsilon",
  "\\varXi",
  "\\mathrel{\\vcentcolon =}",
  "a+\\left(\\vcenter{\\frac{\\frac a b}c}\\right)",
  "\\Vdash",
  "\\vDash",
  "\\vdash",
  "\\vdots",
  "\\vec{F}",
  "\\vee",
  "\\veebar",
  "\\verb!\\frac a b!",
  "\\Vert",
  "\\vert",
  "\\begin{Vmatrix} a & b \\\\ c & d \\end{Vmatrix}",
  "\\begin{Vmatrix*}[r] 0 & -1 \\\\ -1 & 0 \\end{Vmatrix*}",
  "\\begin{vmatrix} a & b \\\\ c & d \\end{vmatrix}",
  "\\begin{vmatrix*}[r] 0 & -1 \\\\ -1 & 0 \\end{vmatrix*}",
  "\\overline{\\vphantom{M}a}",
  "\\Vvdash",
  "\\wedge",
  "\\weierp",
  "\\widecheck{AB}",
  "\\widehat{AB}",
  "\\widetilde{AB}",
  "\\wp",
  "\\wr",
  "\\xcancel{ABC}",
  "\\def\\foo{a}\\xdef\\fcopy{\\foo}\\def\\foo{}\\fcopy",
  "\\Xi",
  "\\xi",
  "\\xhookleftarrow{abc}",
  "\\xhookrightarrow{abc}",
  "\\xLeftarrow{abc}",
  "\\xleftarrow{abc}",
  "\\xleftharpoondown{abc}",
  "\\xleftharpoonup{abc}",
  "\\xLeftrightarrow{abc}",
  "\\xleftrightarrow{abc}",
  "\\xleftrightharpoons{abc}",
  "\\xlongequal{abc}",
  "\\xmapsto{abc}",
  "\\xRightarrow{abc}",
  "\\xrightarrow{abc}",
  "\\xrightharpoondown{abc}",
  "\\xrightharpoonup{abc}",
  "\\xrightleftharpoons{abc}",
  "\\xtofrom{abc}",
  "\\xtwoheadleftarrow{abc}",
  "\\xtwoheadrightarrow{abc}",
  "\\yen",
  "\\Z",
  "\\Zeta",
  "\\zeta",
  "\\hat{M}",
  "\\hat{b}",
  "\\hat{\\aa}",
  "\\hat{o}",
  "\\tilde{M}",
  "\\left\\{\\begin{array}{l} \\nabla\\cdot\\vec{D} = \\rho \\\\ \\nabla\\cdot\\vec{B} = 0 \\\\ \\nabla\\times\\vec{E} = -\\frac{\\partial\\vec{B}}{\\partial t} \\\\ \\nabla\\times\\vec{H} = \\vec{J}_f + \\frac{\\partial\\vec{D}}{\\partial t} \\end{array}\\right.",
  "\u9178 + \u78b1 \\rightarrow \u76d0 + \u6c34",
  "\\sqrt{}",
  "\\begin{pmatrix} a_{11} & a_{12} & a_{13} \\\\ a_{21} & a_{22} & a_{23} \\\\ a_{31} & a_{32} & a_{33} \\end{pmatrix}",
  "\\begin{vmatrix} 1 & 2 & 3 \\\\ 4 & 5 & 6 \\\\ 7 & 8 & 9 \\end{vmatrix}",
  "\\begin{bmatrix} 1 & 0 & 0 \\\\ 0 & 1 & 0 \\\\ 0 & 0 & 1 \\end{bmatrix}",
  "\\begin{pmatrix} \\cos\\theta & -\\sin\\theta & 0 \\\\ \\sin\\theta & \\cos\\theta & 0 \\\\ 0 & 0 & 1 \\end{pmatrix}",
  "\\begin{pmatrix} 1 & 0 & \\cdots & 0 \\\\ 0 & 1 & \\cdots & 0 \\\\ \\vdots & \\vdots & \\ddots & \\vdots \\\\ 0 & 0 & \\cdots & 1 \\end{pmatrix}",
  "\\begin{bmatrix} A & B \\\\ C & D \\end{bmatrix}\\begin{bmatrix} x \\\\ y \\end{bmatrix} = \\begin{bmatrix} 0 \\\\ 0 \\end{bmatrix}",
  "\\begin{Bmatrix} a & b & c \\\\ d & e & f \\\\ g & h & i \\end{Bmatrix}",
  "\\mathring{A}",
  "\\hat{\\hat{x}}",
  "\\dot{\\vec{p}} = -\\nabla V",
  "\\widehat{XY + Z}",
  "\\widetilde{f \\circ g}",
  "\\ddot{\\vec{r}}",
  "R^{\\mu}{}_{{\\nu\\rho\\sigma}}",
  "T^{\\mu\\nu}{}_{\\rho} = g^{\\mu\\alpha}g^{\\nu\\beta}T_{\\alpha\\beta\\rho}",
  "\\Gamma^{\\mu}_{\\nu\\rho} = \\tfrac{1}{2}g^{\\mu\\sigma}\\!\\left(\\partial_\\nu g_{\\rho\\sigma}+\\partial_\\rho g_{\\nu\\sigma}-\\partial_\\sigma g_{\\nu\\rho}\\right)",
  "\\nabla_\\mu T^{\\mu\\nu} = 0",
  "\\exp\\Bigl(\\tfrac{x+y}{1+x^2}\\Bigr)",
  "f^{(n)}(x) = \\dfrac{\\mathrm{d}^n f}{\\mathrm{d}x^n}",
  "\\frac{\\partial^2 u}{\\partial x \\partial y}",
  "\\begin{matrix} \\hline a & b \\\\ c & d \\end{matrix}",
  "\\displaystyle\\binom{k}{p}\\binom{n-k}{r-p}",
  "\\bigl\\{\\begin{smallmatrix} 1&2\\\\3&4 \\end{smallmatrix}\\bigr\\}",
  "\\operatorname{supp} f \\cap \\operatorname{supp} g",
  "\\mathring{U} \\subseteq \\overline{V}",
  "\\acute{\\mathbb{A}}",
  "\\check{\\mathscr{D}}",
  "\\tilde{\\mathrm{e}}^{\\mathrm{i}\\pi}+1=0",
  "\\utilde{EF} + \\widehat{GH}",
  "\\begin{array}{|c|c|} \\hline 1 & 2 \\\\ \\hline 3 & 4 \\\\ \\hline \\end{array}",
  "\\frac{-b \\pm \\sqrt{b^2-4ac}}{2a}",
  "\\ce{CO2 + C -> 2 CO}",
  "\\ce{Hg^2+ ->[I-] HgI2 ->[I-] [Hg^{II}I4]^2-}",
  "$C_p[\\ce{H2O(l)}] = \\pu{75.3 J // mol K}$",
  "\\ce{H2O}",
  "\\ce{Sb2O3}",
  "\\ce{H+}",
  "\\ce{CrO4^2-}",
  "\\ce{[AgCl2]-}",
  "\\ce{Y^99+}",
  "\\ce{Y^{99+}}",
  "\\ce{2 H2O}",
  "\\ce{2H2O}",
  "\\ce{0.5 H2O}",
  "\\ce{1/2 H2O}",
  "\\ce{(1/2) H2O}",
  "\\ce{$n$ H2O}",
  "\\ce{^{227}_{90}Th+}",
  "\\ce{^227_90Th+}",
  "\\ce{^{0}_{-1}n^{-}}",
  "\\ce{^0_-1n-}",
  "\\ce{H{}^3HO}",
  "\\ce{H^3HO}",
  "\\ce{A -> B}",
  "\\ce{A <- B}",
  "\\ce{A <-> B}",
  "\\ce{A <--> B}",
  "\\ce{A <=> B}",
  "\\ce{A <=>> B}",
  "\\ce{A <<=> B}",
  "\\ce{A ->[H2O] B}",
  "\\ce{A ->[{text above}][{text below}] B}",
  "\\ce{A ->[$x$][$x_i$] B}",
  "\\ce{(NH4)2S}",
  "\\ce{[\\{(X2)3\\}2]^3+}",
  "\\ce{CH4 + 2 $\\left( \\ce{O2 + 79/21 N2} \\right)$}",
  "\\ce{H2(aq)}",
  "\\ce{CO3^2-_{(aq)}}",
  "\\ce{NaOH(aq,$\\infty$)}",
  "\\ce{ZnS($c$)}",
  "\\ce{ZnS(\\ca$c$)}",
  "\\ce{NO_x}",
  "\\ce{Fe^n+}",
  "\\ce{x Na(NH4)HPO4 ->[\\Delta] (NaPO3)_x + x NH3 ^ + x H2O}",
  "\\ce{\\mu-Cl}",
  "\\ce{[Pt(\\eta^2-C2H4)Cl3]-}",
  "\\ce{\\beta +}",
  "\\ce{^40_18Ar + \\gamma{} + \\nu_e}",
  "\\ce{Fe(CN)_{$\\frac{6}{2}$}}",
  "\\ce{X_{$i$}^{$x$}}",
  "\\ce{X_$i$^$x$}",
  "\\ce{$cis${-}[PtCl2(NH3)2]}",
  "\\ce{CuS($hP12$)}",
  "\\ce{{Gluconic Acid} + H2O2}",
  "\\ce{X_{{red}}}",
  "\\ce{{(+)}_589{-}[Co(en)3]Cl3}",
  "\\ce{C6H5-CHO}",
  "\\ce{A-B=C#D}",
  "\\ce{A\\bond{-}B\\bond{=}C\\bond{#}D}",
  "\\ce{A\\bond{1}B\\bond{2}C\\bond{3}D}",
  "\\ce{A\\bond{~}B\\bond{~-}C}",
  "\\ce{A\\bond{~--}B\\bond{~=}C\\bond{-~-}D}",
  "\\ce{A\\bond{...}B\\bond{....}C}",
  "\\ce{A\\bond{->}B\\bond{<-}C}",
  "\\ce{KCr(SO4)2*12H2O}",
  "\\ce{KCr(SO4)2.12H2O}",
  "\\ce{KCr(SO4)2 * 12 H2O}",
  "\\ce{Fe^{II}Fe^{III}2O4}",
  "\\ce{OCO^{.-}}",
  "\\ce{NO^{(2.)-}}",
  "\\ce{Li^x_{Li,1-2x}Mg^._{Li,x}$V$'_{Li,x}Cl^x_{Cl}}",
  "\\ce{O''_{i,x}}",
  "\\ce{M^{..}_i}",
  "\\ce{$V$^{4'}_{Ti}}",
  "\\ce{V_{V,1}C_{C,0.8}$V$_{C,0.2}}",
  "\\ce{A + B}",
  "\\ce{A - B}",
  "\\ce{A = B}",
  "\\ce{A \\pm B}",
  "\\ce{SO4^2- + Ba^2+ -> BaSO4 v}",
  "\\ce{A v B (v) -> B ^ B (^)}",
  "\\ce{NO^*}",
  "\\ce{1s^2-N}",
  "\\ce{n-Pr}",
  "\\ce{iPr}",
  "\\ce{\\ca Fe}",
  "\\ce{A, B, C; F}",
  "\\ce{{and others}}",
  "\\ce{Zn^2+  <=>[+ 2OH-][+ 2H+]  $\\underset{\\text{amphoteres Hydroxid}}{\\ce{Zn(OH)2 v}}$  <=>[+ 2OH-][+ 2H+]  $\\underset{\\text{Hydroxozikat}}{\\ce{[Zn(OH)4]^2-}}$}",
  "\\ce{$K = \\frac{[\\ce{Hg^2+}][\\ce{Hg}]}{[\\ce{Hg2^2+}]}$}",
  "\\ce{$K = \\ce{\\frac{[Hg^2+][Hg]}{[Hg2^2+]}}$}",
  "\\ce{Hg^2+ ->[I-]  $\\underset{\\mathrm{red}}{\\ce{HgI2}}$  ->[I-]  $\\underset{\\mathrm{red}}{\\ce{[Hg^{II}I4]^2-}}$}",
  "\\pu{123 kJ}",
  "\\pu{123 mm2}",
  "\\pu{123 J s}",
  "\\pu{123 J*s}",
  "\\pu{123 kJ/mol}",
  "\\pu{123 kJ//mol}",
  "\\pu{123 kJ mol-1}",
  "\\pu{123 kJ*mol-1}",
  "\\pu{1.2e3 kJ}",
  "\\pu{1,2e3 kJ}",
  "\\pu{1.2E3 kJ}",
  "\\pu{1,2E3 kJ}"
];
const GOLDEN_SCORES = {"1":0.935,"2":0.947,"3":0.911,"4":0.935,"5":0.9,"6":0.915,"7":0.886,"8":0.901,"9":0.939,"10":0.902,"11":0.938,"12":0.905,"13":0.941,"14":0.943,"15":0.9,"16":0.719,"17":0.941,"18":0.901,"19":0.946,"20":0.902,"21":0.915,"22":0.889,"23":0.909,"24":0.887,"25":0.936,"26":0.928,"27":0.91,"28":0.892,"29":0.915,"30":0.937,"31":0.942,"32":0.942,"33":0.942,"34":0.833,"35":0.833,"36":0.833,"37":0.804,"38":0.804,"39":0.804,"40":0.921,"41":0.885,"42":0.927,"43":0.929,"44":0.841,"45":0.919,"46":0.8,"47":0.908,"48":0.929,"49":0.923,"50":0.889,"51":0.882,"52":0.937,"53":0.937,"54":0.877,"55":0.875,"56":0.939,"57":0.914,"58":0.863,"59":0.918,"60":0.914,"61":0.897,"62":0.913,"63":0.934,"64":0.901,"65":0.831,"66":0.9,"67":0.926,"68":0.925,"69":0.919,"70":0.752,"71":0.918,"72":0.838,"73":0.925,"74":0.862,"75":0.882,"76":0.924,"77":0.909,"78":0.954,"79":0.903,"80":0.901,"81":0.965,"82":0.935,"83":0.918,"84":0.931,"85":0.918,"86":0.934,"87":0.939,"88":0.921,"89":0.923,"90":0.9,"91":0.689,"92":0.782,"93":0.935,"94":0.916,"95":0.93,"96":0.943,"97":0.721,"98":0.531,"99":0.951,"100":0.954,"101":0.95,"102":0.911,"103":0.935,"104":0.929,"105":0.966,"106":0.859,"107":0.928,"108":0.94,"109":0.949,"110":0.946,"111":0.922,"112":0.973,"113":0.954,"114":0.983,"115":0.953,"116":0.944,"117":0.944,"118":0.908,"119":0.908,"120":0.943,"121":0.908,"122":0.928,"123":0.95,"124":0.839,"125":0.908,"126":0.911,"127":0.901,"128":0.892,"129":0.893,"130":0.857,"131":0.891,"132":0.893,"133":0.894,"134":0.917,"135":0.917,"136":0.905,"137":0.789,"138":0.943,"139":0.901,"140":0.912,"141":0.966,"142":0.966,"143":0.934,"144":0.937,"145":0.901,"146":0.853,"147":0.925,"148":0.936,"149":0.92,"150":0.689,"151":0.806,"152":0.806,"153":0.931,"154":0.844,"155":0.943,"156":0.901,"157":0.807,"158":0.892,"159":0.945,"160":0.935,"161":0.898,"162":0.861,"163":0.924,"164":0.914,"165":0.939,"166":0.848,"167":0.844,"168":0.908,"169":0.897,"170":0.898,"171":0.921,"172":0.907,"173":0.966,"174":0.966,"175":0.873,"176":0.885,"177":0.88,"178":0.887,"179":0.89,"180":0.88,"181":0.873,"182":0.91,"183":0.877,"184":0.91,"185":0.925,"186":0.871,"187":0.873,"188":0.871,"189":0.925,"190":0.877,"191":0.884,"192":0.939,"193":0.98,"194":0.947,"195":0.873,"196":0.894,"197":0.977,"198":0.81,"199":0.937,"200":0.936,"201":0.943,"202":0.922,"203":0.944,"204":0.948,"205":0.915,"206":0.93,"207":0.933,"208":0.945,"209":0.925,"210":0.938,"211":0.884,"212":0.883,"213":0.926,"214":0.93,"215":0.772,"216":0.785,"217":0.924,"218":0.924,"219":0.924,"220":0.927,"221":0.898,"222":0.898,"223":0.91,"224":0.914,"225":0.93,"226":0.924,"227":0.957,"228":0.922,"229":0.89,"230":0.92,"231":0.924,"232":0.924,"233":0.693,"234":0.707,"235":0.912,"236":0.895,"237":0.825,"238":0.941,"239":0.935,"240":0.928,"241":0.931,"242":0.934,"243":0.91,"244":0.873,"245":0.911,"246":0.882,"247":0.896,"248":0.893,"249":0.835,"250":0.835,"251":0.948,"252":0.766,"253":0.903,"254":0.869,"255":0.911,"256":0.944,"257":0.915,"258":0.944,"259":0.961,"260":0.759,"261":0.759,"262":0.694,"263":0.845,"264":0.906,"265":0.937,"266":0.908,"267":0.925,"268":0.925,"269":0.898,"270":0.91,"271":0.932,"272":0.923,"273":0.975,"274":0.927,"275":0.901,"276":0.89,"277":0.75,"278":0.927,"279":0.927,"280":0.939,"281":0.904,"282":0.927,"283":0.922,"284":0.914,"285":0.929,"286":0.874,"287":0.874,"288":0.877,"289":0.873,"290":0.926,"291":0.914,"292":0.915,"293":0.873,"294":0.877,"295":0.932,"296":0.911,"297":0.912,"298":0.901,"299":0.906,"300":0.906,"301":0.915,"302":0.9,"303":0.791,"304":0.964,"305":0.876,"306":0.948,"307":0.761,"308":0.922,"309":0.888,"310":0.935,"311":0.912,"312":0.876,"313":0.921,"314":0.918,"315":0.898,"316":0.898,"317":0.937,"318":0.848,"319":0.905,"320":0.928,"321":0.905,"322":0.928,"323":0.889,"324":0.911,"325":0.923,"326":0.907,"327":0.907,"328":0.907,"329":0.921,"330":0.931,"331":0.896,"332":0.929,"333":0.904,"334":0.88,"335":0.877,"336":0.931,"337":0.886,"338":0.901,"339":0.92,"340":0.906,"341":0.915,"342":0.881,"343":0.907,"344":0.882,"345":0.894,"346":0.894,"347":0.903,"348":0.917,"349":0.937,"350":0.932,"351":0.887,"352":0.905,"353":0.905,"354":0.934,"355":0.942,"356":0.893,"357":0.899,"358":0.942,"359":0.551,"360":0.942,"361":0.934,"362":0.941,"363":0.89,"364":0.89,"365":0.89,"366":0.89,"367":0.949,"368":0.947,"369":0.892,"370":0.904,"371":0.951,"372":0.958,"373":0.874,"374":0.874,"375":0.911,"376":0.869,"377":0.877,"378":0.882,"379":0.872,"381":0.905,"382":0.882,"383":0.882,"384":0.943,"385":0.944,"386":0.985,"387":0.944,"388":0.908,"389":0.904,"390":0.872,"391":0.718,"392":0.893,"393":0.885,"394":0.901,"395":0.917,"396":0.904,"397":0.806,"398":0.937,"399":0.907,"400":0.912,"401":0.912,"402":0.912,"403":0.938,"404":0.908,"405":0.925,"406":0.925,"407":0.893,"408":0.893,"409":0.923,"410":0.883,"411":0.946,"412":0.907,"413":0.804,"414":0.948,"415":0.951,"416":0.878,"417":0.932,"418":0.811,"419":0.937,"420":0.928,"421":0.863,"422":0.899,"423":0.923,"424":0.893,"425":0.861,"426":0.943,"427":0.942,"428":0.866,"429":0.894,"430":0.903,"431":0.922,"432":0.922,"433":0.841,"434":0.906,"435":0.928,"436":0.888,"437":0.913,"438":0.903,"439":0.886,"440":0.922,"441":0.906,"442":0.907,"443":0.881,"444":0.923,"445":0.942,"446":0.946,"447":0.898,"448":0.959,"449":0.854,"450":0.859,"451":0.907,"452":0.857,"453":0.935,"454":0.897,"455":0.931,"456":0.913,"457":0.909,"458":0.909,"459":0.927,"460":0.959,"461":0.894,"462":0.93,"463":0.904,"464":0.92,"465":0.882,"466":0.941,"467":0.951,"468":0.907,"469":0.932,"470":0.863,"471":0.874,"472":0.94,"473":0.913,"474":0.928,"475":0.926,"476":0.914,"477":0.896,"478":0.909,"479":0.894,"480":0.894,"481":0.903,"482":0.932,"483":0.939,"484":0.883,"485":0.885,"486":0.917,"487":0.875,"488":0.844,"489":0.907,"490":0.958,"491":0.892,"492":0.88,"493":0.839,"494":0.94,"495":0.901,"496":0.941,"497":0.797,"498":0.871,"499":0.937,"500":0.935,"501":0.949,"502":0.718,"503":0.935,"504":0.944,"505":0.91,"506":0.813,"507":0.891,"508":0.92,"509":0.929,"510":0.914,"511":0.937,"512":0.94,"513":0.943,"514":0.941,"515":0.935,"516":0.938,"517":0.927,"518":0.92,"519":0.88,"520":0.856,"521":0.945,"522":0.856,"523":0.964,"524":0.939,"525":0.941,"526":0.941,"527":0.915,"528":0.92,"529":0.945,"530":0.947,"531":0.941,"532":0.935,"533":0.763,"534":0.925,"535":0.925,"536":0.917,"537":0.914,"538":0.906,"539":0.88,"540":0.876,"541":0.918,"542":0.919,"543":0.853,"544":0.927,"545":0.919,"546":0.914,"547":0.907,"548":0.887,"549":0.939,"550":0.847,"551":0.833,"552":0.94,"553":0.941,"554":0.833,"555":0.925,"556":0.93,"557":0.94,"558":0.909,"559":0.924,"560":0.921,"561":0.882,"562":0.931,"563":0.919,"564":0.939,"565":0.921,"566":0.915,"567":0.912,"568":0.921,"569":0.926,"570":0.919,"571":0.912,"572":0.927,"573":0.912,"574":0.927,"575":0.892,"576":0.86,"577":0.904,"578":0.901,"579":0.901,"580":0.892,"581":0.911,"582":0.905,"583":0.887,"584":0.9,"585":0.896,"586":0.942,"587":0.913,"588":0.852,"589":0.939,"590":0.897,"591":0.911,"592":0.881,"593":0.884,"594":0.918,"595":0.886,"596":0.715,"597":0.715,"598":0.915,"599":0.908,"600":0.905,"601":0.899,"602":0.863,"603":0.651,"604":0.804,"605":0.873,"606":0.871,"607":0.86,"608":0.73,"609":0.824,"610":0.829,"611":0.875,"612":0.893,"613":0.801,"614":0.88,"615":0.939,"616":0.875,"617":0.932,"618":0.911,"619":0.92,"620":0.925,"621":0.928,"622":0.927,"623":0.914,"624":0.917,"625":0.941,"626":0.942,"627":0.858,"628":0.858,"629":0.926,"630":0.901,"631":0.856,"632":0.848,"633":0.881,"634":0.936,"635":0.921,"636":0.897,"637":0.904,"638":0.93,"639":0.935,"640":0.883,"641":0.907,"642":0.884,"643":0.887,"644":0.898,"645":0.978,"646":0.909,"647":0.885,"648":0.948,"649":0.926,"650":0.925,"651":0.922,"652":0.941,"653":0.944,"654":0.912,"655":0.928,"656":0.93,"657":0.925,"658":0.899,"659":0.899,"660":0.928,"661":0.896,"662":0.941,"663":0.947,"664":0.909,"665":0.927,"666":0.967,"667":0.941,"668":0.941,"669":0.912,"670":0.912,"671":0.935,"672":0.974,"673":0.964,"674":0.92,"675":0.898,"676":0.921,"677":0.915,"678":0.905,"679":0.899,"680":0.928,"681":0.867,"682":0.939,"683":0.94,"684":0.926,"685":0.903,"686":0.862,"687":0.863,"688":0.878,"689":0.902,"690":0.951,"691":0.948,"692":0.92,"693":0.943,"694":0.907,"695":0.896,"696":0.915,"697":0.875,"698":0.933,"699":0.865,"700":0.875,"701":0.844,"702":0.908,"703":0.872,"704":0.942,"705":0.807,"706":0.806,"707":0.909,"708":0.927,"709":0.908,"710":0.83,"711":0.879,"712":0.919,"713":0.957,"714":0.905,"715":0.81,"716":0.91,"717":0.901,"718":0.935,"719":0.925,"720":0.882,"721":0.877,"722":0.925,"723":0.924,"724":0.949,"725":0.92,"726":0.946,"727":0.926,"728":0.83,"729":0.94,"730":0.902,"731":0.879,"732":0.917,"733":0.906,"734":0.929,"735":0.94,"736":0.939,"737":0.965,"738":0.965,"739":0.922,"740":0.845,"741":0.873,"742":0.9,"743":0.892,"744":0.838,"745":0.917,"746":0.943,"747":0.92,"748":0.942,"749":0.907,"750":0.801,"751":0.944,"752":0.87,"753":0.937,"754":0.899,"755":0.87,"756":0.937,"757":0.923,"758":0.932,"759":0.911,"760":0.805,"761":0.886,"762":0.718,"763":0.885,"764":0.943,"765":0.895,"766":0.942,"767":0.943,"768":0.939,"769":0.938,"770":0.82,"771":0.943,"772":0.957,"773":0.936,"774":0.884,"775":0.941,"776":0.691,"777":0.943,"778":0.901,"779":0.941,"780":0.938,"781":0.885,"782":0.885,"783":0.902,"784":0.894,"785":0.94,"786":0.937,"787":0.943,"788":0.941,"789":0.882,"790":0.914,"791":0.897,"792":0.928,"793":0.908,"794":0.928,"795":0.906,"796":0.899,"797":0.925,"798":0.863,"799":0.901,"800":0.934,"801":0.941,"802":0.886,"803":0.888,"804":0.924,"805":0.896,"806":0.896,"807":0.908,"808":0.935,"809":0.838,"810":0.673,"811":0.924,"812":0.943,"813":0.936,"814":0.945,"815":0.683,"816":0.915,"817":0.826,"818":0.925,"819":0.934,"820":0.896,"821":0.908,"822":0.902,"823":0.932,"824":0.928,"825":0.974,"826":0.928,"827":0.915,"828":0.875,"829":0.931,"830":0.935,"831":0.576,"832":0.919,"833":0.919,"834":0.919,"835":0.937,"836":0.929,"837":0.927,"838":0.948,"839":0.914,"840":0.831,"841":0.868,"842":0.846,"843":0.907,"844":0.909,"845":0.909,"846":0.93,"847":0.936,"848":0.925,"849":0.871,"850":0.909,"851":0.909,"852":0.901,"853":0.896,"854":0.91,"855":0.888,"856":0.915,"857":0.893,"858":0.884,"859":0.894,"860":0.935,"861":0.898,"862":0.898,"863":0.913,"864":0.916,"865":0.866,"866":0.815,"867":0.914,"868":0.904,"869":0.887,"870":0.943,"871":0.883,"872":0.914,"873":0.916,"874":0.891,"875":0.875,"876":0.844,"877":0.893,"878":0.796,"879":0.867,"880":0.876,"881":0.897,"882":0.917,"883":0.908,"884":0.925,"885":0.802,"886":0.807,"887":0.708,"888":0.925,"889":0.913,"890":0.764,"891":0.901,"892":0.916,"893":0.915,"894":0.919,"895":0.919,"896":0.881,"897":0.919,"898":0.929,"899":0.895,"900":0.884,"901":0.914,"902":0.89,"903":0.85,"904":0.915,"905":0.887,"906":0.921,"907":0.931,"908":0.899,"909":0.889,"910":0.885,"911":0.866,"912":0.862,"913":0.924,"914":0.907,"915":0.907,"916":0.916,"917":0.914,"918":0.923,"919":0.857,"920":0.927,"921":0.882,"922":0.835,"923":0.794,"924":0.938,"925":0.793,"926":0.866,"927":0.926,"928":0.739,"929":0.665,"930":0.926,"931":0.914,"932":0.949,"933":0.839,"934":0.911,"935":0.67,"936":0.773,"937":0.853,"938":0.927,"939":0.795,"940":0.814,"941":0.919,"942":0.857,"943":0.698,"944":0.893,"945":0.915,"946":0.928,"947":0.78,"948":0.843,"949":0.862,"950":0.901,"951":0.896,"952":0.812,"953":0.703,"954":0.75,"955":0.901,"956":0.752,"957":0.878,"958":0.865,"959":0.911,"960":0.935,"961":0.931,"962":0.891,"963":0.892,"964":0.915,"965":0.915,"966":0.898,"967":0.898,"968":0.873,"969":0.908,"970":0.866,"971":0.889,"972":0.94,"973":0.94,"974":0.931,"975":0.931,"976":0.952,"977":0.952,"978":0.861,"979":0.86,"980":0.851,"981":0.951,"982":0.834,"983":0.829,"984":0.824,"985":0.587,"986":0.92,"987":0.902,"988":0.876,"989":0.936,"990":0.92,"991":0.897,"992":0.836,"993":0.843,"994":0.903,"995":0.869,"996":0.893,"997":0.944,"998":0.719,"999":0.953,"1000":0.923,"1001":0.935,"1002":0.91,"1003":0.911,"1004":0.925,"1005":0.925,"1006":0.798,"1007":0.852,"1008":0.71,"1009":0.912,"1010":0.76,"1011":0.927,"1012":0.829,"1013":0.829,"1014":0.829,"1015":0.709,"1016":0.76,"1017":0.837,"1018":0.85,"1019":0.794,"1020":0.794,"1021":0.794,"1022":0.933,"1023":0.913,"1024":0.888,"1025":0.904,"1026":0.935,"1027":0.945,"1028":0.944,"1029":0.89,"1030":0.865,"1031":0.897,"1032":0.9,"1033":0.914,"1034":0.713,"1035":0.942,"1036":0.944,"1037":0.894,"1038":0.95,"1039":0.964,"1040":0.964,"1041":0.859,"1042":0.89,"1043":0.554,"1044":0.886,"1045":0.886,"1046":0.591,"1047":0.961,"1048":0.945,"1049":0.961,"1050":0.959,"1051":0.873,"1052":0.92,"1053":0.947,"1054":0.951,"1055":0.95,"1056":0.874,"1057":0.938,"1058":0.866};

// ── score → tier ──
function tier(s) {
  if (s === null) return 'nodata';
  if (s >= 0.9) return 'great';
  if (s >= 0.8) return 'high8';
  if (s >= 0.6) return 'good';
  if (s >= 0.4) return 'ok';
  if (s >= 0.3) return 'low';
  return 'bad';
}
function tierClass(t) {
  const map = {
    great: 'bg-emerald-50 text-emerald-800 border border-emerald-200',
    high8: 'bg-teal-50 text-teal-900 border border-teal-200',
    good: 'bg-lime-50 text-lime-900 border border-lime-200',
    ok: 'bg-amber-50 text-amber-900 border border-amber-200',
    low: 'bg-orange-50 text-orange-900 border border-orange-200',
    bad: 'bg-red-50 text-red-800 border border-red-200',
    err: 'bg-red-50 text-red-800 border border-red-200',
    nodata: 'bg-zinc-100 text-zinc-500 border border-dashed border-zinc-300',
  };
  return map[t] || map.nodata;
}
function tierLabel(t) {
  return { great:'Great', high8:'Strong', good:'Good', ok:'Fair',
           low:'Low', bad:'Poor', nodata:'—', err:'Error' }[t];
}

// ── counters ──
let total = FORMULAS.length;
let rendered = 0;    // WASM render done
let wasmOk = 0, wasmErr = 0;

function counterTier(score) {
  if (score === null) return 'low';
  if (score < 0.3) return 'low';
  if (score >= 0.9) return 'great';
  if (score >= 0.8) return 'high8';
  if (score >= 0.5) return 'mid-hi';
  return 'mid-lo';
}

function refreshHero() {
  const byTier = { great: 0, high8: 0, 'mid-hi': 0, 'mid-lo': 0, low: 0 };
  let scoreSum = 0, scoreN = 0;
  for (let i = 1; i <= total; i++) {
    const s = GOLDEN_SCORES[i] ?? null;
    if (s !== null) { scoreSum += s; scoreN++; }
    byTier[counterTier(s)]++;
  }
  document.getElementById('cnt-great').textContent = byTier.great;
  document.getElementById('cnt-high8').textContent = byTier.high8;
  document.getElementById('cnt-mid-lo').textContent = byTier['mid-lo'];
  document.getElementById('cnt-mid-hi').textContent = byTier['mid-hi'];
  document.getElementById('cnt-low').textContent   = byTier.low;
  document.getElementById('cnt-avg').textContent   = scoreN ? (scoreSum/scoreN).toFixed(2) : '—';
  document.getElementById('b-great').textContent   = byTier.great;
  document.getElementById('b-high8').textContent   = byTier.high8;
  document.getElementById('b-mid-lo').textContent  = byTier['mid-lo'];
  document.getElementById('b-mid-hi').textContent  = byTier['mid-hi'];
  document.getElementById('b-low').textContent     = byTier.low;
}

function updateProgress() {
  const pct = rendered / total * 100;
  document.getElementById('pfill').style.width = pct + '%';
  document.getElementById('pcount').textContent = rendered + ' / ' + total + ' live renders done';
  if (rendered >= total) {
    document.getElementById('plabel').textContent = 'Done';
    document.getElementById('tstatus').textContent =
      wasmOk + ' rendered, ' + wasmErr + ' errors';
  }
}

// ── font id → CSS font declaration ──
function fontIdToCss(fontId, sizePx) {
  switch (fontId) {
    case "AMS-Regular":         return `${sizePx}px KaTeX_AMS`;
    case "Caligraphic-Regular": return `${sizePx}px KaTeX_Caligraphic`;
    case "Fraktur-Regular":     return `${sizePx}px KaTeX_Fraktur`;
    case "Fraktur-Bold":        return `bold ${sizePx}px KaTeX_Fraktur`;
    case "Main-Bold":           return `bold ${sizePx}px KaTeX_Main`;
    case "Main-BoldItalic":     return `italic bold ${sizePx}px KaTeX_Main`;
    case "Main-Italic":         return `italic ${sizePx}px KaTeX_Main`;
    case "Main-Regular":        return `${sizePx}px KaTeX_Main`;
    case "Math-BoldItalic":     return `italic bold ${sizePx}px KaTeX_Math`;
    case "Math-Italic":         return `italic ${sizePx}px KaTeX_Math`;
    case "SansSerif-Bold":      return `bold ${sizePx}px KaTeX_SansSerif`;
    case "SansSerif-Italic":    return `italic ${sizePx}px KaTeX_SansSerif`;
    case "SansSerif-Regular":   return `${sizePx}px KaTeX_SansSerif`;
    case "Script-Regular":      return `${sizePx}px KaTeX_Script`;
    case "Size1-Regular":       return `${sizePx}px KaTeX_Size1`;
    case "Size2-Regular":       return `${sizePx}px KaTeX_Size2`;
    case "Size3-Regular":       return `${sizePx}px KaTeX_Size3`;
    case "Size4-Regular":       return `${sizePx}px KaTeX_Size4`;
    case "Typewriter-Regular":  return `${sizePx}px KaTeX_Typewriter`;
    default:                    return `${sizePx}px KaTeX_Main`;
  }
}

// ── draw RaTeX ──
function drawDisplayList(dl, canvas, em, pad) {
  const dpr = window.devicePixelRatio || 1;
  const totalH = dl.height + dl.depth;
  const cssW = Math.max(1, Math.ceil(dl.width * em + 2 * pad));
  const cssH = Math.max(1, Math.ceil(totalH * em + 2 * pad));
  canvas.width  = cssW * dpr;
  canvas.height = cssH * dpr;
  canvas.style.width  = cssW + 'px';
  canvas.style.height = cssH + 'px';
  const ctx = canvas.getContext('2d');
  ctx.scale(dpr, dpr);
  ctx.clearRect(0, 0, cssW, cssH);
  ctx.save(); ctx.translate(pad, pad);
  for (const item of dl.items) {
    const c = item.color;
    const rgb = `rgb(${c.r*255|0},${c.g*255|0},${c.b*255|0})`;
    if (item.type === 'Line') {
      ctx.fillStyle = rgb;
      ctx.fillRect(item.x*em, item.y*em - item.thickness*em/2,
                   item.width*em, Math.max(0.5, item.thickness*em));
    } else if (item.type === 'Rect') {
      ctx.fillStyle = rgb;
      ctx.fillRect(item.x*em, item.y*em, item.width*em, item.height*em);
    } else if (item.type === 'Path') {
      const ox = item.x*em, oy = item.y*em;
      ctx.beginPath();
      for (const cmd of item.commands) {
        if      (cmd.type === 'MoveTo') ctx.moveTo(ox+cmd.x*em, oy+cmd.y*em);
        else if (cmd.type === 'LineTo') ctx.lineTo(ox+cmd.x*em, oy+cmd.y*em);
        else if (cmd.type === 'CubicTo')
          ctx.bezierCurveTo(ox+cmd.x1*em,oy+cmd.y1*em, ox+cmd.x2*em,oy+cmd.y2*em, ox+cmd.x*em,oy+cmd.y*em);
        else if (cmd.type === 'QuadTo')
          ctx.quadraticCurveTo(ox+cmd.x1*em,oy+cmd.y1*em, ox+cmd.x*em,oy+cmd.y*em);
        else if (cmd.type === 'Close') ctx.closePath();
      }
      ctx.fillStyle = rgb;
      if (item.fill) ctx.fill(); else ctx.stroke();
    } else if (item.type === 'GlyphPath') {
      const sz = (item.scale || 1) * em;
      ctx.save();
      ctx.translate(item.x*em, item.y*em);
      ctx.font = fontIdToCss(item.font, sz);
      ctx.textBaseline = 'alphabetic'; ctx.textAlign = 'left';
      ctx.fillStyle = rgb;
      ctx.fillText(String.fromCodePoint(item.char_code), 0, 0);
      ctx.restore();
    }
  }
  ctx.restore();
}

// ── WASM & font state ──
let renderLatex = null, wasmReady = false, fontsReady = false;
const pendingQueue = [];

function tryFlushPending() {
  if (!wasmReady || !fontsReady) return;
  let meta;
  while ((meta = pendingQueue.shift())) {
    if (meta.done) continue;
    doRender(meta);
  }
}

function doRender(meta) {
  if (meta.done) return;
  meta.done = true;
  const { latex, ratexCell, scoreEl } = meta;
  const scoreVal = GOLDEN_SCORES[meta.idx + 1] ?? null;
  try {
    const json = renderLatex(latex);
    const dl   = JSON.parse(json);
    const canvas = document.createElement('canvas');
    canvas.className = 'max-w-full h-auto';
    drawDisplayList(dl, canvas, 20, 3);
    ratexCell.innerHTML = '';
    ratexCell.appendChild(canvas);
    wasmOk++;
  } catch(e) {
    const msg = String(e).replace(/^.*?Error:\s*/,'').slice(0, 100);
    ratexCell.innerHTML =
      '<span class="text-xs text-red-600 font-mono break-words max-w-[12rem]" title="' +
      String(e).replace(/"/g,'&quot;') +
      '">' +
      msg +
      '</span>';
    // if WASM errors but we have a golden score, the score already shows the quality
    wasmErr++;
  }
  // Update score badge to show actual score (was already set from golden, just update color)
  // (score badge was already rendered from GOLDEN_SCORES, nothing extra needed)
  rendered++;
  updateProgress();
}

// ── Build table ──
function buildTable() {
  const tbody = document.getElementById('tbody');
  const frag  = document.createDocumentFragment();
  const observer = new IntersectionObserver(entries => {
    entries.forEach(e => {
      if (!e.isIntersecting) return;
      const meta = e.target._meta;
      if (meta && !meta.done) {
        if (wasmReady && fontsReady) doRender(meta);
        else pendingQueue.push(meta);
      }
      observer.unobserve(e.target);
    });
  }, { rootMargin: '300px' });

  FORMULAS.forEach((latex, idx) => {
    const score = GOLDEN_SCORES[idx + 1] ?? null;
    const t     = tier(score);
    const tr = document.createElement('tr');
    tr.className = 'border-b border-outline/40 bg-white hover:bg-surface/90 transition-colors';
    tr.dataset.tier = counterTier(score);
    tr.dataset.q   = latex.toLowerCase();

    // index
    const tdI = document.createElement('td');
    tdI.className =
      'td-idx w-10 px-2 py-2 text-right text-xs text-on-surface-variant tabular-nums align-top';
    tdI.textContent = idx + 1;
    tr.appendChild(tdI);

    // source
    const tdS = document.createElement('td');
    tdS.className =
      'td-source hidden md:table-cell max-w-[min(240px,28vw)] px-3 py-2 font-mono text-[11px] text-on-surface-variant break-all align-top';
    tdS.textContent = latex;
    tr.appendChild(tdS);

    // KaTeX
    const tdK = document.createElement('td');
    tdK.className = 'td-katex min-w-0 align-middle';
    const kc = document.createElement('div');
    kc.className =
      'katex-cell flex items-center overflow-x-auto overflow-y-visible min-h-[28px] text-[16.53px] text-zinc-900';
    try {
      // displayMode: true — required for AMS environments (align, equation, gather, …)
      // and matches tools/golden_compare/generate_reference.mjs (golden PNG reference).
      kc.innerHTML = katex.renderToString(latex, {
        throwOnError: false, displayMode: true, trust: true, strict: false
      });
    } catch(e) {
      kc.innerHTML = '<span class="text-xs text-red-600">KaTeX error</span>';
    }
    tdK.appendChild(kc); tr.appendChild(tdK);

    // RaTeX
    const tdR = document.createElement('td');
    tdR.className = 'td-ratex min-w-0 align-middle';
    const rc = document.createElement('div');
    rc.className = 'ratex-cell flex items-center min-h-[28px] max-w-full';
    rc.innerHTML = '<span class="text-xs text-on-surface-variant italic">loading…</span>';
    tdR.appendChild(rc); tr.appendChild(tdR);

    // Score (from golden, pre-computed offline)
    const tdSc = document.createElement('td');
    tdSc.className = 'td-score w-[100px] sm:w-[110px] px-2 text-center align-middle';
    const badge = document.createElement('div');
    badge.className =
      'inline-flex flex-col items-center gap-0.5 px-2 py-1.5 rounded-md text-xs font-semibold min-w-[56px] text-center ' +
      tierClass(t);
    if (score !== null) {
      badge.innerHTML =
        score.toFixed(2) +
        '<span class="block text-[9px] font-normal opacity-75 leading-tight">' +
        tierLabel(t) +
        '</span>';
    } else {
      badge.innerHTML =
        'no data<span class="block text-[9px] font-normal opacity-75 leading-tight">not built</span>';
    }
    tdSc.appendChild(badge); tr.appendChild(tdSc);

    const meta = { idx, latex, ratexCell: rc, scoreEl: badge, done: false };
    tr._meta = meta;
    frag.appendChild(tr);
    observer.observe(tr);
  });

  tbody.appendChild(frag);
  refreshHero();
  applyFilter();
  document.getElementById('tstatus').textContent = FORMULAS.length + ' formulas, rendering lazily…';
}

// ── Filter / Search ──
let curFilter = 'all', curQ = '';
function applyFilter() {
  let vis = 0;
  for (const tr of document.getElementById('tbody').children) {
    const matchQ = !curQ || tr.dataset.q.includes(curQ);
    const matchF = curFilter === 'all' ||
      (curFilter === 'great' && tr.dataset.tier === 'great') ||
      (curFilter === 'high8' && tr.dataset.tier === 'high8') ||
      (curFilter === 'mid-hi'&& tr.dataset.tier === 'mid-hi')||
      (curFilter === 'mid-lo'&& tr.dataset.tier === 'mid-lo')||
      (curFilter === 'low'   && tr.dataset.tier === 'low');
    const show = matchQ && matchF;
    tr.classList.toggle('hidden', !show);
    if (show) vis++;
  }
  document.getElementById('b-all').textContent = FORMULAS.length;
  document.getElementById('tstatus').textContent = vis + ' formula' + (vis!==1?'s':'') + ' shown';
}
function filterBtnClass(isActive) {
  return isActive
    ? 'filter-btn rounded-full border border-primary bg-primary px-3 py-1.5 text-xs font-medium text-on-primary shadow-sm transition-colors'
    : 'filter-btn rounded-full border border-outline/60 bg-white px-3 py-1.5 text-xs text-on-surface-variant hover:border-primary/30 transition-colors';
}
document.querySelectorAll('.filter-btn').forEach((btn) =>
  btn.addEventListener('click', () => {
    document.querySelectorAll('.filter-btn').forEach((b) => {
      b.className = filterBtnClass(b === btn);
    });
    curFilter = btn.dataset.filter;
    applyFilter();
  }),
);
document.getElementById('search').addEventListener('input', e => {
  curQ = e.target.value.trim().toLowerCase();
  applyFilter();
});

// ── Boot ──
// Both katex.min.js and mhchem.min.js are defer-loaded in DOM order.
// katex fires its load event before mhchem has executed, so \ce / \pu macros
// are not yet registered at that point. Wait for both before building the table.
let _katexReady = false, _mhchemReady = false;
function _tryBoot() {
  if (_katexReady && _mhchemReady) {
    buildTable();
    loadFontsAndWasm();
  }
}
document.getElementById('katex-script').addEventListener('load', () => { _katexReady = true; _tryBoot(); });
document.getElementById('mhchem-script').addEventListener('load', () => { _mhchemReady = true; _tryBoot(); });

async function loadFontsAndWasm() {
  // Pre-load fonts so canvas renders use KaTeX glyphs, not fallback serif
  try {
    document.getElementById('plabel').textContent = 'Loading fonts…';
    await Promise.all([
      document.fonts.load('20px KaTeX_Main'),
      document.fonts.load('italic 20px KaTeX_Main'),
      document.fonts.load('bold 20px KaTeX_Main'),
      document.fonts.load('italic bold 20px KaTeX_Main'),
      document.fonts.load('italic 20px KaTeX_Math'),
      document.fonts.load('italic bold 20px KaTeX_Math'),
      document.fonts.load('20px KaTeX_AMS'),
      document.fonts.load('20px KaTeX_Caligraphic'),
      document.fonts.load('20px KaTeX_Fraktur'),
      document.fonts.load('bold 20px KaTeX_Fraktur'),
      document.fonts.load('20px KaTeX_SansSerif'),
      document.fonts.load('italic 20px KaTeX_SansSerif'),
      document.fonts.load('bold 20px KaTeX_SansSerif'),
      document.fonts.load('20px KaTeX_Script'),
      document.fonts.load('20px KaTeX_Typewriter'),
      document.fonts.load('20px KaTeX_Size1'),
      document.fonts.load('20px KaTeX_Size2'),
      document.fonts.load('20px KaTeX_Size3'),
      document.fonts.load('20px KaTeX_Size4'),
    ]);
  } catch(e) { console.warn('Font pre-load partial:', e); }
  fontsReady = true;

  try {
    document.getElementById('plabel').textContent = 'Loading WASM…';
    const mod = await import(ratexWasmModuleUrl());
    await mod.default();
    renderLatex = mod.renderLatex;
    wasmReady = true;
    document.getElementById('plabel').textContent = 'Rendering…';
    tryFlushPending();
  } catch(e) {
    document.getElementById('plabel').textContent = 'WASM load failed: ' + e;
  }
}
