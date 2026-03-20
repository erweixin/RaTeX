#!/usr/bin/env node
/**
 * Load KaTeX mhchem.js with katex stubbed; dump state machine transition pattern names
 * and task metadata (no functions) for Rust port verification.
 */
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";
import vm from "vm";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, "..");
const src = fs.readFileSync(path.join(root, "tools/mhchem_reference.js"), "utf8");
const stub =
  "var katex = { __defineMacro: function () {} };\n" +
  "var chemParse, mhchemParser, texify;\n" +
  src
    .split("\n")
    .filter((line) => !line.match(/^\s*import\s+katex/))
    .join("\n");

const ctx = { console, require: () => ({}) };
vm.createContext(ctx);
vm.runInContext(stub, ctx);
const mp = ctx.mhchemParser;
if (!mp || !mp.stateMachines) {
  console.error("mhchemParser not found");
  process.exit(1);
}
for (const name of Object.keys(mp.stateMachines)) {
  const sm = mp.stateMachines[name];
  const tr = sm.transitions;
  let n = 0;
  for (const st of Object.keys(tr)) {
    n += tr[st].length;
  }
  console.log(name, "states", Object.keys(tr).length, "transitions", n);
}
