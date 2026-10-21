# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

When releasing (see `RELEASING.md`), `scripts/set-version.sh` renames
`[Unreleased]` to the new version + date and starts a fresh `[Unreleased]`
section.

## [Unreleased]

### Added

- Optional `mhchem` cargo feature (default on) for `ratex-parser`, forwarded by every crate that
  depends on it. Disable default features to build without `\ce` / `\pu` and the `regex` /
  `fancy-regex` dependencies. See *Building without mhchem* in the README.

## [0.1.15] - 2026-10-16

### Compatibility

- `LayoutOptions` keeps the v0.1.14 fields and `layout(nodes, options)` keeps its
  signature. Existing full struct literals and exhaustive patterns remain valid.
  Host-provided glyph metrics use the additive `layout_with_fallback_metrics`
  API; no caller migration is required. The C ABI and DisplayList JSON remain
  unchanged.

### Added

- `\slashed` and `\centernot` commands.
- Mac Catalyst support in the Apple XCFramework and Swift package.
- `layout_with_fallback_metrics(nodes, options, metrics)` for host-provided
  fallback glyph metrics and more accurate text spacing. See the API rustdoc
  for an example and [layout validation](docs/STACK_SAFETY.md) for resource limits.
- Configurable PNG/SVG padding in the CLI.
- Borrowed lexer tokens via `Lexer::next_span()` and `TokenSpan`.

### Fixed

- PNG rendering reports malformed large-font errors instead of succeeding with missing glyphs.
- PNG glyphs shifting by one pixel after cache reuse at different positions.
- mhchem spacing regression caused by escaped `##` source locations.
- Color inheritance for rules, boxes, and related symbols.
- Website rendering before fonts finish loading.
- Website font-loading failures now time out and allow retrying without reloading the page.
- Delimiter cache keys no longer serialize the host's fallback glyph metrics map on every lookup.
- iOS linking failures involving unwind symbols.

### Security

- Bound macro expansion, mhchem intermediates, and layout work, including
  `\edef`/`\xdef`, `\pmb`, and Array/CD padding.
- Reject overflowing `\char` values and oversized PNG allocations.
- Bound dashed-line rendering and clip work to the viewport.
- Enforce Android 16 KB ELF alignment and verify release packages.

See [resource limits](docs/EXPANSION_RESOURCE_LIMITS.md) for scope and limitations.

### Performance

- Cache glyph masks and emoji rasters; share font buffers and load fallback
  fonts on demand.
- Reuse built-in macros and delimiter measurements; reduce allocations in
  PNG encoding and SVG serialization.

Latest comparison with v0.1.14 (lower is better):

| Metric | v0.1.14 | Unreleased | Change |
|---|---:|---:|---:|
| PNG render | 717 μs | 339 μs | −53% |
| SVG standalone | 938 μs | 403 μs | −57% |
| End-to-end PNG | 1,030 μs | 506 μs | −51% |
| Peak RSS | 389.5 MiB | 218.6 MiB | −44% |

Measured on x86_64 macOS with Rust 1.95.0 and matching dependencies: 100 formulas
in release mode, one warmup and three measured renders per formula, median of
five alternating runs. RSS excludes compilation. These are warmed-render
results; cold renders and continually changing glyph positions may differ.
