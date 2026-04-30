#!/usr/bin/env bash
# build-desktop.sh — Build libratex_ffi for Flutter desktop platforms
#
# Usage:
#   bash platforms/flutter/build-desktop.sh          # host only
#   bash platforms/flutter/build-desktop.sh --all    # all platforms (cargo-zigbuild)
#
# Prerequisites:
#   rustup (with the default host target installed)
#   --all mode additionally requires: zig, cargo-zigbuild
#
# Output:
#   platforms/flutter/macos/Libraries/libratex_ffi.dylib         (universal: arm64 + x86_64)
#   platforms/flutter/windows/ratex_ffi.dll                      (x86_64)
#   platforms/flutter/linux/lib/x86_64/libratex_ffi.so           (x86_64)
#   platforms/flutter/linux/lib/aarch64/libratex_ffi.so          (aarch64)

set -eo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MACOS_LIB="$REPO_ROOT/platforms/flutter/macos/Libraries/libratex_ffi.dylib"
WINDOWS_LIB="$REPO_ROOT/platforms/flutter/windows/ratex_ffi.dll"
LINUX_X86_64_LIB="$REPO_ROOT/platforms/flutter/linux/lib/x86_64/libratex_ffi.so"
LINUX_AARCH64_LIB="$REPO_ROOT/platforms/flutter/linux/lib/aarch64/libratex_ffi.so"

detect_host_target() { rustc -vV | awk '/^host:/ { print $2 }'; }

build_all() {
    command -v cargo-zigbuild >/dev/null 2>&1 || { echo "Error: cargo-zigbuild not found. Install with: cargo install cargo-zigbuild" >&2; exit 1; }
    command -v zig >/dev/null 2>&1 || { echo "Error: zig not found" >&2; exit 1; }

    rustup target add aarch64-apple-darwin x86_64-apple-darwin \
        x86_64-pc-windows-gnu x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu

    echo "==> Building ratex-ffi for all desktop platforms (parallel)..."
    local pids=()

    for target in aarch64-apple-darwin x86_64-apple-darwin \
                  x86_64-pc-windows-gnu x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu; do
        echo "    → $target [starting]"
        (
            cargo zigbuild --manifest-path "$REPO_ROOT/Cargo.toml" --release -p ratex-ffi --target "$target"
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

    # macOS: create universal dylib via lipo
    echo "==> Creating universal macOS dylib (arm64 + x86_64)..."
    mkdir -p "$(dirname "$MACOS_LIB")"
    lipo -create \
        "$REPO_ROOT/target/aarch64-apple-darwin/release/libratex_ffi.dylib" \
        "$REPO_ROOT/target/x86_64-apple-darwin/release/libratex_ffi.dylib" \
        -output "$MACOS_LIB"
    lipo -info "$MACOS_LIB"

    # Windows
    mkdir -p "$(dirname "$WINDOWS_LIB")"
    cp "$REPO_ROOT/target/x86_64-pc-windows-gnu/release/ratex_ffi.dll" "$WINDOWS_LIB"
    echo "    ✓ $WINDOWS_LIB"

    # Linux — per-arch directories (CMakeLists.txt selects at build time)
    mkdir -p "$(dirname "$LINUX_X86_64_LIB")"
    cp "$REPO_ROOT/target/x86_64-unknown-linux-gnu/release/libratex_ffi.so" "$LINUX_X86_64_LIB"
    echo "    ✓ $LINUX_X86_64_LIB"
    mkdir -p "$(dirname "$LINUX_AARCH64_LIB")"
    cp "$REPO_ROOT/target/aarch64-unknown-linux-gnu/release/libratex_ffi.so" "$LINUX_AARCH64_LIB"
    echo "    ✓ $LINUX_AARCH64_LIB"

    echo "==> Done. Libraries copied to platforms/flutter/{macos,windows,linux}/"
}

build_host() {
    local host; host="$(detect_host_target)"
    echo "==> Building ratex-ffi for host: $host"
    cargo build --manifest-path "$REPO_ROOT/Cargo.toml" --release -p ratex-ffi

    case "$host" in
        aarch64-apple-darwin|x86_64-apple-darwin)
            mkdir -p "$(dirname "$MACOS_LIB")"
            cp "$REPO_ROOT/target/release/libratex_ffi.dylib" "$MACOS_LIB"
            echo "==> Done. $MACOS_LIB" ;;
        x86_64-pc-windows-gnu)
            mkdir -p "$(dirname "$WINDOWS_LIB")"
            cp "$REPO_ROOT/target/release/ratex_ffi.dll" "$WINDOWS_LIB"
            echo "==> Done. $WINDOWS_LIB" ;;
        x86_64-unknown-linux-gnu|aarch64-unknown-linux-gnu)
            local arch_dir="${host%%-*}"  # "x86_64" or "aarch64"
            mkdir -p "$REPO_ROOT/platforms/flutter/linux/lib/$arch_dir"
            cp "$REPO_ROOT/target/release/libratex_ffi.so" "$REPO_ROOT/platforms/flutter/linux/lib/$arch_dir/"
            echo "==> Done. $REPO_ROOT/platforms/flutter/linux/lib/$arch_dir/libratex_ffi.so" ;;
        *) echo "==> Error: unsupported host target $host for desktop build" >&2; exit 1 ;;
    esac
}

case "${1:-}" in
    --all) build_all ;;
    *)     build_host ;;
esac
