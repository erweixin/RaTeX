#!/usr/bin/env bash
# install-wasm-pack.sh: install a fixed wasm-pack release and verify its checksum.
#
# Usage:
#   ./scripts/install-wasm-pack.sh
#
# Used by the CI jobs that build the WASM package. It downloads the release
# archive for Linux x86_64, checks it against the SHA-256 below and puts the
# binary in ~/.cargo/bin, where the upstream installer puts it too.
#
# To move to another release, change WASM_PACK_VERSION and WASM_PACK_SHA256
# together (sha256sum of the x86_64-unknown-linux-musl archive of that release).

set -euo pipefail

WASM_PACK_VERSION="v0.13.1"
WASM_PACK_SHA256="c539d91ccab2591a7e975bcf82c82e1911b03335c80aa83d67ad25ed2ad06539"

if [ "$(uname -s)-$(uname -m)" != "Linux-x86_64" ]; then
  echo "::error::install-wasm-pack.sh only knows the Linux x86_64 archive, see https://github.com/wasm-bindgen/wasm-pack/releases for other platforms" >&2
  exit 1
fi

ARCHIVE="wasm-pack-${WASM_PACK_VERSION}-x86_64-unknown-linux-musl"
BIN_DIR="${CARGO_HOME:-$HOME/.cargo}/bin"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

curl -sSfL --retry 3 -o "$WORK_DIR/$ARCHIVE.tar.gz" \
  "https://github.com/wasm-bindgen/wasm-pack/releases/download/${WASM_PACK_VERSION}/${ARCHIVE}.tar.gz"
echo "${WASM_PACK_SHA256}  $WORK_DIR/$ARCHIVE.tar.gz" | sha256sum -c -

tar -xzf "$WORK_DIR/$ARCHIVE.tar.gz" -C "$WORK_DIR"
mkdir -p "$BIN_DIR"
install -m 755 "$WORK_DIR/$ARCHIVE/wasm-pack" "$BIN_DIR/wasm-pack"
"$BIN_DIR/wasm-pack" --version
