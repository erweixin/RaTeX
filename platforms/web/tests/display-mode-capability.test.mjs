import assert from "node:assert/strict";
import test from "node:test";

async function loadFreshModule(name) {
  return import(`../dist/index.js?test=${name}`);
}

test("an explicit capability supports positional functions with default parameters", async () => {
  const ratex = await loadFreshModule("explicit-capability");
  const calls = [];
  function renderLatex(latex, color, displayMode = true) {
    calls.push({ latex, color, displayMode });
    return "{}";
  }

  assert.equal(renderLatex.length, 2, "the regression requires Function.length to be misleading");
  await ratex.initRatex(async () => ({
    renderLatex,
    capabilities: { displayMode: true },
  }));

  ratex.renderLatex("x", undefined, false);
  ratex.renderLatexWithOptions("y", {
    color: { r: 1, g: 0.5, b: 0, a: 1 },
    displayMode: false,
  });

  assert.deepEqual(calls, [
    { latex: "x", color: undefined, displayMode: false },
    { latex: "y", color: "#ff8000ff", displayMode: false },
  ]);
});

test("the options export is detected as displayMode support", async () => {
  const ratex = await loadFreshModule("options-export");
  const calls = [];
  await ratex.initRatex(async () => ({
    renderLatex: () => "{}",
    renderLatexWithOptions(latex, options) {
      calls.push({ latex, options });
      return "{}";
    },
  }));

  ratex.renderLatex("x", undefined, false);

  assert.deepEqual(calls, [
    { latex: "x", options: { color: undefined, displayMode: false } },
  ]);
});

test("inline mode is rejected when an injected module declares no capability", async () => {
  const ratex = await loadFreshModule("legacy-module");
  await ratex.initRatex(async () => ({
    renderLatex: () => "{}",
  }));

  assert.throws(
    () => ratex.renderLatex("x", undefined, false),
    /does not support displayMode/
  );
});
