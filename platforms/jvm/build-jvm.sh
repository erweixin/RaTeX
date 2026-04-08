#!/usr/bin/env bash
# build-jvm.sh — Build libratex_ffi for JVM platforms
#
# Usage:
#   bash platforms/jvm/build-jvm.sh          # host platform only (cargo build)
#   bash platforms/jvm/build-jvm.sh --all    # all platforms (cargo-zigbuild)
#
# Prerequisites:
#   rustup (with the default host target installed)
#   --all mode additionally requires: zig, cargo-zigbuild
#
# Output: platforms/jvm/native/{os-arch}/libratex_ffi.{dylib,so,dll}

set -eo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
NATIVE_DIR="$REPO_ROOT/platforms/jvm/native"

# Rust target → JNA directory, library filename
#   target                        jna_dir          lib_file
TARGETS=(
    "aarch64-apple-darwin         darwin-aarch64   libratex_ffi.dylib"
    "x86_64-apple-darwin          darwin-x86-64    libratex_ffi.dylib"
    "aarch64-unknown-linux-gnu    linux-aarch64    libratex_ffi.so"
    "x86_64-unknown-linux-gnu     linux-x86-64     libratex_ffi.so"
    "x86_64-pc-windows-gnu        win32-x86-64     ratex_ffi.dll"
)

# Detect host Rust target triple
detect_host_target() {
    rustc -vV | awk '/^host:/ { print $2 }'
}

# Copy built library to native/{jna_dir}/
copy_lib() {
    local rust_target="$1" jna_dir="$2" lib_file="$3"
    local src="$REPO_ROOT/target/$rust_target/release/$lib_file"
    local dest="$NATIVE_DIR/$jna_dir"

    if [ ! -f "$src" ]; then
        echo "    ✗ $rust_target — $src not found" >&2
        return 1
    fi
    mkdir -p "$dest"
    cp "$src" "$dest/"
    echo "    ✓ $rust_target → $dest/$lib_file"
}

build_host() {
    local host_target
    host_target="$(detect_host_target)"

    echo "==> Building ratex-ffi for host: $host_target"
    cargo build --manifest-path "$REPO_ROOT/Cargo.toml" --release -p ratex-ffi

    # Find matching JNA dir for host target
    for entry in "${TARGETS[@]}"; do
        read -r target jna_dir lib_file <<< "$entry"
        if [ "$target" = "$host_target" ]; then
            # Host build output is in target/release/ (no target triple subdirectory)
            local src="$REPO_ROOT/target/release/$lib_file"
            local dest="$NATIVE_DIR/$jna_dir"
            if [ -f "$src" ]; then
                mkdir -p "$dest"
                cp "$src" "$dest/"
                echo "==> Done. $dest/$lib_file"
            else
                echo "==> Error: $src not found" >&2
                exit 1
            fi
            return
        fi
    done
    echo "==> Error: host target $host_target not in supported list" >&2
    exit 1
}

build_all() {
    # Check prerequisites
    command -v cargo-zigbuild >/dev/null 2>&1 || { echo "Error: cargo-zigbuild not found. Install with: cargo install cargo-zigbuild" >&2; exit 1; }
    command -v zig >/dev/null 2>&1 || { echo "Error: zig not found. Install with: brew install zig / apt install zig" >&2; exit 1; }

    # Ensure all Rust targets are installed
    local targets_to_add=()
    for entry in "${TARGETS[@]}"; do
        read -r target _ _ <<< "$entry"
        targets_to_add+=("$target")
    done
    rustup target add "${targets_to_add[@]}"

    echo "==> Building ratex-ffi for all platforms (parallel)..."
    local pids=()
    for entry in "${TARGETS[@]}"; do
        read -r target jna_dir lib_file <<< "$entry"
        echo "    → $target [starting]"
        (
            cargo zigbuild --manifest-path "$REPO_ROOT/Cargo.toml" \
                --release -p ratex-ffi --target "$target"
            copy_lib "$target" "$jna_dir" "$lib_file"
        ) &
        pids+=($!)
    done

    local failed=0
    for pid in "${pids[@]}"; do
        wait "$pid" || failed=1
    done

    if [ "$failed" -ne 0 ]; then
        echo "==> Build failed!" >&2
        exit 1
    fi
    echo "==> Done. Libraries copied to $NATIVE_DIR"
}

case "${1:-}" in
    --all) build_all ;;
    *)     build_host ;;
esac
