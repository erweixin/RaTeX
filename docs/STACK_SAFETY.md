# Stack-safety and input-depth policy

RaTeX accepts untrusted LaTeX on native, mobile, and WebAssembly hosts. These
hosts have different thread-stack sizes, so parser-driven rendering must not
depend on a large native stack.

## Supported depth

The maximum supported input-controlled recursive or structural depth is **32**.
For a single recursive structure, depth 32 is accepted subject to the independent
resource limits below; depth 33 returns a `ParseError` whose message contains
`Recursion limit exceeded`.

The budget is cumulative across enclosing structures. For example, an accent
chain inside a braced group consumes both the group depth and the accent depth;
each category does not receive a separate depth-32 allowance.

The budget covers:

- recursively nested parser expressions, including groups, radicals,
  fractions, `\left...\right`, and scripts;
- unbraced structural arguments;
- nested macro expansion paths that synchronously re-enter expansion;
- Unicode combining-accent chains;
- `prooftree` branches, conclusions, and labels;
- mhchem sub-state-machine calls and nested texify values.

Some syntaxes spend budget in helper layers as well as in the visible input
shape. In particular, mhchem enters internal sub-state machines while parsing
`\ce` / `\pu`, so visible nested `\ce{...}` accepts depth 31 and rejects depth
32.

Over-limit input is rejected. It is never truncated or partially ignored.
`parse`, `layout`, and `to_display_list` keep their existing public signatures.
The additive `validate_layout(&LayoutBox) -> Result<(), String>` API checks
geometry-dependent output costs after layout and before DisplayList expansion.
DisplayList conversion remains an infallible transformation; its JSON protocol
is unchanged.

The supported rendering contract starts with ASTs returned by the parser.
Arbitrarily deep `ParseNode` values assembled manually by callers are outside
this guarantee.

## Implementation rules

Parser, macro expansion, mhchem, and iterative builders share one RAII
`DepthBudget`. Do not give a subsystem its own independent 32-level allowance.
The parser also validates the final AST with an iterative logical-depth walk
before returning it.

Any code that converts flat input into a recursive structure must calculate and
enforce the same structural-depth budget, even if it does not recurse while
building that structure. This includes stack-based parsers such as
`prooftree`.

Flat arguments do not consume structural depth. This includes raw strings,
URLs, colors, sizes, and delimiter tokens for `\left`, `\right`, `\middle`, and
`\big*` commands. Do not implement a separate LaTeX source preflight scanner for
these cases; guard the real parser and expansion paths instead.

Prefer iterative traversal for read-only tree queries and output flattening.
Core layout recursion may rely on the parser's 32-level AST contract. Do not add
stack-growth libraries, assembly shims, or new unsafe dependencies to work
around an input-depth problem.

When adding a new recursive syntax path:

1. reject the first level that would exceed the shared budget before entering
   its recursive implementation;
2. return a clear parse error rather than panicking or dropping input;
3. test the accepted boundary, the first rejected boundary, and an adversarial
   depth such as 300;
4. include a 512 KiB isolated-process regression when a stack overflow could
   abort the test runner;
5. verify the WASM boundary returns a JavaScript error rather than a trap.

## Layout and raster resource limits

Stack depth alone does not bound repeated layout or paint work. The parser also
rejects ASTs whose conservative expanded work exceeds **65,536 units**, with
`Layout resource limit exceeded`. Each AST node, proof branch, and array
horizontal rule (`\hline` or `\hdashline`, at every row boundary) contributes
one unit; textual leaves contribute their character count, at least one.
Array/CD padding contributes one unit per missing trailing cell. Supplied cells
already contribute through their AST nodes, so the total includes at least
`rows × maximum columns` units for each rectangular grid. Row deficits are
summed with saturating arithmetic, without allocating padded cells in the AST.
Every enclosing `\pmb` doubles these contributions. All branches, array tags,
and proof payloads count, including branches that will not be displayed. Thus a
bare 15-level `\pmb{...x...}` is accepted, but 16 levels are rejected. This budget is
separate from the depth limit and bounds AST-driven repeated layout work.
As with depth safety, this guarantee applies to parser-produced ASTs.

Layout validation (`validate_layout`) has an independent output-work limit of **65,536
estimated drawing items** (`ratex_layout::MAX_LAYOUT_OUTPUT_ITEMS`). After building
the owned layout tree, an iterative walk accumulates each box's cost
without allocating DisplayItems. Array vertical dashed separators use the completed
array height and rule thickness to determine their segment count. Ordinary
primitives, horizontal rules, cells, tags, and proof payloads also contribute.
Every copied subtree contributes its full cost, so nested `\pmb` layers and
sibling arrays cannot evade the cumulative limit. An excessive cost returns `Layout resource limit exceeded`; invalid dashed-rule dimensions
return `Invalid dashed separator geometry`. Both failures happen during
validation, before DisplayList expansion.

The vertical dash segment count is computed in local coordinates and reused by
DisplayList conversion. Segments are placed by index, so translation, scaling,
and floating-point accumulation cannot create extra iterations or stall the loop.
This can remove negligible trailing slivers caused by the previous accumulating
loop at exact period boundaries; the dash length, gap, and JSON primitives stay
the same.

Existing Rust callers may continue using `layout` and `to_display_list` without
source changes. `layout` constructs the tree without checking output work. Callers
handling untrusted input should explicitly validate the completed tree:

```rust
let layout_box = layout(&ast, &options);
validate_layout(&layout_box)?;
let display_list = to_display_list(&layout_box);
```

Rust hosts that measure fallback fonts can instead call
`layout_with_fallback_metrics(&ast, &options, metrics)`, where `metrics` is an
`Arc<HashMap<char, FallbackGlyphMetrics>>`. This additive entry point keeps
`LayoutOptions` source-compatible with v0.1.14, including full struct literals
and exhaustive patterns. It uses the same layout engine and requires the same
`validate_layout` step before DisplayList conversion. An empty map is equivalent
to `layout`; unlisted characters keep the built-in estimates and KaTeX glyphs
keep their own metrics. Values are measured in em at the base font size, using
the host's rendering font; layout applies style and explicit-size scaling. The
map is shared only for the duration of the call and is not a global setting.
It neither loads fonts nor changes the renderer's font selection.

FFI, WASM, GTK, and the PNG/SVG/PDF command-line tools perform this validation
and propagate its errors before DisplayList conversion, serialization, or
rendering. Their external signatures remain unchanged. Layout construction,
validation, and DisplayList conversion remain separate steps.

`\pmb` lays out its body once and clones the result into the original owned
`LayoutBox` tree. `BoxContent` retains its existing exhaustive public variants;
there is no sharing variant or change to public field types. The parser's
expanded-work budget bounds the copied structure. Unlike a shared DAG, deeply
nested bold allocates in proportion to that expanded structure; resource
regressions enforce a 64 MiB cumulative allocation guard per layout/validation
or DisplayList-conversion phase, not a whole-process memory limit.

Nested `\left...\middle...\right` scopes cache first-pass delimiter heights
within one layout call. Cache keys include AST content and scalar layout options
(with floating-point values represented by their bits). Inherited middle height
is reset for measurement. The fallback glyph metrics map is immutable and shared
throughout the call, so it is not serialized or hashed for each key. Different
calls always use separate caches, including when their fallback metrics differ;
entries retain only measurements, not expanded layout subtrees. A cache hit
skips recursive first-pass measurement, avoiding exponential two-pass layout.
Temporary measurement trees are dropped before building the final tree, and
no cache survives the call. Array/CD layout rows preserve the rectangular public
representation: each row has `col_widths.len()` cells, including empty trailing
boxes. The parser rejects excessive padded-grid work before allocating any
layout tree, including padding copied by enclosing `\pmb` layers and cumulative
costs across sibling arrays. As with the other AST limits, hand-built ASTs that
bypass the parser are outside this guarantee.

The PNG renderer rejects canvases larger than **16,777,216 pixels** (64 MiB of
RGBA canvas storage) before allocation, reporting `PNG pixel limit exceeded`.
The budget includes padding and device pixel ratio. Nonfinite dimensions and
options, nonpositive font sizes or pixel ratios, negative widths/total heights,
and negative padding return errors. PNG encoding and font storage use additional
memory; 64 MiB is a canvas limit, not a total-process memory guarantee.

Horizontal dashed lines remain one DisplayList item. The PNG renderer clips
work to the visible viewport and preserves the original dash phase. It uses
f64 arithmetic and indexed segments instead of an accumulating f32 loop, so
very long offscreen lines cannot stall or iterate over their invisible length.
This bound belongs to rasterization because it depends on the pixel viewport;
the layout item-count limit alone does not bound backend drawing work.
