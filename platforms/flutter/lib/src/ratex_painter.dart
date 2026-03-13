// ratex_painter.dart — Flutter CustomPainter that draws a RaTeX DisplayList.
//
// GlyphPath items carry a font ID (e.g. "Math-Italic") and a Unicode char_code.
// The path commands inside GlyphPath are PLACEHOLDER bounding boxes — they are
// not real glyph outlines.  Glyphs must be drawn with the bundled KaTeX fonts
// via dart:ui ParagraphBuilder, mirroring how the web renderer uses
// ctx.fillText() with KaTeX CSS font families.

import 'dart:math' as math;
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
      }
    }
  }

  @override
  bool shouldRepaint(RaTeXPainter oldDelegate) =>
      oldDelegate.displayList != displayList || oldDelegate.fontSize != fontSize;

  // MARK: - Private helpers

  double _em(double val) => val * fontSize;

  Color _color(RaTeXColor c) => Color(c.toFlutterColor());

  Paint _paint(RaTeXColor c, {bool fill = true}) => Paint()
    ..color = _color(c)
    ..style = fill ? PaintingStyle.fill : PaintingStyle.stroke
    ..strokeWidth = 1.0   // used only for stroke paths (radical surd, angle brackets)
    ..isAntiAlias = true;

  // MARK: Glyph — drawn via dart:ui ParagraphBuilder with KaTeX fonts.
  //
  // The font ID from the DisplayList (e.g. "Math-Italic") maps to a
  // Flutter font family ("KaTeX_Math") plus weight/style attributes,
  // mirroring the web renderer's fontIdToCss() function.

  static ({String family, FontWeight weight, FontStyle style}) _parseFontId(
      String fontId) {
    // fontId examples: "Math-Italic", "Main-Bold", "Main-BoldItalic",
    //                  "AMS-Regular", "Size1-Regular"
    // Family prefix is everything before the first '-'.
    final dash = fontId.indexOf('-');
    final prefix = dash >= 0 ? fontId.substring(0, dash) : fontId;
    final suffix = dash >= 0 ? fontId.substring(dash + 1) : 'Regular';

    final family = 'KaTeX_$prefix';
    final weight = suffix.contains('Bold') ? FontWeight.bold : FontWeight.normal;
    final style  = suffix.contains('Italic') ? FontStyle.italic : FontStyle.normal;

    return (family: family, weight: weight, style: style);
  }

  void _drawGlyph(Canvas canvas, GlyphPathItem g) {
    // GlyphPath.commands are placeholder bounding boxes — ignore them.
    // Draw the actual glyph using the bundled KaTeX font via ParagraphBuilder.
    final (:family, :weight, :style) = _parseFontId(g.font);
    final sizePx = _em(g.scale);

    final pb = ui.ParagraphBuilder(ui.ParagraphStyle(
      fontFamily: family,
      fontWeight: weight,
      fontStyle:  style,
      fontSize:   sizePx,
      textAlign:  TextAlign.left,
    ))
      ..pushStyle(ui.TextStyle(
        color:      _color(g.color),
        fontFamily: family,
        fontWeight: weight,
        fontStyle:  style,
        fontSize:   sizePx,
      ))
      ..addText(String.fromCharCode(g.charCode));

    final paragraph = pb.build()
      ..layout(const ui.ParagraphConstraints(width: double.infinity));

    // g.y is the alphabetic baseline measured downward from the top of the
    // bounding box (same coordinate convention as the web canvas renderer
    // with textBaseline='alphabetic').
    // drawParagraph places the top-left of the paragraph box at the given
    // offset, so subtract the paragraph's alphabeticBaseline to align.
    canvas.drawParagraph(
      paragraph,
      Offset(_em(g.x), _em(g.y) - paragraph.alphabeticBaseline),
    );
  }

  // MARK: Line / Rect / Path

  void _drawLine(Canvas canvas, LineItem l) {
    final t = math.max(0.5, _em(l.thickness));
    final halfT = t / 2;
    canvas.drawRect(
      Rect.fromLTWH(_em(l.x), _em(l.y) - halfT, _em(l.width), t),
      _paint(l.color));
  }

  void _drawRect(Canvas canvas, RectItem r) {
    canvas.drawRect(
      Rect.fromLTWH(_em(r.x), _em(r.y), _em(r.width), _em(r.height)),
      _paint(r.color));
  }

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
      }
    }
    return path;
  }

  void _drawPath(Canvas canvas, PathItem p) {
    final path = _buildPath(p.commands, dx: p.x, dy: p.y);
    canvas.drawPath(path, _paint(p.color, fill: p.fill));
  }
}
