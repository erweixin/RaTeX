# RaTeX — JVM

Native LaTeX math on JVM (Kotlin + AWT Graphics2D). JAR includes native libraries
for macOS, Linux, and Windows via JNA.

## Out of the box

1. **Add dependency** — In your `build.gradle.kts`:
   ```kotlin
   implementation("io.github.erweixin:ratex-jvm:0.0.15")
   ```
2. **Load fonts & render:**
   ```kotlin
   // Load KaTeX fonts once at startup
   RaTeXFontLoader.loadFromDirectory(File("path/to/katex-fonts"))

   // Parse LaTeX → DisplayList
   val displayList = RaTeXEngine.parseBlocking("""\frac{-b \pm \sqrt{b^2-4ac}}{2a}""")

   // Render to BufferedImage
   val renderer = RaTeXRenderer(displayList, fontSize = 48f) { RaTeXFontLoader.getFont(it) }
   val image = renderer.renderToImage()
   ```

## Local development (building from source)

**Prerequisites:** Rust toolchain.

Build the native library for your host platform:
```bash
bash platforms/jvm/build-jvm.sh
# → native/{darwin-aarch64,linux-x86-64,...}/libratex_ffi.{dylib,so,dll}
```

Build all platforms at once (requires [zig](https://ziglang.org/) + `cargo install cargo-zigbuild`):
```bash
bash platforms/jvm/build-jvm.sh --all
```

The script automatically compiles and copies libraries to `native/` for all supported targets:

| Platform | JNA directory | Library |
|---|---|---|
| macOS ARM64 | `darwin-aarch64/` | `libratex_ffi.dylib` |
| macOS x86_64 | `darwin-x86-64/` | `libratex_ffi.dylib` |
| Linux ARM64 | `linux-aarch64/` | `libratex_ffi.so` |
| Linux x86_64 | `linux-x86-64/` | `libratex_ffi.so` |
| Windows x86_64 | `win32-x86-64/` | `ratex_ffi.dll` |

Include as a local Gradle module in `settings.gradle.kts`:
```kotlin
include(":ratex-jvm")
project(":ratex-jvm").projectDir = file("path/to/RaTeX/platforms/jvm")
```
Then in your app: `implementation(project(":ratex-jvm"))`.

## Usage

### Blocking API

```kotlin
val dl = RaTeXEngine.parseBlocking("""\sum_{i=1}^{n} i = \frac{n(n+1)}{2}""")
val renderer = RaTeXRenderer(dl, fontSize = 48f) { RaTeXFontLoader.getFont(it) }
val image = renderer.renderToImage(padding = 4)
ImageIO.write(image, "png", File("formula.png"))
```

### Coroutine API

```kotlin
val dl = RaTeXEngine.parse("""\int_0^\infty e^{-x}\,dx = 1""")  // suspends on Dispatchers.Default
```

### Drawing to Graphics2D

```kotlin
val renderer = RaTeXRenderer(dl, fontSize = 48f) { RaTeXFontLoader.getFont(it) }

// Query dimensions (pixels)
val width  = renderer.widthPx
val height = renderer.totalHeightPx

// Draw onto any Graphics2D (Swing, PDF, etc.)
renderer.draw(g2)
```

### Font loading

Fonts can be loaded from the filesystem or from classpath resources:

```kotlin
// From a directory
RaTeXFontLoader.loadFromDirectory(File("path/to/katex-fonts"))

// From classpath resources (e.g. /fonts/KaTeX_*.ttf in your JAR)
RaTeXFontLoader.loadFromResources("/fonts")

// Ensure loaded (no-op if already loaded)
RaTeXFontLoader.ensureLoaded(File("path/to/katex-fonts"))
```

## Demo

From repo root:
```bash
bash platforms/jvm/build-jvm.sh
cd demo/jvm
./gradlew run
```

**Troubleshooting:** `UnsatisfiedLinkError` → run `build-jvm.sh` first. JNA not finding the library → set `-Djna.library.path=path/to/native`.

## Publishing (maintainers)

- **Local:** `./gradlew publishToMavenLocal -PskipSigning`
- **Maven Central:** configure [central.sonatype.com](https://central.sonatype.com) + GPG, set `SONATYPE_NEXUS_USERNAME` / `SONATYPE_NEXUS_PASSWORD` / `GPG_PRIVATE_KEY` / `GPG_PASSPHRASE` in gradle.properties.
- **CI:** push tag `v{VERSION}` → `.github/workflows/release-jvm.yml` publishes to Central automatically.
