// RaTeXPlugin.kt — Flutter plugin registration for ratex_flutter (Android).
//
// All rendering is done via Dart FFI: DynamicLibrary.open("libratex_ffi.so").
// The .so is packaged from android/jniLibs (see build.gradle sourceSets;
// run platforms/android/build-android.sh to build libratex_ffi.so).
// No method channels or event channels.

package io.ratex

import io.flutter.embedding.engine.plugins.FlutterPlugin

/** Minimal Flutter plugin for RaTeX — no platform channels. */
class RaTeXPlugin : FlutterPlugin {

    override fun onAttachedToEngine(binding: FlutterPlugin.FlutterPluginBinding) {
        // FFI-only: Dart loads libratex_ffi.so via DynamicLibrary.open.
        // No registration of method channels.
    }

    override fun onDetachedFromEngine(binding: FlutterPlugin.FlutterPluginBinding) {}
}
