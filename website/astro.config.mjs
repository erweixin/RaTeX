import { execFileSync } from "node:child_process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { defineConfig } from "astro/config";
import tailwind from "@astrojs/tailwind";
import { vitePluginPlatformsWeb } from "./vite-plugin-platforms-web.mjs";

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const base = "/RaTeX/";

// GitHub Pages project site: https://erweixin.github.io/RaTeX/
export default defineConfig({
  site: "https://erweixin.github.io",
  base,
  integrations: [
    tailwind(),
    {
      name: "ratex-prebuild-platforms-web",
      hooks: {
        /**
         * Cloudflare / CI 常把 Build command 写成 `astro build`，不会触发 npm `prebuild`。
         * 在此阶段跑 wasm-pack + 同步到 public/，保证 dist 含 /platforms/web/*。
         */
        "astro:build:start": () => {
          execFileSync(process.execPath, [path.join(__dirname, "scripts", "prebuild-platforms-web.mjs")], {
            cwd: __dirname,
            stdio: "inherit",
            env: process.env,
          });
        },
      },
    },
  ],
  build: {
    format: "file",
  },
  vite: {
    plugins: [vitePluginPlatformsWeb(base)],
  },
});
