# 发布与统一版本

本仓库通过 **Git 标签 `v*`** 触发多平台发布，并约定**同一 tag 下各包使用同一版本号**。

## 版本落在哪里

| 位置 | 用途 |
|------|------|
| `VERSION` | 单一来源，供脚本和人工对齐用 |
| `Cargo.toml` `[workspace.package].version` | Rust 所有 crate 的版本 |
| `platforms/flutter/pubspec.yaml` `version` | pub.dev 发布 |
| `platforms/web/package.json` `version` | npm 发布 `ratex-wasm` |
| `platforms/react-native/package.json` `version` | npm 发布 `ratex-react-native` |

Android / iOS / JVM 的 Maven 或 Xcode 产物版本在各自 **release workflow** 里由 tag 推导（`-PlibraryVersion`），本地 Gradle 未传参时从根目录 `VERSION` 读取（见 `platforms/android`、`platforms/jvm` 的 `build.gradle.kts`）。

发布 **Flutter** 至 pub.dev 前，建议在 [`platforms/flutter/CHANGELOG.md`](platforms/flutter/CHANGELOG.md) 写入本版本变更摘要（pub.dev 展示用）。

## 发布前：统一版本

1. **改版本（二选一）**
   - 编辑根目录 `VERSION`，写新版本号（如 `0.0.10`），然后执行：
     ```bash
     ./scripts/set-version.sh
     ```
   - 或直接指定版本：
     ```bash
     ./scripts/set-version.sh 0.0.10
     ```
2. **提交**（路径与 [`scripts/set-version.sh`](scripts/set-version.sh) 实际修改的文件一致；若只改部分平台可酌情 `git add` 子集）
   ```bash
   git add VERSION Cargo.toml \
     platforms/flutter/pubspec.yaml \
     platforms/flutter/ios/ratex_flutter.podspec \
     platforms/flutter/android/build.gradle \
     platforms/flutter/README.md platforms/flutter/README.zh-CN.md \
     platforms/android/README.md platforms/android/README.zh-CN.md \
     demo/android/README.md \
     platforms/jvm/README.md platforms/jvm/README.zh-CN.md \
     demo/flutter/pubspec.yaml \
     platforms/web/package.json platforms/react-native/package.json \
     platforms/flutter/CHANGELOG.md
   git commit -m "chore: release v0.0.10"
   ```
3. **打 tag 并推送**
   ```bash
   git tag v0.0.10
   git push origin main --tags
   ```

推送 tag 后会触发：

- **CI** (`ci.yml`) — 仅 main/PR 的 build/test，不依赖 tag
- **Release Cargo** (`release-crates.yml`) — 发布 workspace 内 crate 到 crates.io（需配置 `CARGO_REGISTRY_TOKEN`）
- **Release npm** (`release-npm.yml`) — 发布 `ratex-wasm` 到 npm
- **Release Flutter** (`release-flutter.yml`) — 发布到 pub.dev
- **Release Android** (`release-android.yml`) — 构建 AAR 并发布到 Maven Central
- **Release iOS** (`release-ios.yml`) — 构建 XCFramework、创建 GitHub Release
- **Release JVM** (`release-jvm.yml`) — 构建多平台 native + JAR，发布到 Maven Central，并创建 GitHub Release（附带 JAR）
- **Release React Native** (`release-react-native.yml`) — 发布 `ratex-react-native` 到 npm（含预构建 iOS/Android 原生库）

各 workflow 会校验对应 manifest 的版本是否与 tag 一致，不一致则失败。

## Cargo 首次发布到 crates.io

1. 在 [crates.io](https://crates.io) 登录并生成 API Token。
2. 仓库 Settings → Secrets and variables → Actions 中添加 `CARGO_REGISTRY_TOKEN`，值为该 token。
3. 若某次只想发部分 crate，可编辑 `.github/workflows/release-crates.yml` 中的 `for pkg in ...` 列表。

若**不打算**把 crate 发到 crates.io，可删除或禁用 `.github/workflows/release-crates.yml`，其余发布流程不受影响。
