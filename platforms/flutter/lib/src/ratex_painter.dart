// ratex_painter.dart — Flutter CustomPainter that draws a RaTeX DisplayList.

import 'dart:ui' as ui;
import 'package:flutter/material.dart';

import 'display_list.dart';

/// A [CustomPainter] that renders a pre-parsed [DisplayList].
///
/// Obtain a [DisplayList] from [RaTeXEngine.parse], then pass it to
/// [RaTeXPainter] inside a [CustomPaint] widget or [SizedBox].
class RaTeXPainter extends CustomPainter {
  final DisplayList displayList;

  /// Font size in logical pixels. All em-unit coordinates are multiplied by this.
  final double fontSize;

  const RaTeXPainter({required this.displayList, required this.fontSize});

  // MARK: - Dimensions (logical pixels)

  double get widthPx       => displayList.width  * fontSize;
  double get heightPx      => displayList.height * fontSize;
  double get depthPx       => displayList.depth  * fontSize;
  double get totalHeightPx => heightPx + depthPx;

  // MARK: - Paint

  @override
  void paint(Canvas canvas, Size size) {
    for (final item in displayList.items) {
      switch (item) {
        case GlyphPathItem g: _drawGlyph(canvas, g);
        case LineItem l:      _drawLine(canvas, l);
        case RectItem r:      _drawRect(canvas, r);
        case PathItem p:      _drawPath(canvas, p);
        default: break;
      }
    }
  }

  @override
  bool shouldRepaint(RaTeXPainter oldDelegate) =>
      oldDelegate.displayList != displayList || oldDelegate.fontSize != fontSize;

  // MARK: - Private helpers

  double _em(double val) => val * fontSize;

  Paint _paint(RaTeXColor c, {bool fill = true}) => Paint()
    ..color = Color(c.toFlutterColor())
    ..style = fill ? PaintingStyle.fill : PaintingStyle.stroke
    ..isAntiAlias = true;

  ui.Path _buildPath(List<PathCommand> commands, {double dx = 0, double dy = 0}) {
    final path = ui.Path();
    for (final cmd in commands) {
      switch (cmd) {
        case MoveToCmd c:
          path.moveTo(_em(dx + c.x), _em(dy + c.y));
        case LineToCmd c:
          path.lineTo(_em(dx + c.x), _em(dy + c.y));
        case CubicToCmd c:
          path.cubicTo(
            _em(dx + c.x1), _em(dy + c.y1),
            _em(dx + c.x2), _em(dy + c.y2),
            _em(dx + c.x),  _em(dy + c.y));
        case QuadToCmd c:
          path.conicTo(
            _em(dx + c.x1), _em(dy + c.y1),
            _em(dx + c.x),  _em(dy + c.y),
            1.0); // weight 1 = quadratic Bézier
        case CloseCmd _:
          path.close();
        default: break;
      }
    }
    return path;
  }

  void _drawGlyph(Canvas canvas, GlyphPathItem g) {
    canvas.save();
    canvas.translate(_em(g.x), _em(g.y));
    canvas.scale(g.scale, g.scale);
    final path = _buildPath(g.commands);
    canvas.drawPath(path, _paint(g.color));
    canvas.restore();
  }

  void _drawLine(Canvas canvas, LineItem l) {
    final halfT = _em(l.thickness) / 2;
    canvas.drawRect(
      Rect.fromLTWH(_em(l.x), _em(l.y) - halfT, _em(l.width), _em(l.thickness)),
      _paint(l.color));
  }

  void _drawRect(Canvas canvas, RectItem r) {
    canvas.drawRect(
      Rect.fromLTWH(_em(r.x), _em(r.y), _em(r.width), _em(r.height)),
      _paint(r.color));
  }

  void _drawPath(Canvas canvas, PathItem p) {
    final path = _buildPath(p.commands, dx: p.x, dy: p.y);
    canvas.drawPath(path, _paint(p.color, fill: p.fill));
  }
}
