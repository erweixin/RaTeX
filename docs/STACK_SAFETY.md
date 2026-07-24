# Stack-safety and input-depth policy

RaTeX accepts untrusted LaTeX on native, mobile, and WebAssembly hosts. These
hosts have different thread-stack sizes, so parser-driven rendering must not
depend on a large native stack.

## Supported depth

The maximum supported input-controlled recursive or structural depth is **32**.
For a single recursive structure, depth 32 is accepted and depth 33 returns a
`ParseError` whose message contains `Recursion limit exceeded`.

The budget is cumulative across enclosing structures. For example, an accent
chain inside a braced group consumes both the group depth and the accent depth;
each category does not receive a separate depth-32 allowance.

The budget covers:

- recursively nested parser expressions, including groups, radicals,
  fractions, `\left...\right`, and scripts;
- unbraced structural arguments such as nested `\tag` chains;
- nested macro expansion paths that synchronously re-enter expansion;
- Unicode combining-accent chains;
- `prooftree` branches, conclusions, and labels;
- mhchem sub-state-machine calls and nested texify values.

Some syntaxes spend budget in helper layers as well as in the visible input
shape. In particular, mhchem enters internal sub-state machines while parsing
`\ce` / `\pu`, so visible nested `\ce{...}` accepts depth 31 and rejects depth
32.

Over-limit input is rejected. It is never truncated or partially ignored.
`parse`, `layout`, and `to_display_list` keep their existing public signatures,
and the DisplayList protocol is unchanged.

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
