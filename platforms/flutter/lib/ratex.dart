// ratex.dart — Public API for the ratex_flutter package.
//
// Usage:
//   import 'package:ratex_flutter/ratex.dart';
//
//   Widget build(BuildContext context) => RaTeXWidget(
//     latex: r'\frac{-b \pm \sqrt{b^2-4ac}}{2a}',
//     fontSize: 24,
//   );

library ratex_flutter;

import 'package:flutter/material.dart';

import 'src/display_list.dart';
import 'src/ratex_ffi.dart';
import 'src/ratex_painter.dart';

export 'src/display_list.dart';
export 'src/ratex_ffi.dart' show RaTeXException;

// MARK: - Engine

/// High-level entry point for RaTeX rendering.
class RaTeXEngine {
  static final RaTeXEngine instance = RaTeXEngine._();
  RaTeXEngine._();

  final _ffi = RaTeXFfi();

  /// Parse and lay out [latex], returning a [DisplayList].
  ///
  /// This is a synchronous, CPU-bound call. For long formulas, wrap in an
  /// isolate:
  /// ```dart
  /// final dl = await compute(RaTeXEngine.instance.parseAndLayout, latex);
  /// ```
  DisplayList parseAndLayout(String latex) => _ffi.parseAndLayout(latex);
}

// MARK: - Stateful widget

/// A Flutter widget that renders a LaTeX math formula natively.
///
/// ```dart
/// RaTeXWidget(
///   latex: r'\int_0^\infty e^{-x^2}\,dx = \frac{\sqrt{\pi}}{2}',
///   fontSize: 24,
/// )
/// ```
class RaTeXWidget extends StatefulWidget {
  /// The LaTeX math-mode string to render.
  final String latex;

  /// Font size in logical pixels.
  final double fontSize;

  /// Widget displayed while the formula is being computed.
  final Widget? loading;

  /// Called when a render error occurs.
  final void Function(RaTeXException)? onError;

  const RaTeXWidget({
    super.key,
    required this.latex,
    this.fontSize = 24,
    this.loading,
    this.onError,
  });

  @override
  State<RaTeXWidget> createState() => _RaTeXWidgetState();
}

class _RaTeXWidgetState extends State<RaTeXWidget> {
  DisplayList? _displayList;
  RaTeXException? _error;

  @override
  void initState() {
    super.initState();
    _render(widget.latex);
  }

  @override
  void didUpdateWidget(RaTeXWidget old) {
    super.didUpdateWidget(old);
    if (old.latex != widget.latex || old.fontSize != widget.fontSize) {
      _render(widget.latex);
    }
  }

  void _render(String latex) {
    try {
      final dl = RaTeXEngine.instance.parseAndLayout(latex);
      if (mounted) setState(() { _displayList = dl; _error = null; });
    } on RaTeXException catch (e) {
      widget.onError?.call(e);
      if (mounted) setState(() { _error = e; });
    }
  }

  @override
  Widget build(BuildContext context) {
    if (_error != null) {
      return Text('RaTeX error: ${_error!.message}',
          style: const TextStyle(color: Colors.red, fontSize: 12));
    }
    final dl = _displayList;
    if (dl == null) {
      return widget.loading ?? const SizedBox.shrink();
    }
    final painter = RaTeXPainter(displayList: dl, fontSize: widget.fontSize);
    return SizedBox(
      width:  painter.widthPx,
      height: painter.totalHeightPx,
      child:  CustomPaint(painter: painter),
    );
  }
}
