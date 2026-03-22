#!/usr/bin/env node
/**
 * One-shot: extract serializable mhchem state machines + regex pattern sources from KaTeX mhchem.js.
 * Run: node tools/generate_mhchem_data.mjs
 * Writes: crates/ratex-parser/src/mhchem/data/machines.json + patterns_regex.json
 */
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";
import vm from "vm";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const root = path.join(__dirname, "..");
const srcPath = path.join(root, "tools/mhchem_reference.js");
const outDir = path.join(root, "crates/ratex-parser/src/mhchem/data");
fs.mkdirSync(outDir, { recursive: true });

const src = fs.readFileSync(srcPath, "utf8");
const stub =
  "var katex = { __defineMacro: function () {} };\n" +
  src.split("\n").filter((l) => !l.match(/^\s*import\s+katex/)).join("\n");

const ctx = { console };
vm.createContext(ctx);
vm.runInContext(stub, ctx);
const mp = ctx.mhchemParser;
if (!mp?.stateMachines) throw new Error("no mhchemParser");

function serializeTask(task) {
  return {
    nextState: task.nextState ?? null,
    revisit: !!task.revisit,
    toContinue: !!task.toContinue,
    action_: task.action_ ?? [],
  };
}

const machines = {};
for (const name of Object.keys(mp.stateMachines)) {
  const sm = mp.stateMachines[name];
  const tr = sm.transitions;
  const outStates = {};
  for (const st of Object.keys(tr)) {
    outStates[st] = tr[st].map(({ pattern, task }) => ({
      pattern,
      task: serializeTask(task),
    }));
  }
  machines[name] = { transitions: outStates, hasLocalActions: !!sm.actions };
}

const patternsObj = mp.patterns.patterns;
const regexPatterns = {};
const isReg = (v) =>
  v && typeof v.exec === "function" && typeof v.source === "string";
for (const k of Object.keys(patternsObj)) {
  const v = patternsObj[k];
  if (isReg(v)) {
    regexPatterns[k] = v.source;
  }
}
const functionPatterns = Object.keys(patternsObj).filter(
  (k) => !isReg(patternsObj[k])
);

fs.writeFileSync(
  path.join(outDir, "machines.json"),
  JSON.stringify(machines, null, 0),
  "utf8"
);
fs.writeFileSync(
  path.join(outDir, "patterns_regex.json"),
  JSON.stringify({ regex: regexPatterns, functionKeys: functionPatterns.sort() }, null, 2),
  "utf8"
);

console.log(
  "machines:",
  Object.keys(machines).join(", "),
  "regex patterns:",
  Object.keys(regexPatterns).length,
  "function patterns:",
  functionPatterns.length
);
