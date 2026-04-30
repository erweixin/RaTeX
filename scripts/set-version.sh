#!/usr/bin/env bash
# 将根目录 VERSION 文件中的版本号同步到所有平台的版本声明文件：
#   Cargo.toml（含 [workspace.package].version 与以 ratex- 开头的 path 依赖版本，如 ratex-katex-fonts）,
#   platforms/flutter/pubspec.yaml,
#   platforms/flutter/ios/ratex_flutter.podspec,
#   platforms/flutter/android/build.gradle,
#   platforms/flutter/README.md, platforms/flutter/README.zh-CN.md,
#   platforms/android/README.md, platforms/android/README.zh-CN.md,
#   demo/android/README.md（Maven 示例坐标）,
#   platforms/jvm/README.md, platforms/jvm/README.zh-CN.md,
#   demo/flutter/pubspec.yaml,
#   platforms/web/package.json, platforms/react-native/package.json
# platforms/android / platforms/jvm（Maven Central）在未传 -PlibraryVersion 时从本文件读取版本，见各平台 build.gradle.kts。
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

# Flutter macOS podspec
sed "s/s\.version *= *'[^']*'/s.version = '$VER'/" platforms/flutter/macos/ratex_flutter.podspec > platforms/flutter/macos/ratex_flutter.podspec.tmp && mv platforms/flutter/macos/ratex_flutter.podspec.tmp platforms/flutter/macos/ratex_flutter.podspec

# Flutter android/build.gradle（version 'X.X.X'）
sed "s/^version '[^']*'/version '$VER'/" platforms/flutter/android/build.gradle > platforms/flutter/android/build.gradle.tmp && mv platforms/flutter/android/build.gradle.tmp platforms/flutter/android/build.gradle

# Flutter README（ratex_flutter: ^X.X.X in code blocks）
for flutter_readme in platforms/flutter/README.md platforms/flutter/README.zh-CN.md; do
  sed "s/ratex_flutter: \^[0-9][0-9.]*/ratex_flutter: ^$VER/g" "$flutter_readme" > "$flutter_readme.tmp" && mv "$flutter_readme.tmp" "$flutter_readme"
done

# Demo app pubspec（ratex_flutter: ^X.X.X）
sed "s/ratex_flutter: \^[0-9][0-9.]*/ratex_flutter: ^$VER/" demo/flutter/pubspec.yaml > demo/flutter/pubspec.yaml.tmp && mv demo/flutter/pubspec.yaml.tmp demo/flutter/pubspec.yaml

# Android README（Maven artifact version）
for android_readme in platforms/android/README.md platforms/android/README.zh-CN.md; do
  sed "s/ratex-android:[0-9][0-9.]*/ratex-android:$VER/g" "$android_readme" > "$android_readme.tmp" && mv "$android_readme.tmp" "$android_readme"
done

# Demo Android README（published Maven 示例）
sed "s/ratex-android:[0-9][0-9.]*/ratex-android:$VER/g" demo/android/README.md > demo/android/README.md.tmp && mv demo/android/README.md.tmp demo/android/README.md

# JVM README（Maven artifact version，与根目录 VERSION / -PlibraryVersion 一致）
for jvm_readme in platforms/jvm/README.md platforms/jvm/README.zh-CN.md; do
  sed "s/ratex-jvm:[0-9][0-9.]*/ratex-jvm:$VER/g" "$jvm_readme" > "$jvm_readme.tmp" && mv "$jvm_readme.tmp" "$jvm_readme"
done

# npm：Web（ratex-wasm）与 React Native
node -e "
const fs = require('fs');
for (const p of ['platforms/web/package.json', 'platforms/react-native/package.json']) {
  const j = JSON.parse(fs.readFileSync(p, 'utf8'));
  j.version = '$VER';
  fs.writeFileSync(p, JSON.stringify(j, null, 2) + '\n');
}
"

echo "Done. Updated: Cargo.toml (workspace + ratex-* 依赖版本), platforms/flutter/pubspec.yaml, platforms/flutter/ios/ratex_flutter.podspec, platforms/flutter/macos/ratex_flutter.podspec, platforms/flutter/android/build.gradle, platforms/flutter/README.md, platforms/flutter/README.zh-CN.md, platforms/android/README.md, platforms/android/README.zh-CN.md, demo/android/README.md, platforms/jvm/README.md, platforms/jvm/README.zh-CN.md, demo/flutter/pubspec.yaml, platforms/web/package.json, platforms/react-native/package.json; Android/JVM Maven 使用根目录 VERSION。各 Rust 子 crate 使用 version.workspace = true，无需单独改文件。"
