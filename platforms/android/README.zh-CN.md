# RaTeX — Android

Android 上原生渲染 LaTeX 数学公式（Kotlin + Canvas），AAR 内含 KaTeX 字体。
minSdk 21，targetSdk 34。

## 开箱即用

1. **添加依赖** — 在 app 的 `build.gradle` 中：
   ```kotlin
   implementation("io.github.erweixin:ratex-android:0.1.0")
   ```
2. **使用** — 布局里放 `RaTeXView`，代码中设置 LaTeX 与字号；字体会在首次渲染时自动加载，无需手动调用。
   ```kotlin
   binding.mathView.latex = """\frac{-b \pm \sqrt{b^2-4ac}}{2a}"""
   binding.mathView.fontSize = 24f   // dp — 无需手动换算密度
   ```
   **可选**：若希望启动时提前加载，可在 Application 或首屏调用 `RaTeXFontLoader.loadFromAssets(context, "fonts")`。

## 本地开发（从源码构建）

**环境要求**：NDK 26+、Rust，执行 `cargo install cargo-ndk` 并安装目标：
```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

在仓库根目录执行：
```bash
bash platforms/android/build-android.sh
# → src/main/jniLibs/{arm64-v8a,armeabi-v7a,x86_64}/libratex_ffi.so
```

在 `settings.gradle` 中将本目录作为本地模块引入：
```kotlin
include(":ratex-android")
project(":ratex-android").projectDir = file("path/to/RaTeX/platforms/android")
```
然后在 app 中：`implementation(project(":ratex-android"))`。

## 使用

### 块级公式 — `RaTeXView`

```xml
<io.ratex.RaTeXView android:id="@+id/mathView"
    android:layout_width="wrap_content" android:layout_height="wrap_content" />
```

```kotlin
binding.mathView.latex       = """\frac{-b \pm \sqrt{b^2-4ac}}{2a}"""
binding.mathView.fontSize    = 24f     // dp — 无需手动换算密度
binding.mathView.displayMode = true    // true = 独立块（默认）；false = 行内
```

### 行内公式 — `RaTeXSpan`

`RaTeXSpan` 是一个 `ReplacementSpan`，可将 LaTeX 公式内嵌到普通文字中，基线自动对齐，行高自动扩展。

渲染为异步，需在协程中调用 `RaTeXSpan.create`：

```kotlin
private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())

fun showInlineFormula(textView: TextView) {
    scope.launch {
        val span = RaTeXSpan.create(
            context  = this@MainActivity,
            latex    = """\frac{1+\sqrt{5}}{2}""",
            fontSize = 18f   // dp
        )
        val ssb = SpannableStringBuilder("黄金比例 φ = ")
        val start = ssb.length
        ssb.append("\u200B")
        ssb.setSpan(span, start, ssb.length, 0)
        ssb.append(" ≈ 1.618")
        textView.text = ssb
    }
}
```

| 参数 | 类型 | 说明 |
|------|------|------|
| `context` | `Context` | 用于字体加载时访问 Assets。 |
| `latex` | `String` | LaTeX 数学字符串（不含外层 `$` 或 `\[…\]`）。 |
| `fontSize` | `Float` | 字体大小（dp），内部自动转换为 px。 |

若公式无法解析，抛出 `RaTeXException`。

### 底层（Compose / 自定义绘制）

```kotlin
// 独立块（默认）
val dl = RaTeXEngine.parse(latex)
// 行内
val dl = RaTeXEngine.parse(latex, displayMode = false)

val renderer = RaTeXRenderer(dl, fontSize) { RaTeXFontLoader.getTypeface(it) }
renderer.draw(canvas)
```

## Demo

在仓库根目录执行 `bash platforms/android/build-android.sh`，用 Android Studio 打开 `demo/android` 运行即可。

**常见问题：** `UnsatisfiedLinkError` → 先执行 `build-android.sh`。NDK 未找到 → 安装 NDK 26+ 或设置 `ANDROID_NDK_HOME`。

## 发布（维护者）

- **本地：** `./gradlew :ratex-android:publishReleasePublicationToMavenLocal`
- **Maven Central：** 配置 [central.sonatype.com](https://central.sonatype.com) 与 GPG，在 gradle.properties 中设置 `SONATYPE_NEXUS_USERNAME` / `SONATYPE_NEXUS_PASSWORD` / `GPG_PRIVATE_KEY` / `GPG_PASSPHRASE`。
- **CI：** 推送 tag（如 `v{VERSION}`）会触发 `.github/workflows/release-android.yml` 自动发布到 Central。
