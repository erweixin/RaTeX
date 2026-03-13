// display_list.dart — Dart mirror of ratex-types DisplayList / DisplayItem.
// All types use dart:convert JSON decoding (no code generation required).

// MARK: - Top-level output

class DisplayList {
  final double width;
  final double height;
  final double depth;
  final List<DisplayItem> items;

  const DisplayList({
    required this.width,
    required this.height,
    required this.depth,
    required this.items,
  });

  factory DisplayList.fromJson(Map<String, dynamic> json) => DisplayList(
        width:  (json['width']  as num).toDouble(),
        height: (json['height'] as num).toDouble(),
        depth:  (json['depth']  as num).toDouble(),
        items:  (json['items'] as List<dynamic>)
            .map((e) => DisplayItem.fromJson(e as Map<String, dynamic>))
            .toList(),
      );
}

// MARK: - Drawing commands (serde externally-tagged union)

sealed class DisplayItem {
  const DisplayItem();

  factory DisplayItem.fromJson(Map<String, dynamic> json) {
    if (json.containsKey('GlyphPath')) {
      return GlyphPathItem.fromJson(json['GlyphPath'] as Map<String, dynamic>);
    } else if (json.containsKey('Line')) {
      return LineItem.fromJson(json['Line'] as Map<String, dynamic>);
    } else if (json.containsKey('Rect')) {
      return RectItem.fromJson(json['Rect'] as Map<String, dynamic>);
    } else if (json.containsKey('Path')) {
      return PathItem.fromJson(json['Path'] as Map<String, dynamic>);
    }
    throw FormatException('Unknown DisplayItem variant: ${json.keys.first}');
  }
}

class GlyphPathItem extends DisplayItem {
  final double x, y, scale;
  final String font;
  final int charCode;
  final List<PathCommand> commands;
  final RaTeXColor color;

  const GlyphPathItem({
    required this.x, required this.y, required this.scale,
    required this.font, required this.charCode,
    required this.commands, required this.color,
  });

  factory GlyphPathItem.fromJson(Map<String, dynamic> j) => GlyphPathItem(
        x: (j['x'] as num).toDouble(), y: (j['y'] as num).toDouble(),
        scale: (j['scale'] as num).toDouble(),
        font: j['font'] as String,
        charCode: j['char_code'] as int,
        commands: (j['commands'] as List).map((e) => PathCommand.fromJson(e)).toList(),
        color: RaTeXColor.fromJson(j['color'] as Map<String, dynamic>),
      );
}

class LineItem extends DisplayItem {
  final double x, y, width, thickness;
  final RaTeXColor color;

  const LineItem({required this.x, required this.y,
                  required this.width, required this.thickness,
                  required this.color});

  factory LineItem.fromJson(Map<String, dynamic> j) => LineItem(
        x: (j['x'] as num).toDouble(), y: (j['y'] as num).toDouble(),
        width: (j['width'] as num).toDouble(),
        thickness: (j['thickness'] as num).toDouble(),
        color: RaTeXColor.fromJson(j['color'] as Map<String, dynamic>),
      );
}

class RectItem extends DisplayItem {
  final double x, y, width, height;
  final RaTeXColor color;

  const RectItem({required this.x, required this.y,
                  required this.width, required this.height,
                  required this.color});

  factory RectItem.fromJson(Map<String, dynamic> j) => RectItem(
        x: (j['x'] as num).toDouble(), y: (j['y'] as num).toDouble(),
        width: (j['width'] as num).toDouble(),
        height: (j['height'] as num).toDouble(),
        color: RaTeXColor.fromJson(j['color'] as Map<String, dynamic>),
      );
}

class PathItem extends DisplayItem {
  final double x, y;
  final List<PathCommand> commands;
  final bool fill;
  final RaTeXColor color;

  const PathItem({required this.x, required this.y,
                  required this.commands, required this.fill,
                  required this.color});

  factory PathItem.fromJson(Map<String, dynamic> j) => PathItem(
        x: (j['x'] as num).toDouble(), y: (j['y'] as num).toDouble(),
        commands: (j['commands'] as List).map((e) => PathCommand.fromJson(e)).toList(),
        fill: j['fill'] as bool,
        color: RaTeXColor.fromJson(j['color'] as Map<String, dynamic>),
      );
}

// MARK: - Path commands

sealed class PathCommand {
  const PathCommand();

  factory PathCommand.fromJson(Map<String, dynamic> json) {
    if (json.containsKey('MoveTo')) {
      final d = json['MoveTo'] as Map<String, dynamic>;
      return MoveToCmd((d['x'] as num).toDouble(), (d['y'] as num).toDouble());
    } else if (json.containsKey('LineTo')) {
      final d = json['LineTo'] as Map<String, dynamic>;
      return LineToCmd((d['x'] as num).toDouble(), (d['y'] as num).toDouble());
    } else if (json.containsKey('CubicTo')) {
      final d = json['CubicTo'] as Map<String, dynamic>;
      return CubicToCmd(
        (d['x1'] as num).toDouble(), (d['y1'] as num).toDouble(),
        (d['x2'] as num).toDouble(), (d['y2'] as num).toDouble(),
        (d['x']  as num).toDouble(), (d['y']  as num).toDouble(),
      );
    } else if (json.containsKey('QuadTo')) {
      final d = json['QuadTo'] as Map<String, dynamic>;
      return QuadToCmd(
        (d['x1'] as num).toDouble(), (d['y1'] as num).toDouble(),
        (d['x']  as num).toDouble(), (d['y']  as num).toDouble(),
      );
    } else if (json.containsKey('Close')) {
      return const CloseCmd();
    }
    throw FormatException('Unknown PathCommand variant: ${json.keys.first}');
  }
}

class MoveToCmd  extends PathCommand { final double x, y; const MoveToCmd(this.x, this.y); }
class LineToCmd  extends PathCommand { final double x, y; const LineToCmd(this.x, this.y); }
class CubicToCmd extends PathCommand {
  final double x1, y1, x2, y2, x, y;
  const CubicToCmd(this.x1, this.y1, this.x2, this.y2, this.x, this.y);
}
class QuadToCmd extends PathCommand {
  final double x1, y1, x, y;
  const QuadToCmd(this.x1, this.y1, this.x, this.y);
}
class CloseCmd extends PathCommand { const CloseCmd(); }

// MARK: - Color

class RaTeXColor {
  final double r, g, b, a;
  const RaTeXColor(this.r, this.g, this.b, this.a);

  factory RaTeXColor.fromJson(Map<String, dynamic> j) => RaTeXColor(
        (j['r'] as num).toDouble(), (j['g'] as num).toDouble(),
        (j['b'] as num).toDouble(), (j['a'] as num).toDouble());

  /// Convert to a Flutter [Color] (32-bit ARGB int).
  int toFlutterColor() {
    final ai = (a * 255).round().clamp(0, 255);
    final ri = (r * 255).round().clamp(0, 255);
    final gi = (g * 255).round().clamp(0, 255);
    final bi = (b * 255).round().clamp(0, 255);
    return (ai << 24) | (ri << 16) | (gi << 8) | bi;
  }
}
