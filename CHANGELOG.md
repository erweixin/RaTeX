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
  capped at 4096 entries and 64 MiB of decoded pixel data.
- **Fonts**: use a layered parsed-font cache. Small non-system fonts (up to
  4 MiB each) keep the `FontVec` fast path, while CJK, emoji, and other large
  fonts stay in shared immutable, `Arc`-backed owned buffers and are borrowed
  as `FontRef`, so a render does not retain a whole-font heap copy. CJK loads
  only its primary face; emoji and secondary CJK fallback files are loaded only
  after a glyph actually falls through to them. One per-render resolver then
  retains each parsed fallback face for PNG, standalone SVG, and Cairo, so
  every later glyph reuses the same `FontRef`. Canonical custom-font paths
  share one cached payload. Cache-owned raw font payloads and `FontVec` parsed
  payloads each have a 32 MiB aggregate budget; process-wide system-font
  buffers are shared but are not charged to those budgets. Concurrent cold
  loads share one parse per font generation. Raw-font and parsed-font caches
  also retain their 4096-entry bounds. Eviction only drops cache entries, so
  returned shared font handles remain valid.
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

### Latest main comparison and test device

Latest verification compares the previous `0.1.14` release (`public/main`) to
`Unreleased` after the raw-font cache byte budget and fallback-reuse changes.
It is a warmed, single-run sample
of the same 100-formula release benchmark; timings naturally vary with system
load.

| Metric | `0.1.14` | `Unreleased` | Change |
|---|---:|---:|---:|
| End-to-end PNG | 276 μs | 225 μs | −18% |
| PNG throughput | 3,623 formulas/s | 4,444 formulas/s | +23% |
| Maximum RSS | 416.1 MiB | 54.6 MiB | −87% |
| macOS peak memory footprint | 38.2 MiB | 38.4 MiB | +0.2 MiB |

The RSS reduction reflects avoiding whole-font heap copies and sharing
process-wide, `Arc`-backed owned system-font buffers. These buffers are not
memory-mapped and are outside the loader's 32 MiB raw/parsed cache budgets.
`peak memory footprint` uses a different macOS accounting method and remains
effectively unchanged in this sample.

Test device: Mac mini (Mac16,10), Apple M4 (10 CPU cores), 32 GB unified
memory, macOS 26.3.1 (25D2128), Darwin 25.3.0 / arm64. Command:
`/usr/bin/time -l cargo test --offline -p ratex-render --test bench_render --release -- --ignored --nocapture`.
