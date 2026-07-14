#!/usr/bin/env bash

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
APP_DIR_INPUT="${1:-$REPO_ROOT/demo/flutter}"

if [[ ! -d "$APP_DIR_INPUT" ]]; then
  echo "Flutter app directory does not exist: $APP_DIR_INPUT" >&2
  exit 1
fi

APP_DIR="$(cd "$APP_DIR_INPUT" && pwd -P)"
APP_BINARY="$APP_DIR/build/ios/iphoneos/Runner.app/Runner"
GENERATED_PACKAGE="$APP_DIR/ios/Flutter/ephemeral/Packages/FlutterGeneratedPluginSwiftPackage/Package.swift"

# Switching dependency managers makes Flutter rewrite checked-in project and
# lock files. Preserve their exact pre-run contents, including local edits, so
# this verification never leaves the source app dirty.
APP_STATE_FILES=(
  pubspec.lock
  ios/Podfile.lock
  ios/Runner.xcodeproj/project.pbxproj
  ios/Runner.xcodeproj/xcshareddata/xcschemes/Runner.xcscheme
)
APP_STATE_DIR="$(mktemp -d "${TMPDIR:-/tmp}/ratex-flutter-ios-state.XXXXXX")"

for index in "${!APP_STATE_FILES[@]}"; do
  path="$APP_DIR/${APP_STATE_FILES[$index]}"
  if [[ -f "$path" ]]; then
    cp -p "$path" "$APP_STATE_DIR/$index"
  else
    touch "$APP_STATE_DIR/$index.missing"
  fi
done

restore_app_state() {
  local status=$?
  local restore_failed=false
  local index path

  trap - EXIT INT TERM
  for index in "${!APP_STATE_FILES[@]}"; do
    path="$APP_DIR/${APP_STATE_FILES[$index]}"
    if [[ -f "$APP_STATE_DIR/$index.missing" ]]; then
      if ! rm -f "$path"; then
        echo "Failed to remove generated app state: $path" >&2
        restore_failed=true
      fi
    elif ! cp -p "$APP_STATE_DIR/$index" "$path"; then
      echo "Failed to restore app state: $path" >&2
      restore_failed=true
    fi
  done

  if ! rm -rf "$APP_STATE_DIR"; then
    echo "Failed to remove temporary app-state directory: $APP_STATE_DIR" >&2
    restore_failed=true
  fi

  if [[ "$restore_failed" == true && "$status" -eq 0 ]]; then
    status=1
  fi
  exit "$status"
}

trap restore_app_state EXIT
trap 'exit 130' INT
trap 'exit 143' TERM

verify_exported_symbols() {
  local require_swiftpm_anchor="$1"
  local nm_output
  local symbols=(
    ratex_parse_and_layout
    ratex_free_display_list
    ratex_get_last_error
  )

  if [[ ! -f "$APP_BINARY" ]]; then
    echo "Missing iOS app binary: $APP_BINARY" >&2
    exit 1
  fi

  nm_output="$(nm -gU "$APP_BINARY")"
  for symbol in "${symbols[@]}"; do
    if ! grep -Eq "[[:space:]]_${symbol}$" <<<"$nm_output"; then
      echo "Missing exported FFI symbol _$symbol in $APP_BINARY" >&2
      exit 1
    fi
  done

  if [[ "$require_swiftpm_anchor" == true ]] && \
     ! grep -Eq '[[:space:]]_ratex_flutter_linker_anchor$' <<<"$nm_output"; then
    echo "Missing SwiftPM linker anchor in $APP_BINARY" >&2
    exit 1
  fi

  echo "Verified exported RaTeX symbols in $APP_BINARY"
}

cd "$APP_DIR"

echo "==> Verifying CocoaPods release linkage"
flutter clean
FLUTTER_SWIFT_PACKAGE_MANAGER=false flutter pub get
FLUTTER_SWIFT_PACKAGE_MANAGER=false flutter build ios --release --no-codesign
verify_exported_symbols false

echo "==> Verifying SwiftPM release linkage"
flutter clean
FLUTTER_SWIFT_PACKAGE_MANAGER=true flutter pub get
FLUTTER_SWIFT_PACKAGE_MANAGER=true flutter build ios --release --no-codesign

if [[ ! -f "$GENERATED_PACKAGE" ]]; then
  echo "Flutter did not generate its Swift package manifest: $GENERATED_PACKAGE" >&2
  exit 1
fi

if ! grep -Fq '.product(name: "ratex-flutter", package: "ratex_flutter")' \
    "$GENERATED_PACKAGE"; then
  echo "Generated Swift package does not depend on the ratex-flutter product" >&2
  exit 1
fi

verify_exported_symbols true
