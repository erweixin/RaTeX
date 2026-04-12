# RaTeX Android Demo

Native Android app that renders a list of LaTeX formulas using RaTeX (no WebView).

## Prerequisites

| Tool | Version |
|------|---------|
| Android Studio | Hedgehog+ |
| NDK | 26+ |
| Rust | 1.75+ (`rustup`) |
| cargo-ndk | `cargo install cargo-ndk` |
| **Android emulator or physical device** | Required to run the APK |

Install Rust Android targets once (from any directory):

```bash
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
```

## Step 1 — Build the native library

From the **repo root** (not this folder):

```bash
bash platforms/android/build-android.sh
```

This compiles `libratex_ffi.so` and copies it into `platforms/android/src/main/jniLibs/`.  
If you skip this step, the app will fail at runtime with `UnsatisfiedLinkError`.

## Step 2 — Open the project

In Android Studio: **File → Open** → select the `demo/android` folder (this directory).  
Let Gradle sync.

## Step 3 — Run on emulator or device

You need an **Android emulator or a physical device** to run the app.

- **Emulator**: In Android Studio, open **Device Manager** and create an AVD (e.g. arm64-v8a system image). Then choose that device and press **Run (▶)**.
- **Device**: Enable **Developer options** and **USB debugging** on your phone, connect via USB, then select the device and press **Run**.

## Project structure

```
demo/android/
├── settings.gradle.kts    # includes :app and :ratex-android (../../platforms/android)
├── build.gradle.kts       # root build
├── app/
│   ├── build.gradle.kts   # application, depends on :ratex-android
│   └── src/main/
│       ├── AndroidManifest.xml
│       ├── java/io/ratex/demo/MainActivity.kt
│       └── res/            # layout, values, drawable
└── README.md
```

Fonts are **provided by the ratex-android library** (AAR/assets); no need to copy TTF into the demo. The app shows a **custom formula** section (EditText + font size slider + `RaTeXView`) and a **formula examples** list (same formulas as the iOS demo).

## Use the published Maven package (optional)

To depend on the library as a Maven artifact instead of the local project:

1. Publish to Maven Local once (from this directory):
   ```bash
   ./gradlew :ratex-android:publishReleasePublicationToMavenLocal
   ```
2. In `settings.gradle.kts`, add `mavenLocal()` to `repositories`.
3. In `app/build.gradle.kts`, replace `implementation(project(":ratex-android"))` with `implementation("io.github.erweixin:ratex-android:0.1.0")` and remove the `include(":ratex-android")` / `project(":ratex-android").projectDir` from `settings.gradle.kts`.

## Command-line build (optional)

If you have Gradle installed and want to build from the command line, generate the wrapper from this directory:

```bash
gradle wrapper
```

Then:

```bash
./gradlew :app:assembleDebug
./gradlew :app:installDebug   # with device/emulator connected
```
