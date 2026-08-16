#!/usr/bin/env bash
# 将根目录 VERSION 文件中的版本号同步到所有平台的版本声明文件：
#   Cargo.toml（含 [workspace.package].version 与以 ratex- 开头的 path 依赖版本，如 ratex-katex-fonts / ratex-font-loader）,
#   platforms/flutter/pubspec.yaml,
#   platforms/flutter/ios/ratex_flutter.podspec,
#   platforms/flutter/android/build.gradle,
#   platforms/flutter/README.md, platforms/flutter/README.zh-CN.md,
#   platforms/android/README.md, platforms/android/README.zh-CN.md,
#   demo/android/README.md（Maven 示例坐标）,
#   platforms/jvm/README.md, platforms/jvm/README.zh-CN.md,
#   demo/flutter/pubspec.yaml,
#   platforms/web/package.json, platforms/react-native/package.json,
#   CHANGELOG.md（把首个 "## [Unreleased]" 段改名为 "## [新版本] - 当天日期"，并在其上方新建空的 "## [Unreleased]" 段）
# CHANGELOG 维护依赖 python3。
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

# Cargo.toml：只改 [workspace.package].version 与以 ratex- 开头的依赖行中的 version，
# 覆盖新增的 workspace 内部 crate（例如 ratex-font-loader），不改 phf/serde 等
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

# CHANGELOG.md：把 "## [Unreleased]" 段改名为 "## [新版本] - 日期"，并在其上方新建空的 "## [Unreleased]" 段。
# 幂等：若该版本段已存在则只保证存在新的 Unreleased 段；文件不存在或缺少 Unreleased 段时不报错。
TODAY=$(date +%Y-%m-%d)
CHANGELOG_STATUS=$(python3 - "$VER" "$TODAY" <<'PY'
import re
import sys

ver, today = sys.argv[1], sys.argv[2]
path = 'CHANGELOG.md'
try:
    text = open(path, encoding='utf-8').read()
except FileNotFoundError:
    print('Note: CHANGELOG.md not found; skipping changelog update.')
    sys.exit(0)

lines = text.splitlines(keepends=True)
unreleased_re = re.compile(r'^## \[Unreleased\]\s*$')
heading_re = re.compile(r'^## \[')
version_re = re.compile(r'^## \[' + re.escape(ver) + r'\](?:\s|$)')

# 1) 版本段不存在时，把第一个 "## [Unreleased]" 改名为 "## [ver] - date"
renamed = False
if not any(version_re.match(l.strip()) for l in lines):
    for i, l in enumerate(lines):
        if unreleased_re.match(l.strip()):
            lines[i] = f'## [{ver}] - {today}\n'
            renamed = True
            break
    else:
        print('Note: no "## [Unreleased]" section in CHANGELOG.md; skipping rename.')

# 2) 确保文件顶部（第一个版本段之前）存在一个空的 "## [Unreleased]" 段
inserted = False
if not any(unreleased_re.match(l.strip()) for l in lines):
    first = next((i for i, l in enumerate(lines) if heading_re.match(l.strip())), None)
    if first is not None:
        lines.insert(first, '## [Unreleased]\n\n')
    else:
        if lines and not lines[-1].endswith('\n'):
            lines.append('\n')
        lines.append('## [Unreleased]\n\n')
    inserted = True

if renamed or inserted:
    open(path, 'w', encoding='utf-8').write(''.join(lines))
    if renamed:
        print(f'CHANGELOG.md: released [Unreleased] as [{ver}] - {today}')
    else:
        print('CHANGELOG.md: added an empty [Unreleased] section.')
else:
    print(f'CHANGELOG.md: unchanged (version section [{ver}] already present).')
PY
)
printf '%s\n' "$CHANGELOG_STATUS"
CHANGELOG_SUMMARY=$(printf '%s\n' "$CHANGELOG_STATUS" | tail -n 1 | sed 's/^CHANGELOG\.md: //')

echo "Done. Updated: Cargo.toml (workspace + ratex-* 依赖版本), platforms/flutter/pubspec.yaml, platforms/flutter/ios/ratex_flutter.podspec, platforms/flutter/macos/ratex_flutter.podspec, platforms/flutter/android/build.gradle, platforms/flutter/README.md, platforms/flutter/README.zh-CN.md, platforms/android/README.md, platforms/android/README.zh-CN.md, demo/android/README.md, platforms/jvm/README.md, platforms/jvm/README.zh-CN.md, demo/flutter/pubspec.yaml, platforms/web/package.json, platforms/react-native/package.json; CHANGELOG.md: ${CHANGELOG_SUMMARY:-未修改}。Android/JVM Maven 使用根目录 VERSION。各 Rust 子 crate 使用 version.workspace = true，无需单独改文件。"
