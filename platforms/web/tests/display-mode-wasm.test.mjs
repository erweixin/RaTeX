import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import * as ratex from "../dist/index.js";
import initWasm, * as wasmModule from "../pkg/ratex_wasm.js";

let wasmInitPromise;

function initGeneratedWasm() {
  if (!wasmInitPromise) {
    const wasmUrl = new URL("../pkg/ratex_wasm_bg.wasm", import.meta.url);
    wasmInitPromise = readFile(wasmUrl).then((moduleBytes) =>
      initWasm({ module_or_path: moduleBytes })
    );
  }
  return wasmInitPromise;
}

test("the generated WASM honors displayMode through every public API", async () => {
  await initGeneratedWasm();

  const latex = "\\frac{1}{2}";
  const defaultDisplay = wasmModule.renderLatex(latex);
  const explicitDisplay = wasmModule.renderLatex(latex, undefined, true);
  const positionalInline = wasmModule.renderLatex(latex, undefined, false);
  const optionsInline = wasmModule.renderLatexWithOptions(latex, {
    displayMode: false,
  });

  assert.equal(defaultDisplay, explicitDisplay);
  assert.notEqual(explicitDisplay, positionalInline);
  assert.equal(positionalInline, optionsInline);

  const displayList = JSON.parse(explicitDisplay);
  const inlineList = JSON.parse(positionalInline);
  assert.ok(inlineList.height < displayList.height);
  assert.ok(inlineList.depth < displayList.depth);

  await ratex.initRatex(async () => wasmModule);
  assert.equal(ratex.renderLatex(latex, { displayMode: false }), positionalInline);
  assert.equal(
    ratex.renderLatex(latex, undefined, false),
    positionalInline
  );
});

test("over-nested input returns a JS error instead of trapping WASM", async () => {
  await initGeneratedWasm();

  const overNested = `${"\\sqrt{".repeat(300)}x${"}".repeat(300)}`;
  assert.throws(
    () => wasmModule.renderLatex(overNested),
    /Recursion limit exceeded/
  );
});
