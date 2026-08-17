# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

When releasing (see `RELEASING.md`), `scripts/set-version.sh` renames
`[Unreleased]` to the new version + date and starts a fresh `[Unreleased]`
section.

## [Unreleased]

### Performance

- **PNG**: cache rasterized glyph masks and decoded color-emoji strikes. Glyph
  mask keys are the exact font, glyph, size, sub-pixel position phase, and
  quantized paint color, so cache hits are pixel-identical; repeated renders
  of the same formula (live preview, batch re-renders) become plain pixel
  blits instead of curve flattening + anti-aliased fills. Glyph-mask cache
  capped at 8192 entries and 64 MiB of pixel data; decoded emoji-strike cache
  capped at 4096 entries.
- **Fonts**: bound the raw-font, parsed-font, and font-source caches at 4096
  entries each so long-running renderers with many distinct font directories
  do not grow without limit. Clearing only drops cache entries; returned
  `Arc`-backed font handles remain valid.
- **PNG**: encode from a directly demultiplied RGBA buffer with a pre-sized
  encoder output buffer (and shrink it before returning). The `png` crate
  0.17 already defaults to `Compression::Fast`, `Sub` filtering, and
  non-adaptive filtering; the settings are kept explicit so output stays
  stable if defaults change.
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
