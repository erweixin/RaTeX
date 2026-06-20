// Comprehensive render benchmark: PNG / SVG / SVG-standalone / PDF — 100 formulas
// Run: cargo test --package ratex-render --test bench_render --release -- --nocapture

use std::time::Instant;

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parser::parse;
use ratex_render::{render_to_png, RenderOptions};

fn font_dir() -> String {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fonts")
        .to_string_lossy()
        .to_string()
}

#[derive(Default, Clone)]
#[allow(dead_code)]
struct BenchResult {
    category: &'static str,
    formula: String,
    glyph_count: usize,
    parse_layout_us: u128,
    png_us: u128,
    svg_us: u128,
    svg_standalone_us: u128,
    pdf_us: u128,
}

fn bench_formula(
    category: &'static str,
    label: &str,
    expr: &str,
    render_opts: &RenderOptions,
    warmup: u32,
    iters: u32,
) -> BenchResult {
    let svg_opts = ratex_svg::SvgOptions {
        font_size: render_opts.font_size as f64,
        padding: render_opts.padding as f64,
        stroke_width: 1.5,
        embed_glyphs: false,
        font_dir: render_opts.font_dir.clone(),
    };
    let svg_standalone_opts = ratex_svg::SvgOptions {
        embed_glyphs: true,
        ..svg_opts.clone()
    };
    let pdf_opts = ratex_pdf::PdfOptions {
        font_size: render_opts.font_size as f64,
        padding: render_opts.padding as f64,
        stroke_width: 1.5,
        font_dir: render_opts.font_dir.clone(),
    };
    let layout_opts = LayoutOptions::default();

    // Warmup
    for _ in 0..warmup {
        let ast = parse(expr).expect("parse");
        let l = layout(&ast, &layout_opts);
        let dl = to_display_list(&l);
        let _ = render_to_png(&dl, render_opts);
        let _ = ratex_svg::render_to_svg(&dl, &svg_opts);
        let _ = ratex_svg::render_to_svg(&dl, &svg_standalone_opts);
        let _ = ratex_pdf::render_to_pdf(&dl, &pdf_opts);
    }

    let mut total_parse_layout = 0u128;
    let mut total_png = 0u128;
    let mut total_svg = 0u128;
    let mut total_svg_standalone = 0u128;
    let mut total_pdf = 0u128;
    let mut glyph_count = 0usize;

    for _ in 0..iters {
        let t0 = Instant::now();
        let ast = parse(expr).expect("parse");
        let l = layout(&ast, &layout_opts);
        let dl = to_display_list(&l);
        total_parse_layout += t0.elapsed().as_micros();

        glyph_count = dl
            .items
            .iter()
            .filter(|i| matches!(i, ratex_types::display_item::DisplayItem::GlyphPath { .. }))
            .count();

        let t1 = Instant::now();
        let _ = render_to_png(&dl, render_opts);
        total_png += t1.elapsed().as_micros();

        let t2 = Instant::now();
        let _ = ratex_svg::render_to_svg(&dl, &svg_opts);
        total_svg += t2.elapsed().as_micros();

        let t3 = Instant::now();
        let _ = ratex_svg::render_to_svg(&dl, &svg_standalone_opts);
        total_svg_standalone += t3.elapsed().as_micros();

        let t4 = Instant::now();
        let _ = ratex_pdf::render_to_pdf(&dl, &pdf_opts);
        total_pdf += t4.elapsed().as_micros();
    }

    BenchResult {
        category,
        formula: label.to_string(),
        glyph_count,
        parse_layout_us: total_parse_layout / iters as u128,
        png_us: total_png / iters as u128,
        svg_us: total_svg / iters as u128,
        svg_standalone_us: total_svg_standalone / iters as u128,
        pdf_us: total_pdf / iters as u128,
    }
}

/// Build 100 test formulas across categories.
fn build_formulas() -> Vec<(&'static str, &'static str, &'static str)> {
    let mut v = Vec::with_capacity(100);

    // ── Category 1: Simple math (25) ──
    let simple: &[(&str, &str)] = &[
        ("x^2 + y^2 = z^2", r"x^2 + y^2 = z^2"),
        ("a+b=c", r"a+b=c"),
        ("E=mc^2", r"E=mc^2"),
        ("sin^2+cos^2=1", r"\sin^2\theta + \cos^2\theta = 1"),
        ("e^{i\\pi}+1=0", r"e^{i\pi} + 1 = 0"),
        ("a^2-b^2=(a+b)(a-b)", r"a^2 - b^2 = (a+b)(a-b)"),
        ("\\sqrt{a^2+b^2}", r"\sqrt{a^2 + b^2}"),
        ("\\frac{dy}{dx}", r"\frac{dy}{dx}"),
        (
            "x_{1,2} formula",
            r"x_{1,2} = \frac{-b \pm \sqrt{b^2-4ac}}{2a}",
        ),
        ("F=ma", r"F = ma"),
        (
            "a\\cdot b = |a||b|\\cos\\theta",
            r"a \cdot b = |a||b|\cos\theta",
        ),
        (
            "\\lim_{x\\to 0}\\frac{\\sin x}{x}",
            r"\lim_{x\to 0}\frac{\sin x}{x}",
        ),
        ("\\int_a^b f(x)dx", r"\int_a^b f(x)\,dx"),
        ("\\sum_{i=1}^{n} i", r"\sum_{i=1}^{n} i = \frac{n(n+1)}{2}"),
        ("\\prod_{i=1}^{n} a_i", r"\prod_{i=1}^{n} a_i"),
        ("\\binom{n}{k}", r"\binom{n}{k} = \frac{n!}{k!(n-k)!}"),
        (
            "\\vec{F} = q(\\vec{E} + \\vec{v}\\times\\vec{B})",
            r"\vec{F} = q(\vec{E} + \vec{v}\times\vec{B})",
        ),
        (
            "\\nabla\\cdot\\vec{E}=\\rho/\\varepsilon_0",
            r"\nabla\cdot\vec{E} = \frac{\rho}{\varepsilon_0}",
        ),
        ("\\infty+1=\\infty", r"\infty + 1 = \infty"),
        ("\\partial f/\\partial x", r"\frac{\partial f}{\partial x}"),
        (
            "\\oint_C \\vec{F}\\cdot d\\vec{r}",
            r"\oint_C \vec{F}\cdot d\vec{r}",
        ),
        (
            "\\int_{-\\infty}^{\\infty}e^{-x^2}dx",
            r"\int_{-\infty}^{\infty} e^{-x^2} dx = \sqrt{\pi}",
        ),
        ("\\hat{H}\\psi=E\\psi", r"\hat{H}\psi = E\psi"),
        ("A\\subseteq B", r"A \subseteq B \implies P(A)\leq P(B)"),
        (
            "\\lfloor x\\rfloor\\le x<\\lceil x\\rceil",
            r"\lfloor x\rfloor \leq x < \lceil x\rceil",
        ),
    ];
    for &(l, e) in simple {
        v.push(("math", l, e));
    }

    // ── Category 2: Complex math (20) ──
    let complex: &[(&str, &str)] = &[
        ("\\frac{a}{b}+\\int_0^\\infty e^{-x}dx+\\sum_{i=1}^n i\\sqrt{x}",
         r"\frac{a}{b} + \int_0^\infty e^{-x} dx + \sum_{i=1}^n i \cdot \sqrt{x}"),
        ("\\sum_{k=0}^\\infty\\frac{x^k}{k!}=e^x",
         r"\sum_{k=0}^{\infty} \frac{x^k}{k!} = e^x"),
        ("\\int_{-\\infty}^{\\infty} f(x)\\delta(x-a)dx = f(a)",
         r"\int_{-\infty}^{\infty} f(x)\delta(x-a)dx = f(a)"),
        ("\\frac{d}{dx}\\int_a^x f(t)dt = f(x)",
         r"\frac{d}{dx}\int_a^x f(t)dt = f(x)"),
        ("\\lim_{n\\to\\infty}\\left(1+\\frac{1}{n}\\right)^n",
         r"\lim_{n\to\infty}\left(1+\frac{1}{n}\right)^n = e"),
        ("\\det(A) = \\sum_{\\sigma\\in S_n} \\operatorname{sgn}(\\sigma)\\prod_{i=1}^n a_{i,\\sigma(i)}",
         r"\det(A) = \sum_{\sigma\in S_n} \operatorname{sgn}(\sigma)\prod_{i=1}^n a_{i,\sigma(i)}"),
        ("\\iiint_V \\nabla\\cdot\\vec{F}\\,dV = \\oiint_S \\vec{F}\\cdot d\\vec{S}",
         r"\iiint_V \nabla\cdot\vec{F}\,dV = \oiint_S \vec{F}\cdot d\vec{S}"),
        ("\\zeta(s)=\\sum_{n=1}^{\\infty}\\frac{1}{n^s}",
         r"\zeta(s) = \sum_{n=1}^{\infty} \frac{1}{n^s}"),
        ("\\Gamma(z)=\\int_0^\\infty t^{z-1}e^{-t}dt",
         r"\Gamma(z) = \int_0^\infty t^{z-1}e^{-t}\,dt"),
        ("e^{i\\theta}=\\cos\\theta+i\\sin\\theta",
         r"e^{i\theta} = \cos\theta + i\sin\theta"),
        ("P(A|B)=\\frac{P(B|A)P(A)}{P(B)}",
         r"P(A|B) = \frac{P(B|A)P(A)}{P(B)}"),
        ("\\mathbb{E}[X]=\\int_{-\\infty}^{\\infty}x f(x)dx",
         r"\mathbb{E}[X] = \int_{-\infty}^{\infty} x\,f(x)\,dx"),
        ("y''+\\omega^2 y = 0 \\implies y=A\\cos(\\omega t)+B\\sin(\\omega t)",
         r"y'' + \omega^2 y = 0 \implies y = A\cos(\omega t) + B\sin(\omega t)"),
        ("\\mathcal{F}[f](\\xi) = \\int_{-\\infty}^{\\infty} f(x) e^{-2\\pi i x\\xi} dx",
         r"\mathcal{F}[f](\xi) = \int_{-\infty}^{\infty} f(x) e^{-2\pi i x\xi} dx"),
        ("\\frac{\\partial^2 u}{\\partial t^2}=c^2\\nabla^2 u",
         r"\frac{\partial^2 u}{\partial t^2} = c^2 \nabla^2 u"),
        ("f(x)=\\frac{1}{\\sigma\\sqrt{2\\pi}} e^{-\\frac{(x-\\mu)^2}{2\\sigma^2}}",
         r"f(x) = \frac{1}{\sigma\sqrt{2\pi}} e^{-\frac{(x-\mu)^2}{2\sigma^2}}"),
        ("\\oint_\\gamma f(z)dz=2\\pi i\\sum_k \\operatorname{Res}(f,a_k)",
         r"\oint_\gamma f(z)dz = 2\pi i \sum_k \operatorname{Res}(f, a_k)"),
        ("\\begin{aligned} x&=a+b\\\\ y&=c+d \\end{aligned}",
         r"\begin{aligned} x &= a + b \\ y &= c + d \end{aligned}"),
        ("F_n=\\frac{\\phi^n-(-\\phi)^{-n}}{\\sqrt{5}},\\phi=\\frac{1+\\sqrt{5}}{2}",
         r"F_n = \frac{\phi^n - (-\phi)^{-n}}{\sqrt{5}},\quad \phi=\frac{1+\sqrt{5}}{2}"),
        ("\\int_0^{2\\pi}\\sin^2 x\\,dx=\\pi",
         r"\int_0^{2\pi} \sin^2 x\,dx = \pi"),
    ];
    for &(l, e) in complex {
        v.push(("complex", l, e));
    }

    // ── Category 3: Matrix / array (15) ──
    let matrix: &[(&str, &str)] = &[
        (
            "pmatrix 2x2",
            r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}",
        ),
        (
            "pmatrix 3x3",
            r"\begin{pmatrix} a_{11} & a_{12} & a_{13} \\ a_{21} & a_{22} & a_{23} \\ a_{31} & a_{32} & a_{33} \end{pmatrix}",
        ),
        (
            "bmatrix 2x2",
            r"\begin{bmatrix} 1 & 2 \\ 3 & 4 \end{bmatrix}",
        ),
        (
            "vmatrix 2x2",
            r"\begin{vmatrix} a & b \\ c & d \end{vmatrix}",
        ),
        (
            "cases defn",
            r"|x| = \begin{cases} x & x\geq 0 \\ -x & x<0 \end{cases}",
        ),
        (
            "aligned 3eq",
            r"\begin{aligned} a &= b + c \\ d &= e + f + g \\ h &= i - j \end{aligned}",
        ),
        (
            "gathered 2eq",
            r"\begin{gathered} x = a + b \\ y = c - d \end{gathered}",
        ),
        (
            "Bmatrix 2x2",
            r"\begin{Bmatrix} x & y \\ z & w \end{Bmatrix}",
        ),
        (
            "array 3col",
            r"\begin{array}{ccc} 1 & 2 & 3 \\ 4 & 5 & 6 \end{array}",
        ),
        (
            "array w/lines",
            r"\begin{array}{|c|c|} \hline a & b \\ \hline c & d \\ \hline \end{array}",
        ),
        (
            "smallmatrix 2x2",
            r"\begin{smallmatrix} 1 & 2 \\ 3 & 4 \end{smallmatrix}",
        ),
        ("\\substack sum", r"\sum_{\substack{i=1\\ j=2}}^{n} a_{ij}"),
        (
            "\\binom big",
            r"\binom{n+1}{k} = \binom{n}{k} + \binom{n}{k-1}",
        ),
        (
            "\\cfrac nested",
            r"\cfrac{1}{\sqrt{2} + \cfrac{1}{\sqrt{2} + \cfrac{1}{\sqrt{2}}}}",
        ),
        ("\\overbrace", r"\overbrace{a + b + c + d}^{n\text{ terms}}"),
    ];
    for &(l, e) in matrix {
        v.push(("matrix", l, e));
    }

    // ── Category 4: CJK text + math (15) ──
    let cjk: &[(&str, &str)] = &[
        ("你好世界", r"\text{你好世界}\quad x^2 + y^2 = z^2"),
        (
            "数学公式",
            r"\text{数学公式}\quad \frac{a}{b} + \frac{c}{d}",
        ),
        ("微积分", r"\text{微积分}\quad \int_0^\infty f(x)dx"),
        (
            "定义域值域",
            r"\text{定义域}\; D_f = \{x \in \mathbb{R} \mid x \geq 0\}",
        ),
        (
            "三角函数",
            r"\text{三角函数}\quad \sin\theta + \cos\theta = 1\text{（单位圆）}",
        ),
        (
            "方程求解",
            r"\text{方程}\; ax^2 + bx + c = 0 \;\text{的解为}\; x = \frac{-b\pm\sqrt{b^2-4ac}}{2a}",
        ),
        (
            "概率论",
            r"\text{概率论}\quad P(A \cup B) = P(A) + P(B) - P(A \cap B)",
        ),
        (
            "线性代数",
            r"\text{线性代数}\quad \begin{pmatrix} a & b \\ c & d \end{pmatrix}\text{矩阵乘法}",
        ),
        (
            "物理学",
            r"\text{物理学}\quad E_k = \frac{1}{2}mv^2\quad\text{动能公式}",
        ),
        (
            "统计学",
            r"\text{统计学}\quad \bar{x} = \frac{1}{n}\sum_{i=1}^{n} x_i\quad\text{样本均值}",
        ),
        (
            "数论基础",
            r"\text{数论基础}\quad a \equiv b \pmod{n}\quad\text{同余关系}",
        ),
        (
            "集合论",
            r"\text{集合论}\quad A \subset B \implies |A| \leq |B|",
        ),
        (
            "逻辑推理",
            r"\text{逻辑推理}\quad p \implies q \;\text{等价于}\; \neg p \lor q",
        ),
        (
            "拓扑学",
            r"\text{拓扑学}\quad \forall x \in X,\; \exists U \subset X \text{ 是开集}",
        ),
        (
            "几何学",
            r"\text{几何学}\quad \angle ABC = 180^\circ - \angle BAC - \angle BCA",
        ),
    ];
    for &(l, e) in cjk {
        v.push(("cjk", l, e));
    }

    // ── Category 5: Emoji + math (10) ──
    let emoji: &[(&str, &str)] = &[
        ("😊+math", r"\text{😊}\quad x^2 + y^2 = z^2 \quad \text{✅}"),
        ("⭐+formula", r"\text{⭐} \quad E = mc^2 \quad \text{🔥}"),
        (
            "🎉+integral",
            r"\text{🎉}\quad \int_0^1 f(x)dx \quad \text{💯}",
        ),
        ("❤️+equation", r"\text{❤️}\quad a + b = c \quad \text{👍}"),
        (
            "🚀+frac",
            r"\text{🚀}\quad \frac{a}{b} + \frac{c}{d} \quad \text{🎯}",
        ),
        (
            "💡+sum",
            r"\text{💡}\quad \sum_{i=1}^n i = \frac{n(n+1)}{2} \quad \text{📊}",
        ),
        (
            "🎵+matrix",
            r"\text{🎵}\quad \begin{pmatrix} 1 & 2 \\ 3 & 4 \end{pmatrix} \quad \text{🎶}",
        ),
        (
            "🏆+limit",
            r"\text{🏆}\quad \lim_{n\to\infty} \left(1+\frac{1}{n}\right)^n \quad \text{🥇}",
        ),
        (
            "📐+sqrt",
            r"\text{📐}\quad \sqrt{a^2 + b^2} = c \quad \text{📏}",
        ),
        (
            "🔬+chem",
            r"\text{🔬}\quad \ce{H2O + CO2 -> H2CO3} \quad \text{🧪}",
        ),
    ];
    for &(l, e) in emoji {
        v.push(("emoji", l, e));
    }
    // Remove the last one (chem inside emoji category) to keep emoji pure.
    // We'll add a replacement pure-emoji formula.
    v.pop();
    v.push((
        "emoji",
        "🎨+binom",
        r"\text{🎨}\quad \binom{n}{k} = \frac{n!}{k!(n-k)!} \quad \text{🎭}",
    ));

    // ── Category 6: Chemistry (mhchem) + misc (15) ──
    let misc: &[(&str, &str)] = &[
        ("H2O formation", r"\ce{2H2 + O2 -> 2H2O}"),
        ("CO2 + C -> 2CO", r"\ce{CO2 + C -> 2 CO}"),
        ("H2SO4 + 2NaOH", r"\ce{H2SO4 + 2NaOH -> Na2SO4 + 2H2O}"),
        ("NH3 synthesis", r"\ce{N2 + 3H2 <=> 2NH3 + \text{heat}}"),
        ("CH4 combustion", r"\ce{CH4 + 2O2 -> CO2 + 2H2O}"),
        ("Fe2O3 reduction", r"\ce{Fe2O3 + 3CO -> 2Fe + 3CO2}"),
        ("CaCO3 decomp", r"\ce{CaCO3 ->[\Delta] CaO + CO2 ^}"),
        (
            "NaCl dissolve",
            r"\ce{NaCl_{(s)} ->[\ce{H2O}] Na+_{(aq)} + Cl-_{(aq)}}",
        ),
        (
            "KMnO4 titration",
            r"\ce{5Fe^{2+} + MnO4- + 8H+ -> 5Fe^{3+} + Mn^{2+} + 4H2O}",
        ),
        ("HCl + NaOH", r"\ce{HCl + NaOH -> NaCl + H2O}"),
        ("AgCl precipitate", r"\ce{Ag+ + Cl- -> AgCl v}"),
        (
            "buffer eq",
            r"\text{pH} = \text{p}K_a + \log\frac{[\ce{A-}]}{[\ce{HA}]}",
        ),
        (
            "SO2 oxidation",
            r"\ce{2SO2 + O2 <=>[V2O5][450^\circ C] 2SO3}",
        ),
        (r"Zn + CuSO4", r"\ce{Zn + CuSO4 -> ZnSO4 + Cu v}"),
        (
            "ethanol+H2SO4",
            r"\ce{C2H5OH ->[\ce{H2SO4}][170^\circ C] C2H4 ^ + H2O}",
        ),
    ];
    for &(l, e) in misc {
        v.push(("chem", l, e));
    }

    assert_eq!(v.len(), 100, "must have exactly 100 formulas");
    v
}

#[test]
#[ignore = "run manually: cargo test -p ratex-render --test bench_render --release -- --ignored --nocapture"]
fn bench_render_100() {
    const WARMUP: u32 = 1;
    const ITERS: u32 = 3;

    let font_dir = font_dir();
    let render_opts = RenderOptions {
        font_size: 40.0,
        padding: 10.0,
        background_color: ratex_types::color::Color::WHITE,
        font_dir,
        device_pixel_ratio: 1.0,
    };

    let formulas = build_formulas();

    // ── Header ──
    println!(
        "\n╔══════════════════════════════════════════════════════════════════════════════════╗"
    );
    println!("║        RaTeX 100-Formula Render Benchmark (release, warmup={WARMUP}, iters={ITERS})        ║");
    println!(
        "╠══════════════════════════════════════════════════════════════════════════════════╣"
    );

    let mut results: Vec<BenchResult> = Vec::with_capacity(100);
    let start = Instant::now();

    for (cat, label, expr) in &formulas {
        let r = bench_formula(cat, label, expr, &render_opts, WARMUP, ITERS);
        results.push(r);
    }

    let total_wall = start.elapsed().as_millis();

    // ── Per-category summary ──
    println!("║  Per-category averages                                                                      ║");
    println!("╠══════════════════════════════╦══════╦══════════╦════════╦══════════════╦════════╦════════╣");
    println!(
        "║ {:<28} ║ {:>4} ║ {:>8} ║ {:>6} ║ {:>12} ║ {:>6} ║ {:>6} ║",
        "Category", "Cnt", "P+L(μs)", "PNG(μs)", "SVG-sa(μs)", "SVG(μs)", "PDF(μs)"
    );
    println!("╠══════════════════════════════╬══════╬══════════╬════════╬══════════════╬════════╬════════╣");

    for cat in &["math", "complex", "matrix", "cjk", "emoji", "chem"] {
        let group: Vec<_> = results.iter().filter(|r| r.category == *cat).collect();
        if group.is_empty() {
            continue;
        }
        let n = group.len() as u128;
        let pl = group.iter().map(|r| r.parse_layout_us).sum::<u128>() / n;
        let png = group.iter().map(|r| r.png_us).sum::<u128>() / n;
        let svg = group.iter().map(|r| r.svg_us).sum::<u128>() / n;
        let svg_sa = group.iter().map(|r| r.svg_standalone_us).sum::<u128>() / n;
        let pdf = group.iter().map(|r| r.pdf_us).sum::<u128>() / n;
        let _total_glyphs: usize = group.iter().map(|r| r.glyph_count).sum();
        println!(
            "║ {:<28} ║ {:>4} ║ {:>7} ║ {:>5} ║ {:>10} ║ {:>5} ║ {:>5} ║",
            cat,
            group.len(),
            pl,
            png,
            svg_sa,
            svg,
            pdf
        );
    }

    // ── Overall summary ──
    let n = results.len() as u128;
    let avg_pl = results.iter().map(|r| r.parse_layout_us).sum::<u128>() / n;
    let avg_png = results.iter().map(|r| r.png_us).sum::<u128>() / n;
    let avg_svg = results.iter().map(|r| r.svg_us).sum::<u128>() / n;
    let avg_svg_sa = results.iter().map(|r| r.svg_standalone_us).sum::<u128>() / n;
    let avg_pdf = results.iter().map(|r| r.pdf_us).sum::<u128>() / n;
    let total_glyphs: usize = results.iter().map(|r| r.glyph_count).sum();

    println!("╠══════════════════════════════╩══════╩══════════╩════════╩══════════════╩════════╩════════╣");
    println!("║  OVERALL ({n} formulas, {total_glyphs} glyphs, wall time {total_wall}ms)                                          ║");
    println!("╠══════════════════════════════════════════════════════════════════════════════════════════════╣");
    println!("║  Parse+Layout avg:        {:>5} μs                                                           ║", avg_pl);
    println!("║  PNG avg:                 {:>5} μs                                                           ║", avg_png);
    println!("║  SVG avg (text):          {:>5} μs                                                           ║", avg_svg);
    println!("║  SVG standalone avg:      {:>5} μs                                                           ║", avg_svg_sa);
    println!("║  PDF avg:                 {:>5} μs                                                           ║", avg_pdf);
    println!("║  End-to-end (PL+PNG):     {:>5} μs                                                           ║", avg_pl + avg_png);
    println!("╠══════════════════════════════════════════════════════════════════════════════════════════════╣");
    println!("║  Throughput (end-to-end PNG): {:.1} formulas/sec                                                    ║",
             1_000_000.0 / (avg_pl + avg_png) as f64);
    println!("╚══════════════════════════════════════════════════════════════════════════════════════════════╝\n");

    // Sanity check: all PNG outputs non-empty
    for (cat, label, expr) in &formulas {
        let ast = parse(expr).expect("parse");
        let l = layout(&ast, &LayoutOptions::default());
        let dl = to_display_list(&l);
        let png = render_to_png(&dl, &render_opts).expect("png");
        assert!(!png.is_empty(), "{cat}/{label}: PNG output is empty");
    }
}
