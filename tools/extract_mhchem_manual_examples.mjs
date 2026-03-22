#!/usr/bin/env node
/**
 * Extract ce/pu examples from the official manual (MathJax-mhchem gh-pages),
 * same rules as index.html: :::: -> \ce{}, ::::pu -> \pu{}, ::::$ -> $...$
 *
 * Regenerates: tests/golden/test_case_ce.txt
 *   node tools/extract_mhchem_manual_examples.mjs
 *   node tools/extract_mhchem_manual_examples.mjs path/to/index.html
 */
import fs from "fs";
import https from "https";
import path from "path";
import { fileURLToPath } from "url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, "..");
const outPath = path.join(root, "tests/golden/test_case_ce.txt");
const defaultUrl =
  "https://raw.githubusercontent.com/mhchem/MathJax-mhchem/gh-pages/index.html";

function fetchText(url) {
  return new Promise((resolve, reject) => {
    https
      .get(url, (res) => {
        if (res.statusCode !== 200) {
          reject(new Error(`HTTP ${res.statusCode}`));
          return;
        }
        const chunks = [];
        res.on("data", (c) => chunks.push(c));
        res.on("end", () => resolve(Buffer.concat(chunks).toString("utf8")));
      })
      .on("error", reject);
  });
}

function extractTextareaInner(html) {
  const m = html.match(
    /<textarea\s+id="ta"[^>]*>([\s\S]*?)<\/textarea>/i
  );
  if (!m) throw new Error('no <textarea id="ta"> in manual HTML');
  return m[1];
}

/** Mirror manual JS: trim trailing ` % comment`. */
function stripPercentComment(s) {
  const b = s.match(/^(.*?)(\s+%\s+.*)?$/s);
  return (b ? b[1] : s).trimEnd();
}

function manualLineToLatex(line) {
  if (/^::::pu\s+/.test(line)) {
    const body = line.replace(/^::::pu\s+/, "");
    return "\\pu{" + stripPercentComment(body) + "}";
  }
  if (/^::::\$\s+/.test(line)) {
    const body = line.replace(/^::::\$\s+/, "");
    return "$" + stripPercentComment(body) + "$";
  }
  if (/^::::\s+/.test(line)) {
    const body = line.replace(/^::::\s+/, "");
    return "\\ce{" + stripPercentComment(body) + "}";
  }
  return null;
}

function extractExamples(md) {
  const out = [];
  const seen = new Set();
  for (const line of md.split(/\r?\n/)) {
    const tex = manualLineToLatex(line);
    if (!tex) continue;
    if (seen.has(tex)) continue;
    seen.add(tex);
    out.push(tex);
  }
  return out;
}

async function main() {
  let html;
  const arg = process.argv[2];
  if (arg) {
    html = fs.readFileSync(path.resolve(arg), "utf8");
  } else {
    html = await fetchText(defaultUrl);
  }

  const md = extractTextareaInner(html);
  const lines = extractExamples(md);

  const header = [
    "# mhchem golden inputs — one formula per line (ratex golden_test mhchem suite).",
    "# Source: https://mhchem.github.io/MathJax-mhchem/ (all :::: / ::::pu / ::::$ examples).",
    "# Regenerate: node tools/extract_mhchem_manual_examples.mjs",
    "# Fixtures: KaTeX+mhchem → tests/golden/fixtures_ce/{NNNN}.png when enabling pixel compare.",
    "",
  ].join("\n");

  fs.writeFileSync(outPath, header + lines.join("\n") + "\n", "utf8");
  console.error("Wrote", outPath, "lines:", lines.length);
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
