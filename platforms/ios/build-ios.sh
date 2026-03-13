#!/usr/bin/env bash
# build-ios.sh — Build RaTeX.xcframework for iOS (device + simulator)
#
# Prerequisites:
#   rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
#   Xcode command-line tools installed
#
# Output: platforms/ios/RaTeX.xcframework

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HEADER_DIR="$REPO_ROOT/crates/ratex-ffi/include"
OUTPUT="$REPO_ROOT/platforms/ios/RaTeX.xcframework"

echo "==> Building ratex-ffi for iOS targets..."
cargo build --release -p ratex-ffi --manifest-path "$REPO_ROOT/Cargo.toml" \
    --target aarch64-apple-ios
cargo build --release -p ratex-ffi --manifest-path "$REPO_ROOT/Cargo.toml" \
    --target aarch64-apple-ios-sim
cargo build --release -p ratex-ffi --manifest-path "$REPO_ROOT/Cargo.toml" \
    --target x86_64-apple-ios

echo "==> Creating fat simulator binary..."
lipo -create \
    "$REPO_ROOT/target/aarch64-apple-ios-sim/release/libratex_ffi.a" \
    "$REPO_ROOT/target/x86_64-apple-ios/release/libratex_ffi.a" \
    -output /tmp/libratex_ffi_sim.a

echo "==> Packaging XCFramework..."
rm -rf "$OUTPUT"
xcodebuild -create-xcframework \
    -library "$REPO_ROOT/target/aarch64-apple-ios/release/libratex_ffi.a" \
    -headers "$HEADER_DIR" \
    -library /tmp/libratex_ffi_sim.a \
    -headers "$HEADER_DIR" \
    -output "$OUTPUT"

echo "==> Done: $OUTPUT"
