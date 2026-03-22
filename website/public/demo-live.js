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

const EXAMPLES = [
  { label: "Quadratic formula",     latex: "x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}" },
  { label: "Euler's identity",      latex: "e^{i\\pi} + 1 = 0" },
  { label: "Pythagorean theorem",   latex: "a^2 + b^2 = c^2" },
  { label: "Basel problem",         latex: "\\sum_{n=1}^{\\infty} \\frac{1}{n^2} = \\frac{\\pi^2}{6}" },
  { label: "Gaussian integral",     latex: "\\int_{-\\infty}^{\\infty} e^{-x^2}\\,dx = \\sqrt{\\pi}" },
  { label: "Binomial theorem",      latex: "(x+y)^n = \\sum_{k=0}^{n} \\binom{n}{k} x^k y^{n-k}" },
  { label: "Taylor series",         latex: "f(x) = \\sum_{n=0}^{\\infty} \\frac{f^{(n)}(a)}{n!}(x-a)^n" },
  { label: "Schrödinger equation",  latex: "i\\hbar\\frac{\\partial}{\\partial t}\\Psi = \\hat{H}\\Psi" },
  { label: "Maxwell–Faraday",       latex: "\\nabla \\times \\mathbf{E} = -\\frac{\\partial \\mathbf{B}}{\\partial t}" },
  { label: "Normal distribution",   latex: "f(x) = \\frac{1}{\\sigma\\sqrt{2\\pi}}e^{-\\frac{(x-\\mu)^2}{2\\sigma^2}}" },
  { label: "Fourier transform",     latex: "\\hat{f}(\\xi) = \\int_{-\\infty}^{\\infty} f(x)\\,e^{-2\\pi i x\\xi}\\,dx" },
  { label: "Cauchy integral",       latex: "f(a) = \\frac{1}{2\\pi i}\\oint_\\gamma \\frac{f(z)}{z-a}\\,dz" },
  { label: "Matrix",                latex: "\\begin{pmatrix} a & b \\\\ c & d \\end{pmatrix}" },
  { label: "Determinant",           latex: "\\det(A) = \\sum_{\\sigma \\in S_n} \\text{sgn}(\\sigma) \\prod_{i=1}^n a_{i,\\sigma(i)}" },
  { label: "Stokes' theorem",       latex: "\\oint_{\\partial \\Sigma} \\mathbf{F}\\cdot d\\mathbf{r} = \\iint_{\\Sigma}(\\nabla\\times\\mathbf{F})\\cdot d\\mathbf{S}" },
  { label: "Einstein field eq.",    latex: "G_{\\mu\\nu} + \\Lambda g_{\\mu\\nu} = \\frac{8\\pi G}{c^4} T_{\\mu\\nu}" },
];

// ── font id → CSS font ──
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

// ── draw RaTeX display list ──
// em=29 matches KaTeX container: text-2xl (24px) × KaTeX 1.21em ≈ 29px
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

// ── state ──
let renderLatex = null;
let wasmReady = false, fontsReady = false, katexReady = false;

function setStatus(state, text) {
  const dot = document.getElementById('status-dot');
  const cls = {
    ready: 'inline-block h-2 w-2 shrink-0 rounded-full bg-emerald-500',
    loading: 'inline-block h-2 w-2 shrink-0 rounded-full bg-amber-500',
    error: 'inline-block h-2 w-2 shrink-0 rounded-full bg-red-500',
  };
  dot.className = cls[state] || cls.loading;
  dot.setAttribute('aria-hidden', 'true');
  document.getElementById('status-text').textContent = text;
}

function tryEnableButton() {
  if (wasmReady && fontsReady && katexReady) {
    document.getElementById('render-btn').disabled = false;
    setStatus('ready', 'Ready');
    run();
  }
}

// ── render ──
function run() {
  if (!wasmReady || !fontsReady || !katexReady) return;
  const latex = document.getElementById('formula').value.trim();
  if (!latex) return;

  // KaTeX
  const katexEl = document.getElementById('katex-output');
  try {
    const div = document.createElement('div');
    div.className = 'text-2xl leading-[1.2] text-zinc-900';
    div.innerHTML = katex.renderToString(latex, {
      throwOnError: false, displayMode: false, trust: true, strict: false
    });
    katexEl.innerHTML = '';
    katexEl.appendChild(div);
  } catch(e) {
    katexEl.innerHTML = `<span class="text-xs text-red-600 font-mono break-words">${String(e).replace(/</g,'&lt;')}</span>`;
  }

  // RaTeX — em=20 to match KaTeX visual size
  const ratexEl = document.getElementById('ratex-output');
  try {
    const dl = JSON.parse(renderLatex(latex));
    const canvas = document.createElement('canvas');
    canvas.className = 'max-w-full h-auto';
    drawDisplayList(dl, canvas, 29, 4);
    ratexEl.innerHTML = '';
    ratexEl.appendChild(canvas);
  } catch(e) {
    ratexEl.innerHTML = `<span class="text-xs text-red-600 font-mono break-words">${String(e).replace(/^.*?Error:\s*/, '').slice(0, 200).replace(/</g,'&lt;')}</span>`;
  }
}

// ── build example cards ──
function buildExamples() {
  const grid = document.getElementById('examples-grid');
  for (const ex of EXAMPLES) {
    const card = document.createElement('div');
    card.setAttribute('role', 'listitem');
    card.className =
      'p-4 cursor-pointer border border-outline/50 bg-white rounded-[12px] ring-1 ring-black/[0.04] hover:border-primary/40 hover:ring-primary/10 hover:bg-surface-container-lowest/80 transition-colors';

    const label = document.createElement('div');
    label.className = 'text-xs font-label uppercase tracking-wide text-primary/90 mb-2';
    label.textContent = ex.label;

    const preview = document.createElement('div');
    preview.className =
      'flex min-h-[2.25rem] items-center justify-center overflow-x-auto text-sm text-zinc-900';
    try {
      preview.innerHTML = katex.renderToString(ex.latex, {
        throwOnError: false, displayMode: false, trust: true, strict: false
      });
    } catch(e) {
      preview.textContent = ex.latex;
    }

    card.appendChild(label);
    card.appendChild(preview);
    card.addEventListener('click', () => {
      document.getElementById('formula').value = ex.latex;
      run();
      document.getElementById('formula').scrollIntoView({ behavior: 'smooth', block: 'nearest' });
    });
    grid.appendChild(card);
  }
}

// ── boot ──
document.getElementById('katex-script').addEventListener('load', () => {
  katexReady = true;
  buildExamples();
  tryEnableButton();
  loadFontsAndWasm();
});

async function loadFontsAndWasm() {
  setStatus('loading', 'Loading fonts…');
  try {
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

  setStatus('loading', 'Loading WASM…');
  try {
    const mod = await import(ratexWasmModuleUrl());
    await mod.default();
    renderLatex = mod.renderLatex;
    wasmReady = true;
    tryEnableButton();
  } catch(e) {
    setStatus('error', 'WASM load failed: ' + e);
  }
}

document.getElementById('formula').addEventListener('keydown', e => { if (e.key === 'Enter') run(); });
