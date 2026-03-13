// ratex_ffi.dart — Dart FFI bindings to libratex_ffi (iOS static / Android .so).
//
// The three C functions exposed are:
//   const char* ratex_parse_and_layout(const char* latex);
//   void        ratex_free_display_list(char* json);
//   const char* ratex_get_last_error(void);

import 'dart:convert';
import 'dart:ffi';
import 'dart:io';

import 'package:ffi/ffi.dart';

import 'display_list.dart';

// MARK: - Native function type definitions

typedef _ParseAndLayoutC    = Pointer<Utf8> Function(Pointer<Utf8>);
typedef _ParseAndLayoutDart = Pointer<Utf8> Function(Pointer<Utf8>);

typedef _FreeDisplayListC    = Void Function(Pointer<Utf8>);
typedef _FreeDisplayListDart = void Function(Pointer<Utf8>);

typedef _GetLastErrorC    = Pointer<Utf8> Function();
typedef _GetLastErrorDart = Pointer<Utf8> Function();

// MARK: - Library loader

DynamicLibrary _openLib() {
  if (Platform.isAndroid) {
    return DynamicLibrary.open('libratex_ffi.so');
  }
  // iOS: the static library is linked into the process
  return DynamicLibrary.process();
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
  /// Throws [RaTeXException] on parse errors.
  DisplayList parseAndLayout(String latex) {
    final inputPtr = latex.toNativeUtf8();
    try {
      final resultPtr = _ffi._parseAndLayout(inputPtr);
      if (resultPtr.address == 0) {
        final errPtr = _ffi._getLastError();
        final msg = errPtr.address == 0
            ? 'unknown error'
            : errPtr.toDartString();
        throw RaTeXException(msg);
      }
      final json = resultPtr.toDartString();
      _ffi._freeDisplayList(resultPtr);

      final decoded = jsonDecode(json) as Map<String, dynamic>;
      return DisplayList.fromJson(decoded);
    } finally {
      calloc.free(inputPtr);
    }
  }
}
