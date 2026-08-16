# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

When releasing (see `RELEASING.md`), `scripts/set-version.sh` renames
`[Unreleased]` to the new version + date and starts a fresh `[Unreleased]`
section.

## [Unreleased]

### Performance

- **PNG**: cache rasterized glyph masks. Keys are the exact font, glyph, size,
  and sub-pixel position phase, so cache hits are pixel-identical; repeated
  renders of the same formula (live preview, batch re-renders) become plain
  pixel blits instead of curve flattening + anti-aliased fills. Cache capped
  at 8192 entries.
- **PNG**: faster encoding via the `png` crate with `Compression::Fast` and
  `Sub` filtering (identical decoded pixels, smaller CPU cost).
- **SVG**: allocation-free serialization. Numbers (`fmt_num`), paint colors,
  opacity attributes, character escaping, and glyph path data are written
  directly into the output buffer instead of building per-value `String`s;
  output is byte-identical.

### Benchmarks

Measured on the same machine with the 100-formula render benchmark
(`cargo test -p ratex-render --test bench_render --release -- --ignored
--nocapture`), before → after:

| Metric | Before | After | Change |
|---|---|---|---|
| PNG render | 601 μs | 279 μs | −54% |
| SVG (text glyphs) | 65 μs | 32 μs | −51% |
| SVG standalone (path glyphs) | 669 μs | 331 μs | −51% |
| End-to-end throughput (PNG) | 1292 formulas/s | 2110 formulas/s | +63% |

Steady-state render phase (`phase_breakdown`, repeated rendering):
`x^2 + y^2 = z^2` 337 → 103 μs (−69%), `\frac+\int+\sum` 970 → 367 μs (−62%),
matrix 769 → 287 μs (−63%), CJK 577 → 243 μs (−58%), emoji 420 → 196 μs (−53%).

Quality is unchanged: golden ink scores are identical (main suite 0.9019,
mhchem 0.8814), and PNG pixels / SVG bytes match the previous implementation.
