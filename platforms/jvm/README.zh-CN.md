# RaTeX — JVM

JVM 上原生渲染 LaTeX 数学公式（Kotlin + AWT Graphics2D），JAR 内含 macOS、Linux、Windows 的原生库，通过 JNA 自动加载。

## 开箱即用

1. **添加依赖** — 在 `build.gradle.kts` 中：
   ```kotlin
   implementation("io.github.erweixin:ratex-jvm:0.0.15")
   ```
2. **加载字体并渲染：**
   ```kotlin
   // 启动时加载一次 KaTeX 字体
   RaTeXFontLoader.loadFromDirectory(File("path/to/katex-fonts"))

   // 解析 LaTeX → DisplayList
   val displayList = RaTeXEngine.parseBlocking("""\frac{-b \pm \sqrt{b^2-4ac}}{2a}""")

   // 渲染为 BufferedImage
   val renderer = RaTeXRenderer(displayList, fontSize = 48f) { RaTeXFontLoader.getFont(it) }
   val image = renderer.renderToImage()
   ```

## 本地开发（从源码构建）

**环境要求：** Rust 工具链。

编译当前平台的原生库：
```bash
bash platforms/jvm/build-jvm.sh
# → native/{darwin-aarch64,linux-x86-64,...}/libratex_ffi.{dylib,so,dll}
```

一次编译所有平台（需要 [zig](https://ziglang.org/) + `cargo install cargo-zigbuild`）：
```bash
bash platforms/jvm/build-jvm.sh --all
```

脚本会自动编译并复制库文件到 `native/` 目录，支持以下平台：

| 平台 | JNA 目录 | 库文件 |
|---|---|---|
| macOS ARM64 | `darwin-aarch64/` | `libratex_ffi.dylib` |
| macOS x86_64 | `darwin-x86-64/` | `libratex_ffi.dylib` |
| Linux ARM64 | `linux-aarch64/` | `libratex_ffi.so` |
| Linux x86_64 | `linux-x86-64/` | `libratex_ffi.so` |
| Windows x86_64 | `win32-x86-64/` | `ratex_ffi.dll` |

在 `settings.gradle.kts` 中将本目录作为本地模块引入：
```kotlin
include(":ratex-jvm")
project(":ratex-jvm").projectDir = file("path/to/RaTeX/platforms/jvm")
```
然后在 app 中：`implementation(project(":ratex-jvm"))`。

## 使用

### 阻塞 API

```kotlin
val dl = RaTeXEngine.parseBlocking("""\sum_{i=1}^{n} i = \frac{n(n+1)}{2}""")
val renderer = RaTeXRenderer(dl, fontSize = 48f) { RaTeXFontLoader.getFont(it) }
val image = renderer.renderToImage(padding = 4)
ImageIO.write(image, "png", File("formula.png"))
```

### 协程 API

```kotlin
val dl = RaTeXEngine.parse("""\int_0^\infty e^{-x}\,dx = 1""")  // 在 Dispatchers.Default 上挂起
```

### 绘制到 Graphics2D

```kotlin
val renderer = RaTeXRenderer(dl, fontSize = 48f) { RaTeXFontLoader.getFont(it) }

// 查询尺寸（像素）
val width  = renderer.widthPx
val height = renderer.totalHeightPx

// 绘制到任意 Graphics2D（Swing、PDF 等）
renderer.draw(g2)
```

### 字体加载

支持从文件系统或 classpath 资源加载字体：

```kotlin
// 从目录加载
RaTeXFontLoader.loadFromDirectory(File("path/to/katex-fonts"))

// 从 classpath 资源加载（如 JAR 中的 /fonts/KaTeX_*.ttf）
RaTeXFontLoader.loadFromResources("/fonts")

// 确保已加载（重复调用无副作用）
RaTeXFontLoader.ensureLoaded(File("path/to/katex-fonts"))
```

## Demo

在仓库根目录执行：
```bash
bash platforms/jvm/build-jvm.sh
cd demo/jvm
./gradlew run
```

**常见问题：** `UnsatisfiedLinkError` → 先执行 `build-jvm.sh`。JNA 找不到库 → 设置 `-Djna.library.path=path/to/native`。

## 发布（维护者）

- **本地：** `./gradlew publishToMavenLocal -PskipSigning`
- **Maven Central：** 配置 [central.sonatype.com](https://central.sonatype.com) 与 GPG，在 gradle.properties 中设置 `SONATYPE_NEXUS_USERNAME` / `SONATYPE_NEXUS_PASSWORD` / `GPG_PRIVATE_KEY` / `GPG_PASSPHRASE`。
- **CI：** 推送 tag（如 `v{VERSION}`）会触发 `.github/workflows/release-jvm.yml` 自动发布到 Central。
