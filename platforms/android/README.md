# RaTeX — Android Integration Guide

Native Android rendering of LaTeX math formulas via Kotlin and Android Canvas.
No WebView, no JavaScript.

---

## How it works

```
LaTeX string
    ↓ nativeParseAndLayout() [JNI → libratex_ffi.so]
JSON DisplayList
    ↓ RaTeXEngine.parse()     [Kotlin JSON decode]
DisplayList
    ↓ RaTeXRenderer.draw()    [android.graphics.Canvas]
Custom View / Compose
```

---

## Prerequisites

| Tool | Version |
|------|---------|
| Android Studio | Hedgehog+ |
| NDK | 26+ |
| Rust | 1.75+ (`rustup`) |
| cargo-ndk | latest (`cargo install cargo-ndk`) |
| minSdk | 24 (Android 7.0) |

Install Rust Android targets once:

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

---

## Build the native library

From the repo root:

```bash
bash platforms/android/build-android.sh
```

This compiles `libratex_ffi.so` for `arm64-v8a`, `armeabi-v7a`, and `x86_64`,
then copies them into `platforms/android/src/main/jniLibs/`.

---

## Add to your project

1. Copy `platforms/android/` into your project as a Gradle module (or include it as a submodule).
2. In your app's `build.gradle.kts`:
   ```kotlin
   dependencies {
       implementation(project(":ratex-android"))
   }
   ```
3. Ensure `jniLibs` are included (done automatically by the module's `build.gradle.kts`).

---

## Usage

### Custom View (XML)

```xml
<io.ratex.RaTeXView
    android:id="@+id/mathView"
    android:layout_width="wrap_content"
    android:layout_height="wrap_content" />
```

```kotlin
binding.mathView.latex    = """\frac{-b \pm \sqrt{b^2-4ac}}{2a}"""
binding.mathView.fontSize = resources.displayMetrics.scaledDensity * 24
binding.mathView.onError  = { e -> Log.e("RaTeX", e.message ?: "error") }
```

### Coroutine (low-level)

```kotlin
lifecycleScope.launch {
    val dl       = RaTeXEngine.parse("""\int_0^\infty e^{-x^2}\,dx""")
    val renderer = RaTeXRenderer(dl, fontSize = 24f * resources.displayMetrics.scaledDensity)
    // renderer.draw(canvas) in your custom View.onDraw
}
```

### Jetpack Compose

```kotlin
@Composable
fun MathFormula(latex: String, fontSize: Float = 24f) {
    var renderer by remember { mutableStateOf<RaTeXRenderer?>(null) }

    LaunchedEffect(latex) {
        runCatching { RaTeXEngine.parse(latex) }
            .onSuccess { renderer = RaTeXRenderer(it, fontSize) }
    }

    renderer?.let { r ->
        Canvas(
            modifier = Modifier.size(r.widthPx.dp, r.totalHeightPx.dp)
        ) { r.draw(drawContext.canvas.nativeCanvas) }
    }
}
```

---

## JNI bridge

`RaTeXEngine` declares `nativeParseAndLayout` and `nativeGetLastError` as
`external` JNI methods. These are implemented in Rust in
`crates/ratex-ffi/src/jni.rs` using the `jni` crate:

```rust
// crates/ratex-ffi/src/jni.rs  (excerpt)
#[no_mangle]
pub extern "system" fn Java_io_ratex_RaTeXEngine_nativeParseAndLayout(
    env: JNIEnv, _class: JClass, latex: JString,
) -> jobject { ... }
```

The JNI function name follows the pattern:
`Java_<package_underscored>_<ClassName>_<methodName>`.

---

## File map

| File | Purpose |
|------|---------|
| `build-android.sh` | Build script → `jniLibs/*.so` |
| `build.gradle.kts` | Android library module config |
| `src/main/kotlin/io/ratex/DisplayList.kt` | kotlinx.serialization types |
| `src/main/kotlin/io/ratex/RaTeXEngine.kt` | JNI wrapper + suspend API |
| `src/main/kotlin/io/ratex/RaTeXRenderer.kt` | Canvas drawing loop |
| `src/main/kotlin/io/ratex/RaTeXView.kt` | Custom View |
