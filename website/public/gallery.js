/**
 * RaTeX WASM gallery: lazy-render formulas with IntersectionObserver.
 * WASM entry matches repo layout: platforms/web/pkg/ratex_wasm.js
 * → under site base: /RaTeX/platforms/web/pkg/ratex_wasm.js (GitHub Pages, astro dev, preview).
 *
 * Optional override (tests / custom hosting): set before loading this script
 *   window.__RATES_WASM_IMPORT_URL__ = "https://example.com/pkg/ratex_wasm.js";
 * Local dev: `astro dev` / `astro preview` mount repo platforms/web at /RaTeX/platforms/web/
 * (vite-plugin-platforms-web.mjs) after `cd platforms/web && bash build.sh`.
 *
 * Note: pathname like /RaTeX (no trailing slash) makes new URL("platforms/...", base) drop the
 * site base; we normalize to a directory URL first.
 */
(function (global) {
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

  /** Same path segment as on disk: platforms/web/pkg/ratex_wasm.js */
  function wasmEntryUrl() {
    var g = typeof globalThis !== "undefined" ? globalThis : global;
    if (typeof g.__RATES_WASM_IMPORT_URL__ === "string" && g.__RATES_WASM_IMPORT_URL__.length > 0) {
      return g.__RATES_WASM_IMPORT_URL__;
    }
    var base = typeof g.__RATEX_SITE_BASE__ === "string" ? g.__RATEX_SITE_BASE__ : "";
    if (base) {
      if (!base.endsWith("/")) base += "/";
      return new URL("platforms/web/pkg/ratex_wasm.js", new URL(base, location.origin)).href;
    }
    var pageDir = getSiteDirUrl();
    var rel =
      location.pathname.indexOf("/website/") !== -1
        ? "../platforms/web/pkg/ratex_wasm.js"
        : "platforms/web/pkg/ratex_wasm.js";
    return new URL(rel, pageDir).href;
  }

  const EM = 18;
  const PAD = 4;

  function fontIdToCss(fontId, sizePx) {
    switch (fontId) {
      case "AMS-Regular":
        return `${sizePx}px KaTeX_AMS`;
      case "Caligraphic-Regular":
        return `${sizePx}px KaTeX_Caligraphic`;
      case "Fraktur-Regular":
        return `${sizePx}px KaTeX_Fraktur`;
      case "Main-Bold":
        return `bold ${sizePx}px KaTeX_Main`;
      case "Main-BoldItalic":
        return `italic bold ${sizePx}px KaTeX_Main`;
      case "Main-Italic":
        return `italic ${sizePx}px KaTeX_Main`;
      case "Main-Regular":
        return `${sizePx}px KaTeX_Main`;
      case "Math-BoldItalic":
        return `italic bold ${sizePx}px KaTeX_Math`;
      case "Math-Italic":
        return `italic ${sizePx}px KaTeX_Math`;
      case "SansSerif-Bold":
        return `bold ${sizePx}px KaTeX_SansSerif`;
      case "SansSerif-Italic":
        return `italic ${sizePx}px KaTeX_SansSerif`;
      case "SansSerif-Regular":
        return `${sizePx}px KaTeX_SansSerif`;
      case "Script-Regular":
        return `${sizePx}px KaTeX_Script`;
      case "Size1-Regular":
        return `${sizePx}px KaTeX_Size1`;
      case "Size2-Regular":
        return `${sizePx}px KaTeX_Size2`;
      case "Size3-Regular":
        return `${sizePx}px KaTeX_Size3`;
      case "Size4-Regular":
        return `${sizePx}px KaTeX_Size4`;
      case "Typewriter-Regular":
        return `${sizePx}px KaTeX_Typewriter`;
      default:
        return `${sizePx}px KaTeX_Main`;
    }
  }

  function drawDisplayList(dl, canvas, em, pad) {
    const dpr = window.devicePixelRatio || 1;
    const totalH = dl.height + dl.depth;
    const cssW = Math.max(1, Math.ceil(dl.width * em + 2 * pad));
    const cssH = Math.max(1, Math.ceil(totalH * em + 2 * pad));
    canvas.width = cssW * dpr;
    canvas.height = cssH * dpr;
    canvas.style.width = cssW + "px";
    canvas.style.height = cssH + "px";
    const ctx = canvas.getContext("2d");
    ctx.scale(dpr, dpr);
    ctx.clearRect(0, 0, cssW, cssH);
    ctx.save();
    ctx.translate(pad, pad);
    for (const item of dl.items) {
      const c = item.color;
      const rgb = `rgb(${(c.r * 255) | 0},${(c.g * 255) | 0},${(c.b * 255) | 0})`;
      if (item.type === "Line") {
        ctx.fillStyle = rgb;
        ctx.fillRect(
          item.x * em,
          item.y * em - (item.thickness * em) / 2,
          item.width * em,
          Math.max(0.5, item.thickness * em)
        );
      } else if (item.type === "Rect") {
        ctx.fillStyle = rgb;
        ctx.fillRect(item.x * em, item.y * em, item.width * em, item.height * em);
      } else if (item.type === "Path") {
        const ox = item.x * em;
        const oy = item.y * em;
        ctx.beginPath();
        for (const cmd of item.commands) {
          if (cmd.type === "MoveTo") ctx.moveTo(ox + cmd.x * em, oy + cmd.y * em);
          else if (cmd.type === "LineTo") ctx.lineTo(ox + cmd.x * em, oy + cmd.y * em);
          else if (cmd.type === "CubicTo")
            ctx.bezierCurveTo(
              ox + cmd.x1 * em,
              oy + cmd.y1 * em,
              ox + cmd.x2 * em,
              oy + cmd.y2 * em,
              ox + cmd.x * em,
              oy + cmd.y * em
            );
          else if (cmd.type === "QuadTo")
            ctx.quadraticCurveTo(ox + cmd.x1 * em, oy + cmd.y1 * em, ox + cmd.x * em, oy + cmd.y * em);
          else if (cmd.type === "Close") ctx.closePath();
        }
        ctx.fillStyle = rgb;
        if (item.fill) ctx.fill();
        else ctx.stroke();
      } else if (item.type === "GlyphPath") {
        const sz = (item.scale || 1) * em;
        ctx.save();
        ctx.translate(item.x * em, item.y * em);
        ctx.font = fontIdToCss(item.font, sz);
        ctx.textBaseline = "alphabetic";
        ctx.textAlign = "left";
        ctx.fillStyle = rgb;
        ctx.fillText(String.fromCodePoint(item.char_code), 0, 0);
        ctx.restore();
      }
    }
    ctx.restore();
  }

  async function loadFonts() {
    await Promise.all([
      document.fonts.load(`${EM}px KaTeX_Main`),
      document.fonts.load(`italic ${EM}px KaTeX_Main`),
      document.fonts.load(`bold ${EM}px KaTeX_Main`),
      document.fonts.load(`italic bold ${EM}px KaTeX_Main`),
      document.fonts.load(`italic ${EM}px KaTeX_Math`),
      document.fonts.load(`italic bold ${EM}px KaTeX_Math`),
      document.fonts.load(`${EM}px KaTeX_AMS`),
      document.fonts.load(`${EM}px KaTeX_Caligraphic`),
      document.fonts.load(`${EM}px KaTeX_Fraktur`),
      document.fonts.load(`${EM}px KaTeX_SansSerif`),
      document.fonts.load(`italic ${EM}px KaTeX_SansSerif`),
      document.fonts.load(`bold ${EM}px KaTeX_SansSerif`),
      document.fonts.load(`${EM}px KaTeX_Script`),
      document.fonts.load(`${EM}px KaTeX_Typewriter`),
      document.fonts.load(`${EM}px KaTeX_Size1`),
      document.fonts.load(`${EM}px KaTeX_Size2`),
      document.fonts.load(`${EM}px KaTeX_Size3`),
      document.fonts.load(`${EM}px KaTeX_Size4`),
    ]).catch(() => {});
  }

  async function loadWasm() {
    const mod = await import(wasmEntryUrl());
    await mod.default();
    return mod.renderLatex;
  }

  /** Cached promise for home hero playground (single WASM load). */
  var playgroundEnginePromise = null;
  function ensurePlaygroundEngine() {
    if (!playgroundEnginePromise) {
      playgroundEnginePromise = (async function () {
        await loadFonts();
        try {
          const renderLatex = await loadWasm();
          return { renderLatex, drawDisplayList, EM, PAD };
        } catch (e) {
          return { renderLatex: null, wasmError: e };
        }
      })();
    }
    return playgroundEnginePromise;
  }

  function escapeHtml(s) {
    return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
  }

  /**
   * @param {object} opts
   * @param {string} opts.dataUrl - e.g. "data/math.json"
   * @param {string} [opts.statusEl] - element id for status text
   * @param {string} [opts.countEl] - element id for rendered count
   * @param {string} [opts.listEl] - element id for container (default gallery-list)
   */
  async function init(opts) {
    if (document.readyState === "loading") {
      await new Promise((resolve) =>
        document.addEventListener("DOMContentLoaded", resolve, { once: true })
      );
    }

    const statusEl = opts.statusEl ? document.getElementById(opts.statusEl) : null;
    const countEl = opts.countEl ? document.getElementById(opts.countEl) : null;
    const listEl = document.getElementById(opts.listEl || "gallery-list");

    function setStatus(t) {
      if (statusEl) statusEl.textContent = t;
    }

    if (!listEl) {
      console.error("RaTeXGallery: list container not found (#" + (opts.listEl || "gallery-list") + ")");
      return;
    }

    const dataUrlRaw = opts.dataUrl || "data/math.json";
    const dataUrl = new URL(dataUrlRaw, location.href).href;

    setStatus("Loading fonts…");
    await loadFonts();

    setStatus("Loading formula list…");
    let payload;
    try {
      const res = await fetch(dataUrl, { cache: "no-store" });
      if (!res.ok) throw new Error(res.status + " " + res.statusText);
      payload = await res.json();
    } catch (e) {
      setStatus("Failed to load " + dataUrl + ": " + e);
      return;
    }

    const formulas = payload.formulas || [];
    const sections = payload.sections;
    let total = formulas.length;
    if (total === 0 && sections && sections.length > 0) {
      total = sections.reduce(function (acc, sec) {
        return acc + (sec.formulas && sec.formulas.length ? sec.formulas.length : 0);
      }, 0);
    }

    setStatus("Loading WASM…");
    let renderLatex = null;
    try {
      renderLatex = await loadWasm();
    } catch (e) {
      setStatus("WASM unavailable — showing LaTeX source only. (" + e + ")");
    }
    let rendered = 0;

    setStatus(renderLatex ? "Rendering on scroll…" : "Source list ready (WASM missing).");

    /**
     * @param {HTMLElement} parent
     * @param {string} latex
     * @param {number} index
     */
    const gridClass =
      "gallery-grid grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4 gap-2 sm:gap-2";

    function appendGalleryRow(parent, latex, index) {
      const row = document.createElement("article");
      row.className =
        "gallery-row border border-outline/40 rounded-md bg-white/60 p-2 flex flex-col gap-1.5 min-w-0";
      row.dataset.index = String(index);

      const src = document.createElement("div");
      src.className = "min-w-0";
      const pre = document.createElement("pre");
      pre.className =
        "text-[10px] leading-snug font-mono text-on-surface-variant whitespace-pre-wrap break-all bg-surface-container/90 p-2 border border-outline/30 rounded max-h-28 overflow-y-auto";
      pre.textContent = latex;
      src.appendChild(pre);

      const outWrap = document.createElement("div");
      outWrap.className =
        "flex min-h-[1.75rem] items-center justify-center overflow-x-auto overflow-y-hidden";
      const placeholder = document.createElement("div");
      placeholder.className = "text-xs text-on-surface-variant/50 italic";
      placeholder.textContent = "…";
      outWrap.appendChild(placeholder);

      row.appendChild(src);
      row.appendChild(outWrap);
      parent.appendChild(row);

      const renderOne = () => {
        if (row.dataset.done === "1") return;
        row.dataset.done = "1";
        placeholder.remove();
        if (!renderLatex) {
          const err = document.createElement("div");
          err.className = "text-xs text-red-700 font-mono break-all max-w-full";
          err.textContent =
            "WASM not loaded. On GitHub Pages this should work; locally run: cd platforms/web && bash build.sh — then ensure platforms/web/pkg is deployed next to the site (see CI).";
          outWrap.appendChild(err);
          return;
        }
        try {
          const dl = JSON.parse(renderLatex(latex));
          const canvas = document.createElement("canvas");
          drawDisplayList(dl, canvas, EM, PAD);
          outWrap.appendChild(canvas);
        } catch (e) {
          const err = document.createElement("div");
          err.className = "text-xs text-red-700 font-mono break-all max-w-full";
          const msg = String(e).replace(/^.*?:\s*/, "").slice(0, 400);
          err.textContent = msg;
          outWrap.appendChild(err);
        }
        rendered += 1;
        if (countEl) countEl.textContent = `${rendered} / ${total}`;
      };

      row._renderOne = renderOne;
    }

    if (sections && sections.length > 0) {
      const toc = document.createElement("nav");
      toc.className = "mb-8 p-4 bg-surface-container/60 border border-outline/40 rounded-lg";
      toc.setAttribute("aria-label", "按 KaTeX 文档分类的目录");
      const tocTitle = document.createElement("p");
      tocTitle.className = "text-xs font-label uppercase tracking-wider text-on-surface-variant mb-2";
      tocTitle.textContent = "分类目录（对齐 katex.org/docs/supported.html）";
      const ul = document.createElement("ul");
      ul.className = "flex flex-wrap gap-x-4 gap-y-2 text-sm";
      for (const sec of sections) {
        const li = document.createElement("li");
        const a = document.createElement("a");
        a.href = "#" + sec.id;
        a.className = "text-primary hover:underline";
        a.textContent = sec.title + " (" + sec.formulas.length + ")";
        li.appendChild(a);
        ul.appendChild(li);
      }
      toc.appendChild(tocTitle);
      toc.appendChild(ul);
      if (listEl.parentElement) listEl.parentElement.insertBefore(toc, listEl);

      let globalIdx = 0;
      for (const sec of sections) {
        const section = document.createElement("section");
        section.id = sec.id;
        section.className = "mb-8 scroll-mt-28";
        const h2 = document.createElement("h2");
        h2.className =
          "text-xl sm:text-2xl font-semibold font-serif text-zinc-800 mb-0.5 border-b border-outline/40 pb-1.5";
        h2.textContent = sec.title;
        const sub = document.createElement("p");
        sub.className = "text-[11px] text-on-surface-variant mb-2";
        sub.textContent = sec.titleEn + " · " + sec.formulas.length + " 条";
        const grid = document.createElement("div");
        grid.className = gridClass;
        section.appendChild(h2);
        section.appendChild(sub);
        section.appendChild(grid);
        for (const latex of sec.formulas) {
          appendGalleryRow(grid, latex, globalIdx++);
        }
        listEl.appendChild(section);
      }
    } else {
      const grid = document.createElement("div");
      grid.className = gridClass;
      listEl.appendChild(grid);
      formulas.forEach((latex, index) => {
        appendGalleryRow(grid, latex, index);
      });
    }

    const io = new IntersectionObserver(
      (entries) => {
        for (const ent of entries) {
          if (ent.isIntersecting) {
            const row = ent.target;
            if (row._renderOne) row._renderOne();
            io.unobserve(row);
          }
        }
      },
      { root: null, rootMargin: "240px 0px", threshold: 0 }
    );

    listEl.querySelectorAll(".gallery-row").forEach((row) => io.observe(row));

    if (countEl) countEl.textContent = `0 / ${total}`;
    setStatus(
      renderLatex ? "Ready — formulas render as you scroll." : "WASM missing — only source lines are listed; build platforms/web to enable Canvas output."
    );
  }

  global.RaTeXGallery = { init, ensurePlaygroundEngine };
})(typeof window !== "undefined" ? window : globalThis);
