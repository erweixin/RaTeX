/**
 * 1) Optionally compile WASM (platforms/web/pkg 不入库，需在打包前 wasm-pack)。
 *    设置 SKIP_WEB_WASM_BUILD=1 可跳过（例如已手动跑过 build.sh）。
 * 2) 同步 pkg + 字体到 public/，供 astro build 产出静态文件。
 *
 * Cloudflare Pages 默认镜像无 wasm-pack：当 CF_PAGES=1（CF 会自动注入）或
 * AUTO_INSTALL_WASM_TOOLCHAIN=1 时，自动安装 rustc（若尚无）与 wasm-pack。
 */
import { spawnSync } from "node:child_process";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const WEBSITE_ROOT = path.join(__dirname, "..");
const WASM_BUILD_SH = path.join(WEBSITE_ROOT, "..", "platforms", "web", "build.sh");
const SYNC_SCRIPT = path.join(__dirname, "sync-platforms-web.mjs");

function augmentPathWithCargoBin(env) {
  const bin = path.join(os.homedir(), ".cargo", "bin");
  const cur = env.PATH ?? "";
  const parts = cur.split(path.delimiter).filter(Boolean);
  if (parts.includes(bin)) return env;
  return { ...env, PATH: [bin, ...parts].join(path.delimiter) };
}

function wasmPackWorks(env) {
  const r = spawnSync("wasm-pack", ["--version"], { env, stdio: "pipe" });
  return r.status === 0;
}

function ensureWasmToolchain(env) {
  env = augmentPathWithCargoBin(env);
  if (wasmPackWorks(env)) return env;

  const auto =
    process.env.CF_PAGES === "1" || process.env.AUTO_INSTALL_WASM_TOOLCHAIN === "1";
  if (!auto) {
    console.error(
      `[ratex-website] 未找到 wasm-pack。
本地安装：https://rustwasm.github.io/wasm-pack/installer/
Cloudflare Pages 会在 CF_PAGES=1 时尝试自动安装；其它 CI 请设 AUTO_INSTALL_WASM_TOOLCHAIN=1`
    );
    process.exit(1);
  }

  console.warn("[ratex-website] 正在安装 wasm 工具链（Pages/CI）…");

  const installScript = `
set -euo pipefail
export PATH="\${HOME}/.cargo/bin:\${PATH}"
if command -v rustc >/dev/null 2>&1; then
  rustup target add wasm32-unknown-unknown --toolchain stable
else
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal
  export PATH="\${HOME}/.cargo/bin:\${PATH}"
  rustup target add wasm32-unknown-unknown --toolchain stable
fi
if ! command -v wasm-pack >/dev/null 2>&1; then
  curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
fi
wasm-pack --version
`;

  const r = spawnSync("bash", ["-lc", installScript], {
    stdio: "inherit",
    env: process.env,
  });
  if (r.status !== 0) process.exit(r.status ?? 1);

  env = augmentPathWithCargoBin({ ...process.env });
  if (!wasmPackWorks(env)) {
    console.error(
      `[ratex-website] wasm-pack 安装完成但仍无法执行（PATH 不包含 ${path.join(os.homedir(), ".cargo", "bin")}）。`
    );
    process.exit(1);
  }
  return env;
}

let env = augmentPathWithCargoBin({ ...process.env });

if (process.env.SKIP_WEB_WASM_BUILD !== "1") {
  env = ensureWasmToolchain(env);
  const wasm = spawnSync("bash", [WASM_BUILD_SH], { stdio: "inherit", cwd: WEBSITE_ROOT, env });
  if (wasm.error) {
    console.error(wasm.error);
    process.exit(1);
  }
  if (wasm.status !== 0) process.exit(wasm.status ?? 1);
}

const sync = spawnSync(process.execPath, [SYNC_SCRIPT], { stdio: "inherit", cwd: WEBSITE_ROOT, env });
if (sync.status !== 0) process.exit(sync.status ?? 1);
