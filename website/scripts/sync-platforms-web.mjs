/**
 * Copy repo `platforms/web` WASM bindgen (`pkg/`), KaTeX `fonts/`, and `fonts.css`
 * into `public/platforms/web/` so `astro build` emits them as static files.
 * Dev still uses vite-plugin-platforms-web (repo mount); production / Cloudflare Pages needs this.
 */
import { cpSync, existsSync, mkdirSync, rmSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const WEBSITE_ROOT = path.join(__dirname, "..");
const SRC = path.join(WEBSITE_ROOT, "..", "platforms", "web");
const DEST = path.join(WEBSITE_ROOT, "public", "platforms", "web");
const PKG = path.join(SRC, "pkg");

function main() {
  if (!existsSync(SRC)) {
    console.error(
      `[ratex-website] Missing ${SRC}. Clone the full repo (not website-only) or set up platforms/web.`
    );
    process.exit(1);
  }
  if (!existsSync(PKG)) {
    console.error(
      `[ratex-website] Missing WASM pkg at ${PKG}. Run:\n  cd platforms/web && bash build.sh`
    );
    process.exit(1);
  }
  let st;
  try {
    st = statSync(PKG);
  } catch {
    console.error(`[ratex-website] Cannot read ${PKG}`);
    process.exit(1);
  }
  if (!st.isDirectory()) {
    console.error(`[ratex-website] Not a directory: ${PKG}`);
    process.exit(1);
  }

  if (existsSync(DEST)) {
    rmSync(DEST, { recursive: true });
  }
  mkdirSync(DEST, { recursive: true });

  cpSync(PKG, path.join(DEST, "pkg"), { recursive: true });
  cpSync(path.join(SRC, "fonts"), path.join(DEST, "fonts"), { recursive: true });
  cpSync(path.join(SRC, "fonts.css"), path.join(DEST, "fonts.css"));

  console.log(`[ratex-website] synced platforms/web → ${DEST}`);
}

main();
