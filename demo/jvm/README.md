# RaTeX JVM demo

Minimal Kotlin sample that depends on the local [`platforms/jvm`](../../platforms/jvm) module via `settings.gradle.kts` (`project(":ratex-jvm").projectDir`).

For API usage, fonts, and **Maven Central** coordinates, see [`platforms/jvm/README.md`](../../platforms/jvm/README.md).

## Prerequisites

- JDK 17+
- Rust toolchain
- Native library for your host OS:

  ```bash
  bash ../../platforms/jvm/build-jvm.sh
  ```

  The demo’s `applicationDefaultJvmArgs` adds `platforms/jvm/native` and `target/release` to `jna.library.path` so `libratex_ffi` can be loaded.

## Run

From this directory:

```bash
./gradlew run
```
