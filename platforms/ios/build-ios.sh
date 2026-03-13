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
# Both slices must use the same filename (libratex_ffi.a) so CocoaPods
# accepts the xcframework.  Place each in its own temp directory.
SIM_DIR=/tmp/ratex_sim
rm -rf "$SIM_DIR" && mkdir -p "$SIM_DIR"
lipo -create \
    "$REPO_ROOT/target/aarch64-apple-ios-sim/release/libratex_ffi.a" \
    "$REPO_ROOT/target/x86_64-apple-ios/release/libratex_ffi.a" \
    -output "$SIM_DIR/libratex_ffi.a"

echo "==> Packaging XCFramework..."
rm -rf "$OUTPUT"
xcodebuild -create-xcframework \
    -library "$REPO_ROOT/target/aarch64-apple-ios/release/libratex_ffi.a" \
    -headers "$HEADER_DIR" \
    -library "$SIM_DIR/libratex_ffi.a" \
    -headers "$HEADER_DIR" \
    -output "$OUTPUT"

echo "==> Adding module.modulemap to XCFramework headers..."
for HDIR in "$OUTPUT"/*/Headers; do
  cat > "$HDIR/module.modulemap" << 'EOF'
module RaTeXFFI {
    header "ratex.h"
    export *
}
EOF
done

echo "==> Done: $OUTPUT"
