/**
 * Fails CI / deploy preview when Astro output is missing WASM (often means `astro build`
 * ran without syncing public/platforms/web → Cloudflare returns index.html → "Failed to fetch module").
 *
 * Override: SKIP_VERIFY_DIST_WASM=1
 */
import { existsSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const ROOT = path.join(__dirname, "..");
const PKG = path.join(ROOT, "dist", "platforms", "web", "pkg");
const WASM = path.join(PKG, "ratex_wasm_bg.wasm");
const SHIM = path.join(PKG, "ratex_wasm.js");

if (process.env.SKIP_VERIFY_DIST_WASM === "1") {
  process.exit(0);
}

function ok(p) {
  try {
    return existsSync(p) && statSync(p).isFile() && statSync(p).size > 0;
  } catch {
    return false;
  }
}

if (ok(WASM) && ok(SHIM)) {
  console.log("[ratex-website] dist WASM present:", WASM);
  process.exit(0);
}

console.error(
  [
    "[ratex-website] dist 缺少 WASM（Cloudflare 会上传 dist，缺失时 /platforms/web/pkg/*.js 会回退成 HTML，浏览器报 Failed to fetch dynamically imported module）。",
    "  预期文件: " + WASM,
    "  请用仓库根目录完整 clone，且构建命令为: npm run build（或确保 astro:build:start 会跑 scripts/prebuild-platforms-web.mjs）。",
    "  Cloudflare Pages 需安装 Rust wasm32-unknown-unknown + wasm-pack，且不要用仅执行 `astro build` 而跳过预同步 的方式。",
  ].join("\n")
);
process.exit(1);
