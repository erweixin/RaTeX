# RaTeX — Android

Native LaTeX math on Android (Kotlin + Canvas). AAR includes KaTeX fonts.
minSdk 21, targetSdk 34.

## Out of the box

1. **Add dependency** — In your app's `build.gradle`:
   ```kotlin
   implementation("io.github.erweixin:ratex-android:0.1.0")
   ```
2. **Use** — Add `RaTeXView` in your layout and set LaTeX in code; fonts load automatically on first render.
   ```kotlin
   binding.mathView.latex = """\frac{-b \pm \sqrt{b^2-4ac}}{2a}"""
   binding.mathView.fontSize = 24f   // dp — no manual density conversion needed
   ```
   **Optional:** To preload fonts at startup, call `RaTeXFontLoader.loadFromAssets(context, "fonts")` in your `Application` or first screen.

## Local development (building from source)

**Prerequisites:** NDK 26+, Rust + `cargo install cargo-ndk`, and targets:
```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

Build from repo root:
```bash
bash platforms/android/build-android.sh
# → src/main/jniLibs/{arm64-v8a,armeabi-v7a,x86_64}/libratex_ffi.so
```

Include as a local module in `settings.gradle`:
```kotlin
include(":ratex-android")
project(":ratex-android").projectDir = file("path/to/RaTeX/platforms/android")
```
Then in your app: `implementation(project(":ratex-android"))`.

## Usage

### Block formula — `RaTeXView`

```xml
<io.ratex.RaTeXView android:id="@+id/mathView"
    android:layout_width="wrap_content" android:layout_height="wrap_content" />
```

```kotlin
binding.mathView.latex       = """\frac{-b \pm \sqrt{b^2-4ac}}{2a}"""
binding.mathView.fontSize    = 24f     // dp — no manual density conversion needed
binding.mathView.displayMode = true    // true = display/block (default); false = inline/text
```

### Inline formula — `RaTeXSpan`

`RaTeXSpan` is a `ReplacementSpan` that renders a LaTeX formula inline with surrounding text, baseline-aligned, with line height expanding automatically.

Rendering is async. Call `RaTeXSpan.create` from a coroutine:

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

| Parameter | Type | Description |
|-----------|------|-------------|
| `context` | `Context` | Used for asset access during font loading. |
| `latex` | `String` | LaTeX math-mode string (no surrounding `$` or `\[…\]`). |
| `fontSize` | `Float` | Font size in dp. Converted to px internally. |

**Throws** `RaTeXException` if the formula cannot be parsed.

### Low-level (Compose / custom drawing)

```kotlin
// display mode (default)
val dl = RaTeXEngine.parse(latex)
// inline mode
val dl = RaTeXEngine.parse(latex, displayMode = false)

val renderer = RaTeXRenderer(dl, fontSize) { RaTeXFontLoader.getTypeface(it) }
renderer.draw(canvas)
```

## Demo

From repo root: `bash platforms/android/build-android.sh`, then open `demo/android` in Android Studio and run.

**Troubleshooting:** `UnsatisfiedLinkError` → run `build-android.sh`. NDK not found → install NDK 26+ or set `ANDROID_NDK_HOME`.

## Publishing (maintainers)

- **Local:** `./gradlew :ratex-android:publishReleasePublicationToMavenLocal`
- **Maven Central:** configure [central.sonatype.com](https://central.sonatype.com) + GPG, set `SONATYPE_NEXUS_USERNAME` / `SONATYPE_NEXUS_PASSWORD` / `GPG_PRIVATE_KEY` / `GPG_PASSPHRASE` in gradle.properties.
- **CI:** push tag `v{VERSION}` → `.github/workflows/release-android.yml` publishes to Central automatically.
