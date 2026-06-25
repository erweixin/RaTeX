#!/usr/bin/env bash
# build-ios.sh — Compatibility wrapper for iOS-only XCFramework build.
#
# This script intentionally delegates to the unified Apple builder to avoid
# logic drift across multiple build scripts.
#
# IMPORTANT:
# - iOS-only artifacts:    use this script (or build-apple-xcframework.sh --ios)
# - SPM/macOS consumers:   must use scripts/build-apple-xcframework.sh (default)
#                          so RaTeX.xcframework includes macOS slices.

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
exec bash "$REPO_ROOT/scripts/build-apple-xcframework.sh" --ios
