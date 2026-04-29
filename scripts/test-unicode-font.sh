#!/usr/bin/env bash
# test-unicode-font.sh — Batch-test RATEX_UNICODE_FONT with TTF and OTF fonts
#
# Usage:
#   ./scripts/test-unicode-font.sh              # run all font tests
#   ./scripts/test-unicode-font.sh --quick      # only TTF + OTF representative samples
#   ./scripts/test-unicode-font.sh --renderer render|render-svg|render-pdf  # single renderer
#   ./scripts/test-unicode-font.sh --build      # rebuild binaries first, then test
#
# Note: render-svg requires standalone feature for CJK glyph paths.
#       Use --build flag to rebuild all three binaries with correct features.
#
# Each font gets its own output subdirectory under test-output/.
# Set BIN_DIR to override the binary location (default: target/release).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
BIN_DIR="${BIN_DIR:-$PROJECT_DIR/target/release}"
FORMULAS="$SCRIPT_DIR/test-formulas.txt"
OUTPUT_ROOT="$PROJECT_DIR/test-output"

# ---- font candidate list (TTF and OTF) ------------------------------------
# Format: "label|path"
# These are macOS paths; edit or extend for your OS.
# Note: true CJK coverage requires fonts > 5 MB. Small subset fonts will render
# CJK characters as invisible/tofu. TTC (TrueType Collection) is not supported.

TTF_FONTS=(
  "ArialUnicode|/Library/Fonts/Arial Unicode.ttf"
  "AppleGothic|/System/Library/Fonts/Supplemental/AppleGothic.ttf"
  "AppleMyungjo|/System/Library/Fonts/Supplemental/AppleMyungjo.ttf"
)

# No CJK-capable OTF fonts are shipped with macOS.
# NotoSansCJKsc-Regular.otf is downloaded by the setup step below.
OTF_FONTS=(
  "NotoSansCJKsc|$PROJECT_DIR/scripts/fonts/NotoSansCJKsc-Regular.otf"
)

# ---------------------------------------------------------------------------
# helpers
# ---------------------------------------------------------------------------

dim()  { printf '\033[2m%s\033[0m\n' "$*"; }
bold() { printf '\033[1m%s\033[0m\n' "$*"; }
ok()   { printf '\033[32m  OK  \033[0m %s\n' "$*"; }
err()  { printf '\033[31m  ERR \033[0m %s\n' "$*"; }
warn() { printf '\033[33m  WARN \033[0m %s\n' "$*"; }

probe_fonts() {
  # Reads entries from the named array via indirect expansion and prints
  # "label|path" lines for every entry whose path exists on disk.
  local _arr_name="$1"
  eval "local _entries=(\"\${${_arr_name}[@]}\")"
  for _entry in "${_entries[@]}"; do
    local _path="${_entry#*|}"
    if [[ -f "$_path" ]]; then
      echo "$_entry"
    else
      warn "font not found, skipping: $_path"
    fi
  done
}

font_has_cjk() {
  # Heuristic: fonts with CJK coverage are typically > 5 MB (CJK glyph set is large).
  # Small OTF/TTF files (< 100 KB) are definitely subsets without CJK.
  local _path="$1"
  local _size
  _size=$(wc -c < "$_path" 2>/dev/null || echo 0)
  if [[ "$_size" -gt 5000000 ]]; then
    return 0  # large enough to potentially have CJK
  elif [[ "$_size" -lt 100000 ]]; then
    return 1  # too small for CJK coverage
  else
    # Mid-size: check if the font name or path suggests CJK
    if [[ "$_path" =~ [Cc][Jj][Kk]|unihan|[Uu]nicode|[Gg]othic|[Mm]yungjo|[Pp]ing[Ff]ang|[Hh]eiti|[Ss]ongti ]]; then
      return 0
    fi
    return 1
  fi
}

run_renderer() {
  local _label="$1" _font_path="$2" _renderer="$3"
  local _out_dir="$OUTPUT_ROOT/${_label}/${_renderer}"
  mkdir -p "$_out_dir"

  local _bin="$BIN_DIR/$_renderer"
  if [[ ! -x "$_bin" ]]; then
    err "binary not found: $_bin"
    return 1
  fi

  local _font_info
  if font_has_cjk "$_font_path"; then
    _font_info="$_font_path [CJK-capable]"
  else
    _font_info="$_font_path [! no CJK glyphs]"
  fi
  dim "  -> $_renderer  (RATEX_UNICODE_FONT=$_font_info)"

  RATEX_UNICODE_FONT="$_font_path" \
    "$_bin" \
      --font-dir "$PROJECT_DIR/fonts" \
      --output-dir "$_out_dir" \
      --font-size 40 \
      < "$FORMULAS" 2>&1 | while IFS= read -r line; do
        case "$line" in
          OK*)   dim "    $line" ;;
          ERR*)  err "    $line" ;;
          *)     dim "    $line" ;;
        esac
      done
}

# ---- main ------------------------------------------------------------------

main() {
  local _mode="full"
  local _target_renderer=""
  local _do_build=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --quick) _mode="quick"; shift ;;
      --build) _do_build=1; shift ;;
      --renderer)
        _target_renderer="$2"
        case "$_target_renderer" in
          render|render-svg|render-pdf) ;;
          *) err "invalid --renderer: $_target_renderer (expected render, render-svg, or render-pdf)"; exit 2 ;;
        esac
        shift 2 ;;
      -h|--help)
        echo "Usage: $0 [--quick] [--build] [--renderer render|render-svg|render-pdf]"
        exit 0 ;;
      *) err "unknown flag: $1"; exit 2 ;;
    esac
  done

  # Resolve renderers to run
  local _renderers=()
  if [[ -n "$_target_renderer" ]]; then
    _renderers=("$_target_renderer")
  else
    _renderers=("render" "render-svg" "render-pdf")
  fi

  # ---- build -------------------------------------------------------------
  if [[ $_do_build -eq 1 ]]; then
    bold "Building renderers..."
    cargo build --release -p ratex-render --bin render || { err "render build failed"; exit 1; }
    cargo build --release -p ratex-svg --features "cli,standalone" --bin render-svg || { err "render-svg build failed"; exit 1; }
    cargo build --release -p ratex-pdf --features "cli" --bin render-pdf || { err "render-pdf build failed"; exit 1; }
    ok "All binaries built"
  fi
  if [[ ! -f "$FORMULAS" ]]; then
    err "formulas file missing: $FORMULAS"
    exit 1
  fi

  for _r in "${_renderers[@]}"; do
    if [[ ! -x "$BIN_DIR/$_r" ]]; then
      err "binary not found: $BIN_DIR/$_r -- build it first: cargo build --release"
      exit 1
    fi
  done

  rm -rf "$OUTPUT_ROOT"
  mkdir -p "$OUTPUT_ROOT"

  echo ""
  bold "=== RaTeX Unicode Font Rendering Test ==="
  dim "Formulas : $FORMULAS"
  dim "Output   : $OUTPUT_ROOT"
  dim "Renderers: ${_renderers[*]}"
  echo ""

  # ---- probe fonts --------------------------------------------------------
  local _ttf_list _otf_list _ttf_count _otf_count
  _ttf_list="$(probe_fonts TTF_FONTS)"

  # Convert newline-separated strings to arrays (bash 3.2 compat)
  local _ttf_found=() _otf_found=()
  local _ifs_save="$IFS"

  if [[ -n "$_ttf_list" ]]; then
    IFS=$'\n'
    _ttf_found=($_ttf_list)
    IFS="$_ifs_save"
  fi
  _ttf_count=${#_ttf_found[@]}

  # OTF probe — only if OTF_FONTS array is non-empty
  if [[ ${#OTF_FONTS[@]} -gt 0 ]]; then
    _otf_list="$(probe_fonts OTF_FONTS)"
    if [[ -n "$_otf_list" ]]; then
      IFS=$'\n'
      _otf_found=($_otf_list)
      IFS="$_ifs_save"
    fi
    _otf_count=${#_otf_found[@]}
  else
    _otf_count=0
  fi

  if [[ "$_mode" == "quick" ]]; then
    # Representative samples: first 2 TTF + first 1 OTF
    local _qttf=() _qotf=()
    [[ -n "${_ttf_found[0]:-}" ]] && _qttf+=("${_ttf_found[0]}")
    [[ -n "${_ttf_found[1]:-}" ]] && _qttf+=("${_ttf_found[1]}")
    [[ -n "${_otf_found[0]:-}" ]] && _qotf+=("${_otf_found[0]}")
    if [[ ${#_qttf[@]} -gt 0 ]]; then
      _ttf_found=("${_qttf[@]}")
    else
      _ttf_found=()
    fi
    if [[ ${#_qotf[@]} -gt 0 ]]; then
      _otf_found=("${_qotf[@]}")
    else
      _otf_found=()
    fi
    bold "Quick mode: ${#_ttf_found[@]} TTF + ${#_otf_found[@]} OTF fonts"
  else
    bold "Full mode: $_ttf_count TTF + $_otf_count OTF fonts"
  fi
  echo ""

  # ---- TTF round -----------------------------------------------------------
  if [[ ${#_ttf_found[@]} -gt 0 ]]; then
    bold "=== TTF Fonts ==="
    for _entry in "${_ttf_found[@]}"; do
      [[ -z "$_entry" ]] && continue
      local _label="${_entry%%|*}"
      local _path="${_entry#*|}"
      echo ""
      bold "Font: $_label"
      dim "  Path: $_path  (TTF)"

      for _r in "${_renderers[@]}"; do
        run_renderer "$_label" "$_path" "$_r" || true
      done

      # Quick CJK check on PNG output for formula 0005
      local _png_0005="$OUTPUT_ROOT/${_label}/render/0005.png"
      if [[ -f "$_png_0005" ]]; then
        local _png_size
        _png_size=$(wc -c < "$_png_0005" 2>/dev/null || echo 0)
        if [[ "$_png_size" -lt 6000 ]]; then
          warn "CJK glyphs may be INVISIBLE: $_label (0005.png = $_png_size bytes, expected > 10K for visible CJK)"
        else
          dim "  CJK check: 0005.png = $_png_size bytes (CJK visible)"
        fi
      fi
    done
  fi

  # ---- OTF round -----------------------------------------------------------
  if [[ ${#_otf_found[@]} -gt 0 ]]; then
    echo ""
    bold "=== OTF Fonts ==="
    for _entry in "${_otf_found[@]}"; do
      [[ -z "$_entry" ]] && continue
      local _label="${_entry%%|*}"
      local _path="${_entry#*|}"
      echo ""
      bold "Font: $_label"
      dim "  Path: $_path  (OTF)"

      for _r in "${_renderers[@]}"; do
        run_renderer "$_label" "$_path" "$_r" || true
      done

      # Quick CJK check on PNG output for formula 0005
      local _png_0005="$OUTPUT_ROOT/${_label}/render/0005.png"
      if [[ -f "$_png_0005" ]]; then
        local _png_size
        _png_size=$(wc -c < "$_png_0005" 2>/dev/null || echo 0)
        if [[ "$_png_size" -lt 6000 ]]; then
          warn "CJK glyphs may be INVISIBLE: $_label (0005.png = $_png_size bytes, expected > 10K for visible CJK)"
        else
          dim "  CJK check: 0005.png = $_png_size bytes (CJK visible)"
        fi
      fi
    done
  fi

  # ---- summary -------------------------------------------------------------
  echo ""
  bold "=== Done ==="
  dim "Output tree:"
  if command -v tree &>/dev/null; then
    tree -L 2 "$OUTPUT_ROOT" 2>/dev/null || find "$OUTPUT_ROOT" -type f | sort | while read -r f; do
      dim "  ${f#$OUTPUT_ROOT/}"
    done
  else
    find "$OUTPUT_ROOT" -type f | sort | while read -r f; do
      dim "  ${f#$OUTPUT_ROOT/}"
    done
  fi

  for _r in "${_renderers[@]}"; do
    local _count
    _count=$(find "$OUTPUT_ROOT" -path "*/$_r/*" -type f 2>/dev/null | wc -l | tr -d ' ')
    ok "$_r: $_count output files"
  done
}

main "$@"
