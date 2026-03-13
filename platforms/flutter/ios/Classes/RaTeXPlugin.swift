import Flutter
import UIKit

/// Minimal Flutter plugin registration for ratex_flutter.
///
/// All rendering happens via Dart FFI (DynamicLibrary.process()) — the symbols
/// from libratex_ffi are linked into the app binary through the xcframework.
/// No method channels or event channels are needed.
public class RaTeXPlugin: NSObject, FlutterPlugin {
    public static func register(with registrar: FlutterPluginRegistrar) {
        // FFI-only plugin: no channels to register.
    }
}
