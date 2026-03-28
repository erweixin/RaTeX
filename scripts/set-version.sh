#!/usr/bin/env bash
# 将根目录 VERSION 文件中的版本号同步到所有平台的版本声明文件：
#   Cargo.toml, platforms/flutter/pubspec.yaml,
#   platforms/flutter/ios/ratex_flutter.podspec,
#   platforms/flutter/android/build.gradle,
#   platforms/flutter/README.md,
#   platforms/android/README.md,
#   demo/flutter/pubspec.yaml,
#   platforms/web/package.json, platforms/react-native/package.json
# platforms/android（Maven Central）在未传 -PlibraryVersion 时从本文件读取版本，见 platforms/android/build.gradle.kts。
# 用法: ./scripts/set-version.sh [版本号]
# 若省略版本号，则使用 VERSION 文件内容。

set -e
cd "$(dirname "$0")/.."

if [ -n "$1" ]; then
  VER="$1"
  echo "$VER" > VERSION
else
  VER=$(cat VERSION | tr -d '[:space:]')
fi

if [ -z "$VER" ]; then
  echo "Usage: $0 [version]" >&2
  echo "  If version is omitted, reads from VERSION file." >&2
  exit 1
fi

echo "Setting version to: $VER"

# Cargo.toml：只改 [workspace.package].version 与以 ratex- 开头的依赖行中的 version，不改 phf/serde 等
sed -e '/^[[:space:]]*version = "/s/version = "[^"]*"/version = "'"$VER"'"/' \
    -e '/^ratex-/s/version = "[^"]*"/version = "'"$VER"'"/g' \
    Cargo.toml > Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml

# Flutter pubspec
sed "s/^version: .*/version: $VER/" platforms/flutter/pubspec.yaml > platforms/flutter/pubspec.yaml.tmp && mv platforms/flutter/pubspec.yaml.tmp platforms/flutter/pubspec.yaml

# Flutter podspec（s.version = 'X.X.X'）
sed "s/s\.version *= *'[^']*'/s.version = '$VER'/" platforms/flutter/ios/ratex_flutter.podspec > platforms/flutter/ios/ratex_flutter.podspec.tmp && mv platforms/flutter/ios/ratex_flutter.podspec.tmp platforms/flutter/ios/ratex_flutter.podspec

# Flutter android/build.gradle（version 'X.X.X'）
sed "s/^version '[^']*'/version '$VER'/" platforms/flutter/android/build.gradle > platforms/flutter/android/build.gradle.tmp && mv platforms/flutter/android/build.gradle.tmp platforms/flutter/android/build.gradle

# Flutter README.md（ratex_flutter: ^X.X.X in code blocks）
sed "s/ratex_flutter: \^[0-9][0-9.]*/ratex_flutter: ^$VER/g" platforms/flutter/README.md > platforms/flutter/README.md.tmp && mv platforms/flutter/README.md.tmp platforms/flutter/README.md

# Demo app pubspec（ratex_flutter: ^X.X.X）
sed "s/ratex_flutter: \^[0-9][0-9.]*/ratex_flutter: ^$VER/" demo/flutter/pubspec.yaml > demo/flutter/pubspec.yaml.tmp && mv demo/flutter/pubspec.yaml.tmp demo/flutter/pubspec.yaml

# Android README（Maven artifact version）
sed "s/ratex-android:[0-9][0-9.]*/ratex-android:$VER/g" platforms/android/README.md > platforms/android/README.md.tmp && mv platforms/android/README.md.tmp platforms/android/README.md

# npm：Web（ratex-wasm）与 React Native
node -e "
const fs = require('fs');
for (const p of ['platforms/web/package.json', 'platforms/react-native/package.json']) {
  const j = JSON.parse(fs.readFileSync(p, 'utf8'));
  j.version = '$VER';
  fs.writeFileSync(p, JSON.stringify(j, null, 2) + '\n');
}
"

echo "Done. Updated: Cargo.toml, platforms/flutter/pubspec.yaml, platforms/flutter/ios/ratex_flutter.podspec, platforms/flutter/android/build.gradle, platforms/flutter/README.md, platforms/android/README.md, demo/flutter/pubspec.yaml, platforms/web/package.json, platforms/react-native/package.json; Android Maven 使用根目录 VERSION"
