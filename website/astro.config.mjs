import { defineConfig } from "astro/config";
import tailwind from "@astrojs/tailwind";
import { vitePluginPlatformsWeb } from "./vite-plugin-platforms-web.mjs";

const base = "/RaTeX/";

// GitHub Pages project site: https://erweixin.github.io/RaTeX/
export default defineConfig({
  site: "https://erweixin.github.io",
  base,
  integrations: [tailwind()],
  build: {
    format: "file",
  },
  vite: {
    plugins: [vitePluginPlatformsWeb(base)],
  },
});
