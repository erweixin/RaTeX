import FlutterMacOS
import Foundation

/// Minimal Flutter plugin registration for ratex_flutter (macOS).
///
/// All rendering happens via Dart FFI (DynamicLibrary.process()) — the symbols
/// from libratex_ffi are loaded via the vendored dylib linked into the process.
/// No method channels or event channels are needed.
public class RaTeXPlugin: NSObject, FlutterPlugin {
    public static func register(with registrar: FlutterPluginRegistrar) {
        // FFI-only plugin: no channels to register.
    }
}
