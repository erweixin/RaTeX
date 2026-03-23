/** Resolves platforms/web/pkg/ratex_wasm.js under Astro site base, not the current page directory. */
function ratexWasmModuleUrl() {
  var g = typeof globalThis !== "undefined" ? globalThis : window;
  if (typeof g.__RATES_WASM_IMPORT_URL__ === "string" && g.__RATES_WASM_IMPORT_URL__.length > 0) {
    return g.__RATES_WASM_IMPORT_URL__;
  }
  var base = typeof g.__RATEX_SITE_BASE__ === "string" ? g.__RATEX_SITE_BASE__ : "";
  if (base) {
    if (!base.endsWith("/")) base += "/";
    return new URL("platforms/web/pkg/ratex_wasm.js", new URL(base, location.origin)).href;
  }
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
  var pageDir = getSiteDirUrl();
  var rel =
    location.pathname.indexOf("/website/") !== -1
      ? "../platforms/web/pkg/ratex_wasm.js"
      : "platforms/web/pkg/ratex_wasm.js";
  return new URL(rel, pageDir).href;
}

const FORMULAS = ["n!", "a\\!b", "\\def\\sqr#1{#1^2} \\sqr{y}", "\\#", "\\%", "\\begin{matrix} a & b \\\\ c & d \\end{matrix}", "\\text{\\'{a}}", "\\text{\\(\\frac a b\\)}", "a\\ b", "\\text{\\\"{a}}", "a\\,\\,{b}", "\\text{\\.{a}}", "a\\:\\:{b}", "\\;\\;{b}", "x_i", "\\text{\\={a}}", "a\\>\\>{b}", "{a}", "\\text{no~no~no~breaks}", "\\text{\\~{a}}", "x^i", "\\text{\\^{a}}", "\\text{\\textdollar}", "\\text{\\`{a}}", "\\text{\\AA}", "\\text{\\aa}", "{a \\above{2pt} b+1}", "\\acute e", "\\text{\\AE}", "\\text{\\ae}", "\\alef", "\\alefsym", "\\aleph", "\\begin{align*} a&=b+c \\\\ d+e&=f \\end{align*}", "\\begin{align*} a&=b+c \\\\ d+e&=f \\end{align*}", "\\begin{aligned} a&=b+c \\\\ d+e&=f \\end{aligned}", "\\begin{alignat*}{2} 10&x+ &3&y = 2 \\\\ 3&x+&13&y = 4 \\end{alignat*}", "\\begin{alignat*}{2} 10&x+ &3&y = 2 \\\\ 3&x+&13&y = 4 \\end{alignat*}", "\\begin{alignedat}{2} 10&x+ &3&y = 2 \\\\ 3&x+&13&y = 4 \\end{alignedat}", "\\Alpha", "\\alpha", "\\amalg", "\\And", "a_{\\angl n}", "\\angln", "\\allowbreak", "\\angle", "\\approx", "\\approxeq", "\\approxcolon", "\\approxcoloncolon", "\\arccos", "\\arcctg", "\\arcsin", "\\arctan", "\\arctg", "\\arg", "\\argmax", "\\argmin", "\\begin{array}{cc} a & b \\\\ c & d \\end{array}", "\\def\\arraystretch{1.5} \\begin{array}{cc} a & b \\\\ c & d \\end{array}", "\\ast", "\\asymp", "{a \\atop b}", "\\backepsilon", "\\backprime", "\\backsim", "\\backsimeq", "\\backslash", "\\bar{y}", "\\barwedge", "\\Bbb{ABC}", "\\Bbbk", "\\bcancel{5}", "\\because", "\\Beta", "\\beta", "\\beth", "\\between", "\\begingroup a \\endgroup", "\\bf AaBb12", "\\big(\\big)", "\\Big(\\Big)", "\\bigcap", "\\bigcirc", "\\bigcup", "\\bigg(\\bigg)", "\\Bigg(\\Bigg)", "\\biggl(", "\\Biggl(", "\\biggm\\vert", "\\Biggm\\vert", "\\biggr)", "\\Biggr)", "\\bigl(", "\\Bigl(", "\\bigm\\vert", "\\Bigm\\vert", "\\bigodot", "\\bigoplus", "\\bigotimes", "\\bigr)", "\\Bigr)", "\\bigsqcup", "\\bigstar", "\\bigtriangledown", "\\bigtriangleup", "\\biguplus", "\\bigvee", "\\bigwedge", "\\binom n k", "\\blacklozenge", "\\blacksquare", "\\blacktriangle", "\\blacktriangledown", "\\blacktriangleleft", "\\blacktriangleright", "\\bm{AaBb}", "\\begin{Bmatrix} a & b \\\\ c & d \\end{Bmatrix}", "\\begin{Bmatrix*}[r] 0 & -1 \\\\ -1 & 0 \\end{Bmatrix*}", "\\begin{bmatrix} a & b \\\\ c & d \\end{bmatrix}", "\\begin{bmatrix*}[r] 0 & -1 \\\\ -1 & 0 \\end{bmatrix*}", "a \\bmod b", "\\bold{AaBb123}", "\\boldsymbol{AaBb}", "\\bot", "\\bowtie", "\\Box", "\\boxdot", "\\boxed{ab}", "\\boxminus", "\\boxplus", "\\boxtimes", "\\Bra{\\psi}", "\\bra{\\psi}", "\\braket{\\phi|\\psi}", "\\Braket{a|\\frac{\\partial^2}{\\partial t^2}|b}", "{n\\brace k}", "{n\\brack k}", "\\breve{eu}", "\\bull", "\\bullet", "\\Bumpeq", "\\bumpeq", "\\cal AaBb123", "\\cancel{5}", "\\Cap", "\\cap", "\\begin{cases} a &\\text{if } b \\\\ c &\\text{if } d \\end{cases}", "\\begin{CD} A @>a>> B \\\\ @VbVV @AAcA \\\\ C @= D \\end{CD}", "\\cdot", "\\cdotp", "\\cdots", "\\ce{C6H5-CHO}", "a\\centerdot b", "\\cfrac{2}{1+\\cfrac{2}{1+\\cfrac{2}{1}}}", "\\char\"263a", "\\check{oe}", "\\ch", "\\checkmark", "\\Chi", "\\chi", "{n+1 \\choose k+2}", "\\circ", "\\circeq", "\\circlearrowleft", "\\circlearrowright", "\\circledast", "\\circledcirc", "\\circleddash", "\\circledR", "\\circledS", "\\clubs", "\\clubsuit", "\\cnums", "\\colon", "\\Colonapprox", "\\colonapprox", "\\coloncolon", "\\coloncolonapprox", "\\coloncolonequals", "\\coloncolonminus", "\\coloncolonsim", "\\Coloneq", "\\coloneq", "\\colonequals", "\\Coloneqq", "\\coloneqq", "\\colonminus", "\\Colonsim", "\\colonsim", "\\color{#0000FF} AaBb123", "\\colorbox{red}{Black on red}", "\\complement", "\\Complex", "\\cong", "\\coprod", "\\copyright", "\\cos", "\\cosec", "\\cosh", "\\cot", "\\cotg", "\\coth", "\\begin{matrix} a & b \\cr c & d \\end{matrix}", "\\csc", "\\ctg", "\\cth", "\\Cup", "\\cup", "\\curlyeqprec", "\\curlyeqsucc", "\\curlyvee", "\\curlywedge", "\\curvearrowleft", "\\curvearrowright", "\\dag", "\\Dagger", "\\dagger", "\\daleth", "\\Darr", "\\dArr", "\\darr", "\\begin{darray}{cc} a & b \\\\ c & d \\end{darray}", "\\dashleftarrow", "\\dashrightarrow", "\\dashv", "\\dbinom n k", "\\dblcolon", "\\begin{dcases} a &\\text{if } b \\\\ c &\\text{if } d \\end{dcases}", "\\ddag", "\\ddagger", "\\ddddot x", "\\dddot x", "\\ddot x", "\\ddots", "\\def\\foo{x^2} \\foo + \\foo", "\\deg", "\\degree", "\\delta", "\\Delta", "\\det", "\\digamma", "\\dfrac{a-1}{b-1}", "\\diagdown", "\\diagup", "\\Diamond", "\\diamond", "\\diamonds", "\\diamondsuit", "\\dim", "\\displaystyle\\sum_0^n", "\\div", "\\divideontimes", "\\dot x", "\\Doteq", "\\doteq", "\\doteqdot", "\\dotplus", "x_1 + \\dots + x_n", "x_1 +\\dotsb + x_n", "x,\\dotsc,y", "\\int_{A_1}\\int_{A_2}\\dotsi", "x_1 x_2 \\dotsm x_n", "\\dotso", "\\doublebarwedge", "\\doublecap", "\\doublecup", "\\Downarrow", "\\downarrow", "\\downdownarrows", "\\downharpoonleft", "\\downharpoonright", "\\begin{drcases} a &\\text{if } b \\\\ c &\\text{if } d \\end{drcases}", "\\def\\foo{a}\\edef\\fcopy{\\foo}\\def\\foo{}\\fcopy", "\\ell", "\\emph{nested \\emph{emphasis}}", "\\empty", "\\emptyset", "a\\enspace b", "\\Epsilon", "\\epsilon", "\\eqcirc", "\\Eqcolon", "\\eqcolon", "\\begin{equation*} a = b + c \\end{equation*}", "\\begin{equation*} a = b + c \\end{equation*}", "\\Eqqcolon", "\\eqqcolon", "\\eqsim", "\\eqslantgtr", "\\eqslantless", "\\equalscolon", "\\equalscoloncolon", "\\equiv", "\\Eta", "\\eta", "\\eth", "\\exist", "\\exists", "\\exp", "\\fallingdotseq", "\\fbox{Hi there!}", "\\fcolorbox{red}{aqua}{A}", "\\Finv", "\\flat", "\\footnotesize footnotesize", "\\forall", "\\frac a b", "\\frak{AaBb}", "\\frown", "\\Game", "\\Gamma", "\\gamma", "\\begin{gather*} a=b \\\\ e=b+c \\end{gather*}", "\\begin{gathered} a=b \\\\ e=b+c \\end{gathered}", "\\gcd", "\\gdef\\sqr#1{#1^2} \\sqr{y} + \\sqr{y}", "\\gdef\\VERT{|} \\braket{\\phi\\VERT\\psi}", "\\ge", "\\genfrac ( ] {2pt}{0}a{a+1}", "\\geq", "\\geqq", "\\geqslant", "\\gets", "\\gg", "\\ggg", "\\gggtr", "\\gimel", "\\global\\def\\add#1#2{#1+#2} \\add 2 3", "\\gnapprox", "\\gneq", "\\gneqq", "\\gnsim", "\\grave{eu}", "a \\gt b", "\\gtrdot", "\\gtrapprox", "\\gtreqless", "\\gtreqqless", "\\gtrless", "\\gtrsim", "\\gvertneqq", "\\text{\\H{a}}", "\\Harr", "\\hArr", "\\harr", "\\hat{\\theta}", "\\hbar", "\\hbox{$x^2$}", "\\begin{matrix} a & b \\\\ \\hdashline c & d \\end{matrix}", "\\hearts", "\\heartsuit", "\\begin{matrix} a & b \\\\ \\hline c & d \\end{matrix}", "\\hom", "\\hookleftarrow", "\\hookrightarrow", "a\\hphantom{bc}d", "\\href{https://katex.org/}{\\KaTeX}", "w\\hskip1em i\\hskip2em d", "\\hslash", "s\\hspace{7ex} k", "\\htmlClass{foo}{x}", "\\htmlData{foo=a, bar=b}{x}", "\\htmlId{bar}{x}", "\\htmlStyle{color: red;}{x}", "\\huge huge", "\\Huge Huge", "\\text{\\i}", "A\\iff B", "\\iiint", "\\iint", "\\Im", "\\image", "\\imageof", "\\imath", "P\\impliedby Q", "P\\implies Q", "\\in", "\\includegraphics[height=0.8em, totalheight=0.9em, width=0.9em, alt=KA logo]{https://cdn.kastatic.org/images/apple-touch-icon-57x57-precomposed.new.png}", "\\inf", "\\infin", "\\infty", "\\injlim", "\\int", "\\intercal", "\\intop", "\\Iota", "\\iota", "\\isin", "{\\it AaBb}", "\\text{\\j}", "\\jmath", "\\Join", "\\Kappa", "\\kappa", "\\KaTeX", "\\ker", "I\\kern-2.5pt R", "\\Ket{\\psi}", "\\ket{\\psi}", "\\Lambda", "\\lambda", "\\land", "\\lang A\\rangle", "\\langle A\\rangle", "\\Larr", "\\lArr", "\\larr", "\\large large", "\\Large Large", "\\LARGE LARGE", "\\LaTeX", "\\lBrace", "\\lbrace", "\\lbrack", "\\lceil", "\\ldotp", "\\ldots", "\\le", "\\leadsto", "\\left\\lbrace \\dfrac ab \\right.", "\\leftarrow", "\\Leftarrow", "\\leftarrowtail", "\\leftharpoondown", "\\leftharpoonup", "\\leftleftarrows", "\\Leftrightarrow", "\\leftrightarrow", "\\leftrightarrows", "\\leftrightharpoons", "\\leftrightsquigarrow", "\\leftthreetimes", "\\leq", "\\leqq", "\\leqslant", "\\lessapprox", "\\lessdot", "\\lesseqgtr", "\\lesseqqgtr", "\\lessgtr", "\\lesssim", "\\lfloor", "\\lg", "\\lgroup", "\\lhd", "\\lim", "\\liminf", "\\lim\\limits_x", "\\limsup", "\\ll", "{=}\\llap{/\\,}", "\\llbracket", "\\llcorner", "\\Lleftarrow", "\\lll", "\\llless", "\\lmoustache", "\\ln", "\\lnapprox", "\\lneq", "\\lneqq", "\\lnot", "\\lnsim", "\\log", "\\Longleftarrow", "\\longleftarrow", "\\Longleftrightarrow", "\\longleftrightarrow", "\\longmapsto", "\\Longrightarrow", "\\longrightarrow", "\\looparrowleft", "\\looparrowright", "\\lor", "\\lozenge", "\\lparen", "\\Lrarr", "\\lrArr", "\\lrarr", "\\lrcorner", "\\lq", "\\Lsh", "\\lt", "\\ltimes", "\\lVert", "\\lvert", "\\lvertneqq", "\\maltese", "\\mapsto", "\\mathbb{AB}", "\\mathbf{AaBb123}", "a\\mathbin{!}b", "\\mathcal{AaBb123}", "a\\mathchoice{\\,}{\\,\\,}{\\,\\,\\,}{\\,\\,\\,\\,}b", "\\sum_{\\mathclap{1\\le i\\le n}} x_{i}", "a + (b\\mathclose\\gt + c", "\\mathellipsis", "\\mathfrak{AaBb}", "ab\\mathinner{\\text{inside}}cd", "\\mathit{AaBb}", "{=}\\mathllap{/\\,}", "\\mathnormal{AaBb}", "\\mathop{\\star}_a^b", "\\{x∈ℝ\\mid x>0\\}", "P\\left(A\\middle\\vert B\\right)", "\\min", "\\minuscolon", "\\minuscoloncolon", "\\minuso", "a\\mkern18mu b", "3\\equiv 5 \\mod 2", "\\models", "\\mp", "a\\mskip{10mu}b", "\\Mu", "\\mu", "\\multimap", "\\N", "\\nabla", "\\natnums", "\\natural", "a\\negmedspace b", "\\ncong", "\\ne", "\\nearrow", "\\neg", "a\\negthickspace b", "a\\negthinspace b", "\\neq", "\\newcommand\\chk{\\checkmark} \\chk", "a\\newline b", "\\nexists", "\\ngeq", "\\ngeqq", "\\ngeqslant", "\\ngtr", "\\ni", "\\nleftarrow", "\\nLeftarrow", "\\nLeftrightarrow", "\\nleftrightarrow", "\\nleq", "\\nleqq", "\\nleqslant", "\\nless", "\\nmid", "a\\nobreakspace b", "\\lim\\nolimits_x", "\\begin{align*} a&=b+c \\\\ d+e&=f \\end{align*}", "\\normalsize normalsize", "\\not =", "\\begin{align*} a&=b+c \\\\ d+e&=f \\end{align*}", "\\notin", "\\notni", "\\nparallel", "\\nprec", "\\npreceq", "\\nRightarrow", "\\nrightarrow", "\\nshortmid", "\\nshortparallel", "\\nsim", "\\nsubseteq", "\\nsubseteqq", "\\nsucc", "\\nsucceq", "\\nsupseteq", "\\nsupseteqq", "\\ntriangleleft", "\\ntrianglelefteq", "\\ntriangleright", "\\ntrianglerighteq", "\\Nu", "\\nu", "\\nVDash", "\\nVdash", "\\nvDash", "\\nvdash", "\\nwarrow", "\\text{\\O}", "\\text{\\o}", "\\odot", "\\text{\\OE}", "\\text{\\oe}", "\\oiiint", "\\oiint", "\\oint", "\\omega", "\\Omega", "\\Omicron", "\\omicron", "\\ominus", "\\operatorname{asin} x", "\\operatorname*{asin}\\limits_y x", "\\operatornamewithlimits{asin}\\limits_y x", "\\oplus", "\\origof", "\\oslash", "\\otimes", "{a+1 \\over b+2}+c", "\\overbrace{x+⋯+x}^{n\\text{ times}}", "\\overgroup{AB}", "\\overleftarrow{AB}", "\\overleftharpoon{AB}", "\\overleftrightarrow{AB}", "\\overline{\\text{a long argument}}", "\\overlinesegment{AB}", "\\Overrightarrow{AB}", "\\overrightarrow{AB}", "\\overrightharpoon{ac}", "\\overset{!}{=}", "\\owns", "\\text{\\P}", "\\parallel", "\\partial", "\\perp", "\\Gamma^{\\phantom{i}j}_{i\\phantom{j}k}", "\\phase{-78^\\circ}", "\\Phi", "\\phi", "\\Pi", "\\pi", "\\pitchfork", "\\plim", "\\plusmn", "\\pm", "\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}", "\\begin{pmatrix*}[r] 0 & -1 \\\\ -1 & 0 \\end{pmatrix*}", "\\pmb{\\mu}", "x\\pmod a", "x \\pod a", "\\pounds", "\\Pr", "\\prec", "\\precapprox", "\\preccurlyeq", "\\preceq", "\\precnapprox", "\\precneqq", "\\precnsim", "\\precsim", "\\prime", "\\prod", "\\projlim", "\\propto", "\\providecommand\\greet{\\text{Hello}} \\greet", "\\psi", "\\Psi", "\\pu{123 kJ//mol}", "a\\qquad\\qquad{b}", "a\\quad\\quad{b}", "\\R", "\\text{\\r{a}}", "h\\raisebox{2pt}{ighe}r", "\\langle A\\rang", "\\Rarr", "\\rArr", "\\rarr", "\\ratio", "\\rBrace", "\\rbrace", "\\rbrack", "\\begin{rcases} a &\\text{if } b \\\\ c &\\text{if } d \\end{rcases}", "\\rceil", "\\Re", "\\real", "\\Reals", "\\reals", "\\def\\hail{Hi!} \\renewcommand\\hail{\\text{Ahoy!}} \\hail", "\\restriction", "\\rfloor", "\\rgroup", "\\rhd", "\\Rho", "\\rho", "\\left.\\dfrac a b\\right)", "\\Rightarrow", "\\rightarrow", "\\rightarrowtail", "\\rightharpoondown", "\\rightharpoonup", "\\rightleftarrows", "\\rightleftharpoons", "\\rightrightarrows", "\\rightsquigarrow", "\\rightthreetimes", "\\risingdotseq", "\\rlap{\\,/}{=}", "\\rm AaBb12", "\\rmoustache", "\\rparen", "\\rq", "\\rrbracket", "\\Rrightarrow", "\\Rsh", "\\rtimes", "x\\rule[6pt]{2ex}{1ex}x", "\\rVert", "\\rvert", "\\text{\\S}", "\\scriptscriptstyle \\frac cd", "\\scriptsize scriptsize", "\\frac ab + {\\scriptstyle \\frac cd}", "\\sdot", "\\searrow", "\\sec", "\\text{\\sect}", "\\set{x|x<5}", "\\Set{ x | x<\\frac 1 2 }", "\\setminus", "\\sf AaBb123", "\\sharp", "\\shortmid", "\\shortparallel", "\\Sigma", "\\sigma", "\\sim", "\\simcolon", "\\simcoloncolon", "\\simeq", "\\sin", "\\sinh", "\\sixptsize sixptsize", "\\sh", "\\small small", "\\smallfrown", "\\smallint", "\\begin{smallmatrix} a & b \\\\ c & d \\end{smallmatrix}", "\\smallsetminus", "\\smallsmile", "\\left(x^{\\smash{2}}\\right)", "\\smile", "\\sout{abc}", "a\\space b", "\\spades", "\\spadesuit", "\\sphericalangle", "\\begin{equation} \\begin{split} a &=b+c\\\\ &=e+f \\end{split} \\end{equation}", "\\sqcap", "\\sqcup", "\\square", "\\sqrt[3]{x}", "\\sqsubset", "\\sqsubseteq", "\\sqsupset", "\\sqsupseteq", "\\text{\\ss}", "\\stackrel{!}{=}", "\\star", "\\sub", "\\sube", "\\Subset", "\\subset", "\\subseteq", "\\subseteqq", "\\subsetneq", "\\subsetneqq", "\\sum_{\\substack{0\\le i\\le n \\\\ i\\text{ even}}} x_i", "\\text{\\textgreater}", "\\textit{AaBb}", "\\text{\\textless}", "\\textmd{AaBb123}", "\\textnormal{AB}", "\\text{\\textquotedblleft}", "\\text{\\textquotedblright}", "\\text{\\textquoteleft}", "\\text{\\textquoteright}", "\\text{\\textregistered}", "\\textrm{AaBb123}", "\\textsf{AaBb123}", "\\text{\\textsterling}", "\\textstyle\\sum_0^n", "\\texttt{AaBb123}", "\\text{\\textunderscore}", "\\textup{AaBb123}", "\\tfrac ab", "\\tg", "\\th", "\\therefore", "\\Theta", "\\theta", "\\thetasym", "\\thickapprox", "\\thicksim", "a\\thickspace b", "a\\thinspace b", "\\tilde M", "\\times", "\\tiny tiny", "\\to", "\\top", "\\triangle", "\\triangledown", "\\triangleleft", "\\trianglelefteq", "\\triangleq", "\\triangleright", "\\trianglerighteq", "{\\tt AaBb123}", "\\twoheadleftarrow", "\\twoheadrightarrow", "\\text{\\u{a}}", "\\Uarr", "\\uArr", "\\uarr", "\\ulcorner", "\\underbar{X}", "\\underbrace{x+⋯+x}_{n\\text{ times}}", "\\undergroup{AB}", "\\underleftarrow{AB}", "\\underleftrightarrow{AB}", "\\underrightarrow{AB}", "\\underline{\\text{a long argument}}", "\\underlinesegment{AB}", "\\underset{!}{=}", "\\unlhd", "\\unrhd", "\\Uparrow", "\\uparrow", "\\Updownarrow", "\\updownarrow", "\\upharpoonleft", "\\upharpoonright", "\\uplus", "\\Upsilon", "\\upsilon", "\\upuparrows", "\\urcorner", "\\url{https://katex.org/}", "\\utilde{AB}", "\\text{\\v{a}}", "\\varDelta", "\\varepsilon", "\\varGamma", "\\varinjlim", "\\varkappa", "\\varLambda", "\\varliminf", "\\varlimsup", "\\varnothing", "\\varOmega", "\\varPhi", "\\varphi", "\\varPi", "\\varpi", "\\varprojlim", "\\varpropto", "\\varPsi", "\\varrho", "\\varSigma", "\\varsigma", "\\varsubsetneq", "\\varsubsetneqq", "\\varsupsetneq", "\\varsupsetneqq", "\\varTheta", "\\vartheta", "\\vartriangle", "\\vartriangleleft", "\\vartriangleright", "\\varUpsilon", "\\varXi", "\\mathrel{\\vcentcolon =}", "a+\\left(\\vcenter{\\frac{\\frac a b}c}\\right)", "\\Vdash", "\\vDash", "\\vdash", "\\vdots", "\\vec{F}", "\\vee", "\\veebar", "\\verb!\\frac a b!", "\\Vert", "\\vert", "\\begin{Vmatrix} a & b \\\\ c & d \\end{Vmatrix}", "\\begin{Vmatrix*}[r] 0 & -1 \\\\ -1 & 0 \\end{Vmatrix*}", "\\begin{vmatrix} a & b \\\\ c & d \\end{vmatrix}", "\\begin{vmatrix*}[r] 0 & -1 \\\\ -1 & 0 \\end{vmatrix*}", "\\overline{\\vphantom{M}a}", "\\Vvdash", "\\wedge", "\\weierp", "\\widecheck{AB}", "\\widehat{AB}", "\\widetilde{AB}", "\\wp", "\\wr", "\\xcancel{ABC}", "\\def\\foo{a}\\xdef\\fcopy{\\foo}\\def\\foo{}\\fcopy", "\\Xi", "\\xi", "\\xhookleftarrow{abc}", "\\xhookrightarrow{abc}", "\\xLeftarrow{abc}", "\\xleftarrow{abc}", "\\xleftharpoondown{abc}", "\\xleftharpoonup{abc}", "\\xLeftrightarrow{abc}", "\\xleftrightarrow{abc}", "\\xleftrightharpoons{abc}", "\\xlongequal{abc}", "\\xmapsto{abc}", "\\xRightarrow{abc}", "\\xrightarrow{abc}", "\\xrightharpoondown{abc}", "\\xrightharpoonup{abc}", "\\xrightleftharpoons{abc}", "\\xtofrom{abc}", "\\xtwoheadleftarrow{abc}", "\\xtwoheadrightarrow{abc}", "\\yen", "\\Z", "\\Zeta", "\\zeta", "\\hat{M}", "\\hat{b}", "\\hat{\\aa}", "\\hat{o}", "\\tilde{M}", "\\left\\{\\begin{array}{l} \\nabla\\cdot\\vec{D} = \\rho \\\\ \\nabla\\cdot\\vec{B} = 0 \\\\ \\nabla\\times\\vec{E} = -\\frac{\\partial\\vec{B}}{\\partial t} \\\\ \\nabla\\times\\vec{H} = \\vec{J}_f + \\frac{\\partial\\vec{D}}{\\partial t} \\end{array}\\right.", "酸 + 碱 \\rightarrow 盐 + 水"];
const GOLDEN_SCORES = {"1":0.889,"2":0.914,"3":0.756,"4":0.898,"5":0.875,"6":0.915,"7":0.675,"8":0.853,"9":0.909,"10":0.643,"11":0.905,"12":0.643,"13":0.911,"14":0.906,"15":0.88,"16":0.911,"17":0.911,"18":0.898,"19":0.866,"20":0.65,"21":0.915,"22":0.69,"23":0.899,"24":0.69,"25":0.756,"26":0.732,"27":0.879,"28":0.691,"29":0.884,"30":0.922,"31":0.926,"32":0.926,"33":0.926,"34":0.683,"35":0.605,"36":0.605,"37":0.476,"38":0.558,"39":0.558,"40":0.887,"41":0.876,"42":0.907,"43":0.904,"44":0.825,"45":0.855,"46":0.8,"47":0.894,"48":0.884,"49":0.921,"50":0.611,"51":0.578,"52":0.903,"53":0.905,"54":0.825,"55":0.781,"56":0.906,"57":0.884,"58":0.811,"59":0.908,"60":0.904,"61":0.387,"62":0.911,"63":0.886,"64":0.902,"65":0.818,"66":0.9,"67":0.926,"68":0.878,"69":0.919,"70":0.77,"71":0.885,"72":0.792,"73":0.915,"74":0.782,"75":0.771,"76":0.9,"77":0.827,"78":0.892,"79":0.884,"80":0.898,"81":0.922,"82":0.935,"83":0.918,"84":0.928,"85":0.889,"86":0.933,"87":0.939,"88":0.839,"89":0.923,"90":0.847,"91":0.689,"92":0.473,"93":0.935,"94":0.87,"95":0.93,"96":0.943,"97":0.532,"98":0.534,"99":0.951,"100":0.954,"101":0.95,"102":0.911,"103":0.875,"104":0.929,"105":0.932,"106":0.796,"107":0.919,"108":0.935,"109":0.937,"110":0.935,"111":0.53,"112":0.956,"113":0.954,"114":0.974,"115":0.953,"116":0.939,"117":0.931,"118":0.853,"119":0.902,"120":0.525,"121":0.901,"122":0.509,"123":0.909,"124":0.743,"125":0.853,"126":0.909,"127":0.901,"128":0.875,"129":0.876,"130":0.783,"131":0.88,"132":0.883,"133":0.882,"134":0.832,"135":0.832,"136":0.852,"137":0.521,"138":0.434,"139":0.525,"140":0.667,"141":0.964,"142":0.964,"143":0.932,"144":0.935,"145":0.771,"146":0.792,"147":0.919,"148":0.936,"149":0.835,"151":0.78,"152":0.78,"153":0.924,"155":0.914,"156":0.476,"157":0.39,"158":0.549,"159":0.892,"160":0.927,"161":0.825,"162":0.846,"163":0.893,"164":0.913,"165":0.939,"166":0.821,"167":0.822,"168":0.889,"169":0.88,"170":0.873,"171":0.916,"172":0.906,"173":0.956,"174":0.956,"175":0.869,"176":0.794,"177":0.601,"178":0.627,"179":0.822,"180":0.601,"181":0.592,"182":0.684,"183":0.68,"184":0.684,"185":0.634,"186":0.547,"187":0.592,"188":0.547,"189":0.634,"190":0.68,"191":0.648,"192":0.788,"193":0.98,"194":0.946,"195":0.869,"196":0.876,"197":0.975,"198":0.741,"199":0.914,"200":0.906,"201":0.895,"202":0.851,"203":0.911,"204":0.914,"205":0.915,"206":0.91,"207":0.902,"208":0.912,"209":0.92,"210":0.938,"211":0.83,"212":0.861,"213":0.908,"214":0.909,"215":0.701,"216":0.758,"217":0.923,"218":0.924,"219":0.923,"220":0.903,"221":0.88,"222":0.88,"223":0.829,"224":0.904,"225":0.93,"226":0.92,"227":0.88,"228":0.53,"229":0.822,"230":0.835,"231":0.924,"232":0.924,"233":0.579,"234":0.619,"235":0.644,"236":0.832,"237":0.672,"238":0.884,"239":0.881,"240":0.911,"241":0.683,"242":0.899,"243":0.903,"244":0.862,"245":0.902,"246":0.808,"247":0.862,"248":0.85,"249":0.79,"250":0.79,"251":0.909,"252":0.667,"253":0.856,"254":0.847,"255":0.643,"256":0.944,"257":0.911,"258":0.944,"259":0.961,"260":0.787,"261":0.787,"262":0.608,"263":0.479,"264":0.674,"265":0.916,"266":0.856,"267":0.919,"268":0.92,"269":0.88,"270":0.829,"271":0.925,"272":0.912,"273":0.891,"274":0.835,"275":0.898,"276":0.886,"277":0.643,"278":0.917,"279":0.917,"280":0.91,"281":0.81,"282":0.905,"283":0.882,"284":0.646,"285":0.61,"286":0.397,"287":0.821,"288":0.577,"289":0.538,"290":0.888,"291":0.906,"292":0.915,"293":0.538,"294":0.577,"295":0.874,"296":0.822,"297":0.895,"298":0.899,"299":0.884,"300":0.884,"301":0.857,"302":0.844,"303":0.886,"304":0.535,"305":0.816,"306":0.943,"307":0.579,"308":0.848,"309":0.849,"310":0.854,"311":0.879,"312":0.861,"313":0.623,"314":0.889,"315":0.467,"316":0.84,"317":0.918,"318":0.599,"319":0.852,"320":0.926,"321":0.497,"322":0.926,"323":0.865,"324":0.91,"325":0.917,"326":0.902,"327":0.881,"328":0.881,"329":0.897,"330":0.918,"331":0.891,"332":0.927,"333":0.889,"334":0.859,"335":0.612,"336":0.876,"337":0.818,"338":0.898,"339":0.92,"340":0.901,"341":0.835,"342":0.838,"343":0.894,"344":0.671,"345":0.848,"346":0.848,"347":0.873,"348":0.715,"349":0.934,"350":0.89,"351":0.782,"352":0.879,"353":0.879,"354":0.759,"355":0.847,"356":0.878,"357":0.854,"358":0.905,"359":0.138,"360":0.771,"361":0.93,"362":0.903,"363":0.885,"364":0.885,"365":0.885,"366":0.885,"367":0.91,"368":0.628,"369":0.883,"370":0.892,"371":0.951,"372":0.958,"373":0.854,"374":0.854,"375":0.163,"376":0.788,"377":0.739,"378":0.743,"379":0.825,"381":0.898,"382":0.85,"383":0.85,"384":0.861,"385":0.944,"386":0.956,"387":0.944,"388":0.892,"389":0.901,"390":0.825,"391":0.911,"392":0.892,"393":0.885,"394":0.901,"395":0.827,"396":0.884,"397":0.668,"398":0.879,"399":0.668,"400":0.826,"401":0.826,"402":0.646,"403":0.917,"404":0.908,"405":0.883,"406":0.883,"407":0.837,"408":0.837,"409":0.917,"410":0.675,"411":0.843,"412":0.834,"413":0.67,"414":0.655,"415":0.94,"416":0.79,"417":0.932,"418":0.775,"419":0.916,"420":0.928,"421":0.808,"422":0.625,"423":0.917,"424":0.837,"425":0.837,"426":0.815,"427":0.88,"428":0.863,"429":0.848,"430":0.873,"431":0.918,"432":0.918,"433":0.788,"434":0.855,"435":0.928,"436":0.874,"437":0.913,"438":0.903,"439":0.821,"440":0.922,"441":0.906,"442":0.825,"443":0.847,"444":0.905,"445":0.894,"446":0.941,"447":0.852,"448":0.915,"449":0.812,"450":0.787,"451":0.834,"452":0.819,"453":0.924,"454":0.813,"455":0.928,"456":0.912,"457":0.887,"458":0.887,"459":0.919,"460":0.902,"461":0.894,"462":0.93,"463":0.894,"464":0.92,"465":0.868,"466":0.888,"467":0.907,"468":0.904,"469":0.888,"470":0.841,"471":0.838,"472":0.886,"473":0.852,"474":0.891,"475":0.892,"476":0.914,"477":0.862,"478":0.814,"479":0.848,"480":0.848,"481":0.873,"482":0.928,"483":0.92,"484":0.841,"485":0.823,"486":0.829,"487":0.834,"488":0.737,"489":0.899,"490":0.926,"491":0.856,"492":0.847,"493":0.743,"494":0.92,"495":0.771,"496":0.91,"497":0.562,"498":0.752,"499":0.916,"500":0.854,"501":0.901,"502":0.911,"503":0.924,"504":0.868,"505":0.861,"506":0.699,"507":0.572,"508":0.829,"509":0.61,"510":0.646,"511":0.542,"512":0.91,"513":0.884,"514":0.895,"515":0.912,"516":0.908,"517":0.899,"518":0.829,"519":0.811,"520":0.731,"521":0.945,"522":0.731,"523":0.897,"524":0.908,"525":0.924,"526":0.912,"527":0.903,"528":0.92,"529":0.915,"530":0.914,"531":0.912,"532":0.927,"533":0.748,"534":0.864,"535":0.925,"536":0.917,"537":0.914,"538":0.855,"539":0.807,"540":0.82,"541":0.911,"542":0.891,"543":0.781,"544":0.927,"545":0.919,"546":0.914,"547":0.859,"548":0.819,"549":0.909,"550":0.756,"551":0.621,"552":0.791,"553":0.912,"554":0.621,"555":0.92,"556":0.93,"557":0.861,"558":0.853,"559":0.922,"560":0.916,"561":0.769,"562":0.891,"563":0.901,"564":0.902,"565":0.921,"566":0.885,"567":0.86,"568":0.92,"569":0.924,"570":0.881,"571":0.856,"572":0.927,"573":0.871,"574":0.927,"575":0.801,"576":0.854,"577":0.857,"578":0.846,"579":0.857,"580":0.84,"581":0.844,"582":0.897,"583":0.877,"584":0.886,"585":0.88,"586":0.925,"587":0.913,"588":0.852,"589":0.939,"590":0.897,"591":0.657,"592":0.855,"593":0.874,"594":0.894,"595":0.765,"596":0.6,"597":0.6,"598":0.895,"599":0.163,"600":0.885,"601":0.887,"602":0.773,"603":0.654,"604":0.658,"605":0.622,"606":0.615,"607":0.629,"608":0.654,"609":0.617,"610":0.782,"611":0.618,"612":0.572,"613":0.717,"614":0.807,"615":0.905,"616":0.834,"617":0.91,"618":0.909,"619":0.714,"620":0.705,"621":0.815,"622":0.89,"623":0.667,"624":0.879,"625":0.919,"626":0.856,"627":0.813,"628":0.813,"629":0.926,"630":0.511,"631":0.842,"632":0.717,"633":0.762,"634":0.886,"635":0.893,"636":0.814,"637":0.902,"638":0.921,"639":0.935,"640":0.855,"641":0.892,"642":0.86,"643":0.843,"644":0.896,"645":0.976,"646":0.796,"647":0.867,"648":0.874,"649":0.874,"650":0.708,"652":0.91,"653":0.909,"654":0.903,"655":0.732,"656":0.887,"657":0.883,"658":0.823,"659":0.823,"660":0.862,"661":0.835,"662":0.418,"663":0.936,"664":0.823,"665":0.835,"666":0.873,"667":0.918,"668":0.918,"669":0.903,"670":0.903,"671":0.884,"672":0.877,"673":0.851,"674":0.917,"675":0.859,"676":0.903,"677":0.834,"678":0.625,"679":0.823,"680":0.862,"681":0.819,"682":0.819,"683":0.881,"684":0.92,"685":0.9,"686":0.802,"687":0.808,"688":0.85,"689":0.845,"690":0.896,"691":0.925,"692":0.918,"693":0.912,"694":0.872,"695":0.85,"696":0.884,"697":0.855,"698":0.831,"699":0.378,"700":0.834,"701":0.737,"702":0.838,"703":0.836,"704":0.898,"705":0.652,"706":0.78,"707":0.871,"708":0.907,"709":0.838,"710":0.763,"711":0.658,"712":0.919,"713":0.916,"714":0.883,"715":0.71,"716":0.903,"717":0.613,"718":0.919,"719":0.888,"720":0.645,"721":0.67,"722":0.878,"723":0.913,"724":0.885,"725":0.835,"726":0.884,"727":0.87,"728":0.748,"729":0.929,"730":0.414,"731":0.78,"732":0.83,"733":0.754,"734":0.92,"735":0.596,"736":0.909,"737":0.955,"738":0.955,"739":0.906,"740":0.606,"741":0.832,"742":0.808,"743":0.875,"744":0.703,"745":0.895,"746":0.943,"747":0.883,"748":0.942,"749":0.902,"750":0.717,"751":0.886,"752":0.824,"753":0.937,"754":0.891,"755":0.824,"756":0.937,"757":0.92,"758":0.932,"759":0.899,"760":0.792,"761":0.808,"762":0.705,"763":0.823,"764":0.92,"765":0.854,"766":0.879,"767":0.873,"768":0.92,"769":0.909,"770":0.375,"771":0.92,"772":0.63,"773":0.886,"774":0.807,"775":0.606,"776":0.678,"777":0.92,"778":0.853,"779":0.894,"780":0.888,"781":0.779,"782":0.768,"783":0.896,"784":0.894,"785":0.901,"786":0.901,"787":0.908,"788":0.91,"789":0.639,"790":0.83,"791":0.827,"792":0.862,"793":0.908,"794":0.919,"795":0.902,"796":0.873,"797":0.925,"798":0.853,"799":0.873,"800":0.934,"801":0.918,"802":0.873,"803":0.807,"804":0.721,"805":0.877,"806":0.877,"807":0.82,"808":0.874,"809":0.794,"810":0.392,"811":0.863,"812":0.927,"813":0.887,"814":0.895,"815":0.647,"816":0.858,"817":0.768,"818":0.925,"819":0.934,"820":0.877,"821":0.82,"822":0.897,"823":0.876,"824":0.92,"825":0.877,"826":0.928,"827":0.722,"828":0.768,"829":0.906,"830":0.809,"831":0.044,"832":0.594,"833":0.569,"834":0.909,"835":0.922,"836":0.904,"837":0.579,"838":0.936,"839":0.839,"840":0.571,"841":0.578,"842":0.827,"843":0.839,"844":0.887,"845":0.898,"846":0.892,"847":0.9,"848":0.609,"849":0.856,"850":0.851,"851":0.815,"852":0.838,"853":0.876,"854":0.836,"855":0.87,"856":0.835,"857":0.865,"858":0.85,"859":0.894,"860":0.922,"861":0.852,"862":0.859,"863":0.901,"864":0.874,"865":0.541,"866":0.582,"867":0.902,"868":0.874,"869":0.88,"870":0.877,"871":0.667,"872":0.914,"873":0.898,"874":0.832,"875":0.834,"876":0.737,"877":0.511,"878":0.402,"879":0.487,"880":0.379,"881":0.881,"882":0.905,"883":0.908,"884":0.816,"885":0.567,"886":0.56,"887":0.565,"888":0.816,"889":0.908,"890":0.636,"891":0.898,"892":0.716,"893":0.909,"894":0.619,"895":0.592,"896":0.639,"897":0.632,"898":0.625,"899":0.61,"900":0.609,"901":0.611,"902":0.642,"903":0.64,"904":0.596,"905":0.594,"906":0.586,"907":0.6,"908":0.588,"909":0.64,"910":0.63,"911":0.626,"912":0.585,"913":0.908,"914":0.891,"915":0.876,"916":0.916,"917":0.605,"918":0.772,"919":0.723,"920":0.736,"921":0.639};

// ── score → tier ──
function tier(s) {
  if (s === null) return 'nodata';
  if (s >= 0.8)  return 'great';
  if (s >= 0.6)  return 'good';
  if (s >= 0.4)  return 'ok';
  if (s >= 0.3)  return 'low';
  return 'bad';
}
function tierClass(t) {
  const map = {
    great: 'bg-emerald-50 text-emerald-800 border border-emerald-200',
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
  return { great:'Great', good:'Good', ok:'Fair',
           low:'Low', bad:'Poor', nodata:'—', err:'Error' }[t];
}

// ── counters ──
let total = FORMULAS.length;
let rendered = 0;    // WASM render done
let wasmOk = 0, wasmErr = 0;

function counterTier(score) {
  if (score === null) return 'low';
  if (score < 0.3) return 'low';
  if (score >= 0.8) return 'great';
  if (score >= 0.5) return 'mid-hi';
  return 'mid-lo';
}

function refreshHero() {
  const byTier = { great: 0, 'mid-hi': 0, 'mid-lo': 0, low: 0 };
  let scoreSum = 0, scoreN = 0;
  for (let i = 1; i <= total; i++) {
    const s = GOLDEN_SCORES[i] ?? null;
    if (s !== null) { scoreSum += s; scoreN++; }
    byTier[counterTier(s)]++;
  }
  document.getElementById('cnt-great').textContent = byTier.great;
  document.getElementById('cnt-mid-lo').textContent = byTier['mid-lo'];
  document.getElementById('cnt-mid-hi').textContent = byTier['mid-hi'];
  document.getElementById('cnt-low').textContent   = byTier.low;
  document.getElementById('cnt-avg').textContent   = scoreN ? (scoreSum/scoreN).toFixed(2) : '—';
  document.getElementById('b-great').textContent   = byTier.great;
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
    tr.dataset.tier = (score === null || score < 0.3) ? 'low'
      : score >= 0.8 ? 'great'
      : score >= 0.5 ? 'mid-hi'
      : 'mid-lo';
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
document.getElementById('katex-script').addEventListener('load', () => {
  buildTable();
  loadFontsAndWasm();
});

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
