# Stack-safety and input-depth policy

RaTeX accepts untrusted LaTeX on native, mobile, and WebAssembly hosts. These
hosts have different thread-stack sizes, so parser-driven rendering must not
depend on a large native stack.

## Supported depth

The maximum supported input-controlled recursive or structural depth is **32**.
Depth 32 is accepted; depth 33 returns a `ParseError` whose message contains
`Recursion limit exceeded`.

The budget covers:

- recursively nested parser expressions, including groups, radicals,
  fractions, `\left...\right`, and scripts;
- Unicode combining-accent chains;
- `prooftree` branches;
- mhchem sub-state-machine calls and nested texify values.

Over-limit input is rejected. It is never truncated or partially ignored.
`parse`, `layout`, and `to_display_list` keep their existing public signatures,
and the DisplayList protocol is unchanged.

The supported rendering contract starts with ASTs returned by the parser.
Arbitrarily deep `ParseNode` values assembled manually by callers are outside
this guarantee.

## Implementation rules

Any code that converts flat input into a recursive structure must calculate and
enforce the same structural-depth budget, even if it does not recurse while
building that structure. This includes stack-based parsers such as
`prooftree`.

Prefer iterative traversal for read-only tree queries and output flattening.
Core layout recursion may rely on the parser's 32-level AST contract. Do not add
stack-growth libraries, assembly shims, or new unsafe dependencies to work
around an input-depth problem.

When adding a new recursive syntax path:

1. reject the 33rd level before entering its recursive implementation;
2. return a clear parse error rather than panicking or dropping input;
3. test depths 32, 33, and an adversarial depth such as 300;
4. include a 512 KiB isolated-process regression when a stack overflow could
   abort the test runner;
5. verify the WASM boundary returns a JavaScript error rather than a trap.
