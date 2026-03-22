/**
 * Dev / preview: serve ../../platforms/web from the repo at {base}/platforms/web/
 * so RaTeX WASM loads without copying pkg into public/ (same layout as GitHub Pages _site).
 */
import { createReadStream, existsSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = fileURLToPath(new URL(".", import.meta.url));
const PLATFORMS_WEB = path.join(__dirname, "..", "platforms", "web");

const MIME = {
  ".wasm": "application/wasm",
  ".js": "application/javascript",
  ".mjs": "application/javascript",
  ".json": "application/json",
  ".css": "text/css",
  ".woff2": "font/woff2",
  ".woff": "font/woff",
  ".ttf": "font/ttf",
};

function mimeFor(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  return MIME[ext] || "application/octet-stream";
}

function mount(middlewares, prefixes) {
  if (!existsSync(PLATFORMS_WEB)) {
    console.warn(
      `[ratex-wasm] Skip dev mount: no ${PLATFORMS_WEB} (run: cd platforms/web && bash build.sh)`
    );
    return;
  }

  middlewares.use((req, res, next) => {
    const raw = req.url || "";
    let prefix = null;
    for (const p of prefixes) {
      if (raw.startsWith(p)) {
        prefix = p;
        break;
      }
    }
    if (!prefix) return next();
    if (req.method !== "GET" && req.method !== "HEAD") return next();

    const rel = decodeURIComponent(raw.slice(prefix.length).split("?")[0]);
    if (!rel) return next();

    const resolved = path.resolve(PLATFORMS_WEB, rel);
    const root = path.resolve(PLATFORMS_WEB);
    if (!resolved.startsWith(root + path.sep) && resolved !== root) {
      return next();
    }
    const filePath = resolved;

    let st;
    try {
      st = statSync(filePath);
    } catch {
      return next();
    }
    if (!st.isFile()) return next();

    const mime = mimeFor(filePath);
    res.setHeader("Content-Type", mime);
    res.setHeader("Content-Length", String(st.size));

    if (req.method === "HEAD") {
      res.statusCode = 200;
      res.end();
      return;
    }

    createReadStream(filePath).on("error", () => next()).pipe(res);
  });
}

/**
 * @param {string} siteBase Astro `base`, e.g. "/RaTeX/"
 */
export function vitePluginPlatformsWeb(siteBase) {
  const normalized = siteBase.replace(/\/?$/, "");
  /** Primary: /RaTeX/platforms/web/… Fallback: /platforms/web/… when URL resolution drops site base. */
  const prefixes = [`${normalized}/platforms/web/`, "/platforms/web/"];

  return {
    name: "vite-plugin-ratex-platforms-web",
    configureServer(server) {
      mount(server.middlewares, prefixes);
    },
    configurePreviewServer(server) {
      mount(server.middlewares, prefixes);
    },
  };
}
