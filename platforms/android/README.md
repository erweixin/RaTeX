# RaTeX — Android

Native LaTeX math on Android (Kotlin + Canvas). AAR includes KaTeX fonts.
minSdk 21, targetSdk 34.

## Out of the box

1. **Add dependency** — In your app's `build.gradle`:
   ```kotlin
   implementation("io.github.erweixin:ratex-android:0.1.16")
   ```
2. **Use** — Add `RaTeXView` in your layout and set LaTeX in code; fonts load automatically on first render.
   ```kotlin
   binding.mathView.latex = """\frac{-b \pm \sqrt{b^2-4ac}}{2a}"""
   binding.mathView.fontSize = 24f   // dp — no manual density conversion needed
   ```
   **Optional:** To preload fonts at startup, call `RaTeXFontLoader.loadFromAssets(context, "fonts")` in your `Application` or first screen.

## Local development (building from source)

**Prerequisites:** NDK 26+, Python 3, Rust + `cargo install cargo-ndk`, and targets:
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

## 16 KB page compatibility

Source builds explicitly set both `max-page-size=16384` and
`common-page-size=16384` when linking the 64-bit Android `ratex-ffi` library.
This aligns LOAD segments and GNU_RELRO ends, including with NDK r27.
The build script verifies both `arm64-v8a` and `x86_64` and fails on missing
libraries or invalid alignment. This source fix does not change the already
published 0.1.14 AAR, whose RELRO ends are not 16 KB aligned (issue #165).

Verify a release AAR before publishing, or check a downloaded AAR:

```bash
(cd demo/android && ./gradlew :ratex-android:assembleRelease)
python3 scripts/verify_android_elf.py platforms/android/build/outputs/aar/ratex-android-release.aar
# Also accepts a jniLibs directory:
python3 scripts/verify_android_elf.py platforms/android/src/main/jniLibs
python3 -m unittest discover -s scripts -p 'test_verify_android_elf.py'
```

CI checks NDK r27 and r28c builds; Android, React Native, and Flutter releases
verify their Android libraries before publishing. The Maven release also checks
the assembled AAR. Keep RELRO enabled; checking LOAD alignment alone is insufficient.

Applications must separately satisfy APK ZIP alignment (use AGP 8.5.1+ for
uncompressed native libraries; the repository demo currently uses AGP 8.2.2).
Run `zipalign -c -P 16 -v 4 app.apk`, then test library loading and formula
rendering on both 4 KB and 16 KB devices. `adb shell getconf PAGE_SIZE` must
return `16384` on the latter. See the
[Android page-size guide](https://developer.android.com/guide/practices/page-sizes).

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
binding.mathView.color       = android.graphics.Color.parseColor("#1E88E5")
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
import android.graphics.Color

// display mode (default)
val dl = RaTeXEngine.parse(latex)
// inline mode
val dl = RaTeXEngine.parse(latex, displayMode = false)
// custom default color
val blueDl = RaTeXEngine.parse(latex, color = Color.parseColor("#1E88E5"))

val renderer = RaTeXRenderer(dl, fontSize) { RaTeXFontLoader.getTypeface(it) }
renderer.draw(canvas)
```

Explicit LaTeX colors such as `\color{...}` still take precedence over the default color.

## Demo

From repo root: `bash platforms/android/build-android.sh`, then open `demo/android` in Android Studio and run.

**Troubleshooting:** `UnsatisfiedLinkError` → run `build-android.sh`. NDK not found → install NDK 26+ or set `ANDROID_NDK_HOME`.

## Publishing (maintainers)

- **Local:** `./gradlew :ratex-android:publishReleasePublicationToMavenLocal`
- **Maven Central:** configure [central.sonatype.com](https://central.sonatype.com) + GPG, set `SONATYPE_NEXUS_USERNAME` / `SONATYPE_NEXUS_PASSWORD` / `GPG_PRIVATE_KEY` / `GPG_PASSPHRASE` in gradle.properties.
- **CI:** push tag `v{VERSION}` → `.github/workflows/release-android.yml` publishes to Central automatically.
