#!/usr/bin/env bash
# build-android.sh — Build libratex_ffi.so for Android ABIs
#
# Prerequisites:
#   cargo install cargo-ndk
#   rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
#   NDK installed (set ANDROID_NDK_HOME or let cargo-ndk auto-detect)
#
# Output: platforms/android/src/main/jniLibs/{arm64-v8a,armeabi-v7a,x86_64}/libratex_ffi.so

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
JNILIBS="$REPO_ROOT/platforms/android/src/main/jniLibs"

declare -A ABI_MAP=(
    ["aarch64-linux-android"]="arm64-v8a"
    ["armv7-linux-androideabi"]="armeabi-v7a"
    ["x86_64-linux-android"]="x86_64"
)

echo "==> Building ratex-ffi for Android targets..."
for RUST_TARGET in "${!ABI_MAP[@]}"; do
    ABI="${ABI_MAP[$RUST_TARGET]}"
    echo "    → $RUST_TARGET ($ABI)"
    cargo ndk \
        --target "$RUST_TARGET" \
        --manifest-path "$REPO_ROOT/Cargo.toml" \
        build --release -p ratex-ffi

    DEST="$JNILIBS/$ABI"
    mkdir -p "$DEST"
    cp "$REPO_ROOT/target/$RUST_TARGET/release/libratex_ffi.so" "$DEST/"
done

echo "==> Done. Libraries copied to $JNILIBS"
