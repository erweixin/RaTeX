// ratex_ffi.dart — Dart FFI bindings to libratex_ffi (iOS, Android, macOS, Windows, Linux).
//
// C ABI:
//   RatexResult ratex_parse_and_layout(const char* latex, const RatexOptions* opts);
//   void        ratex_free_display_list(char* json);
//   const char* ratex_get_last_error(void);

import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

import 'display_list.dart';

// MARK: - C struct mirrors

/// Mirror of `RatexOptions` from ratex.h.
///
/// Always set [structSize] to `sizeOf<RatexOptions>()` before use.
final class NativeRatexColor extends Struct {
  @Float()
  external double r;

  @Float()
  external double g;

  @Float()
  external double b;

  @Float()
  external double a;
}

final class RatexOptions extends Struct {
  /// Must equal `sizeOf<RatexOptions>()`.
  @UintPtr()
  external int structSize;

  /// `0` = inline/text style (`$...$`), `1` = display/block style (`$$...$$`).
  @Int32()
  external int displayMode;

  external Pointer<NativeRatexColor> color;
}

/// Mirror of `RatexResult` from ratex.h.
final class RatexResult extends Struct {
  /// JSON display list on success, null pointer on error.
  external Pointer<Utf8> data;

  /// `0` on success, non-zero on error.
  @Int32()
  external int errorCode;
}

// MARK: - Native function type definitions

typedef _ParseAndLayoutC    = RatexResult Function(Pointer<Utf8>, Pointer<RatexOptions>);
typedef _ParseAndLayoutDart = RatexResult Function(Pointer<Utf8>, Pointer<RatexOptions>);

typedef _FreeDisplayListC    = Void Function(Pointer<Utf8>);
typedef _FreeDisplayListDart = void Function(Pointer<Utf8>);

typedef _GetLastErrorC    = Pointer<Utf8> Function();
typedef _GetLastErrorDart = Pointer<Utf8> Function();

// MARK: - Library loader

DynamicLibrary _openLib() {
  if (Platform.isAndroid) {
    return DynamicLibrary.open('libratex_ffi.so');
  }
  if (Platform.isIOS || Platform.isMacOS) {
    // iOS: static library is force-loaded into the process via CocoaPods
    // macOS: dynamic library is linked via vendored_libraries in the podspec
    return DynamicLibrary.process();
  }
  if (Platform.isWindows) {
    return DynamicLibrary.open('ratex_ffi.dll');
  }
  if (Platform.isLinux) {
    return DynamicLibrary.open('libratex_ffi.so');
  }
  throw UnsupportedError('Unsupported platform: ${Platform.operatingSystem}');
}

// MARK: - FFI bindings (lazy singleton)

class _RaTeXFFI {
  static final _RaTeXFFI _instance = _RaTeXFFI._();
  factory _RaTeXFFI() => _instance;

  _RaTeXFFI._() {
    final lib = _openLib();
    _parseAndLayout  = lib.lookupFunction<_ParseAndLayoutC,    _ParseAndLayoutDart>('ratex_parse_and_layout');
    _freeDisplayList = lib.lookupFunction<_FreeDisplayListC,   _FreeDisplayListDart>('ratex_free_display_list');
    _getLastError    = lib.lookupFunction<_GetLastErrorC,      _GetLastErrorDart>('ratex_get_last_error');
  }

  late final _ParseAndLayoutDart  _parseAndLayout;
  late final _FreeDisplayListDart _freeDisplayList;
  late final _GetLastErrorDart    _getLastError;
}

// MARK: - Public wrapper

/// Exception thrown when RaTeX fails to parse or lay out a formula.
class RaTeXException implements Exception {
  final String message;
  const RaTeXException(this.message);
  @override String toString() => 'RaTeXException: $message';
}

/// Dart FFI wrapper around the RaTeX C ABI.
class RaTeXFfi {
  final _RaTeXFFI _ffi = _RaTeXFFI();

  /// Parse and lay out [latex], returning a [DisplayList].
  ///
  /// [displayMode] controls the rendering style:
  /// - `true` (default) — display/block style, equivalent to `$$...$$`
  /// - `false`          — inline/text style, equivalent to `$...$`
  ///
  /// Throws [RaTeXException] on parse errors.
  DisplayList parseAndLayout(
    String latex, {
    bool displayMode = true,
    RaTeXColor color = const RaTeXColor(0, 0, 0, 1),
  }) {
    final inputPtr = latex.toNativeUtf8();
    final optsPtr = calloc<RatexOptions>();
    final colorPtr = calloc<NativeRatexColor>();
    try {
      optsPtr.ref.structSize = sizeOf<RatexOptions>();
      optsPtr.ref.displayMode = displayMode ? 1 : 0;
      colorPtr.ref.r = color.r;
      colorPtr.ref.g = color.g;
      colorPtr.ref.b = color.b;
      colorPtr.ref.a = color.a;
      optsPtr.ref.color = colorPtr;

      final result = _ffi._parseAndLayout(inputPtr, optsPtr);
      if (result.errorCode != 0) {
        final errPtr = _ffi._getLastError();
        final tail = errPtr.address == 0
            ? 'no message (code ${result.errorCode})'
            : errPtr.toDartString();
        throw RaTeXException(tail);
      }
      if (result.data.address == 0) {
        throw const RaTeXException(
          'native returned success but null data (FFI layout or linking issue)',
        );
      }
      final json = result.data.toDartString();
      _ffi._freeDisplayList(result.data);

      final decoded = jsonDecode(json) as Map<String, dynamic>;
      return DisplayList.fromJson(decoded);
    } finally {
      calloc.free(inputPtr);
      calloc.free(colorPtr);
      calloc.free(optsPtr);
    }
  }
}
