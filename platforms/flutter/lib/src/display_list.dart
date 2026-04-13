// display_list.dart — Dart mirror of ratex-types DisplayList / DisplayItem
//
// JSON uses serde internally-tagged format:
//   DisplayItem: {"type": "GlyphPath", "x": ..., ...}  (flat)
//   PathCommand: {"type": "MoveTo",    "x": ..., "y": ...}  (flat)

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

// MARK: - Drawing commands (internally tagged: {"type": "GlyphPath", ...})

sealed class DisplayItem {
  const DisplayItem();

  factory DisplayItem.fromJson(Map<String, dynamic> json) {
    final type = json['type'] as String;
    switch (type) {
      case 'GlyphPath': return GlyphPathItem.fromJson(json);
      case 'Line':      return LineItem.fromJson(json);
      case 'Rect':      return RectItem.fromJson(json);
      case 'Path':      return PathItem.fromJson(json);
      default: throw FormatException('Unknown DisplayItem type: $type');
    }
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
        commands: (j['commands'] as List?)
                ?.map((e) => PathCommand.fromJson(e as Map<String, dynamic>))
                .toList() ??
            [],
        color: RaTeXColor.fromJson(j['color'] as Map<String, dynamic>),
      );
}

class LineItem extends DisplayItem {
  final double x, y, width, thickness;
  final RaTeXColor color;
  final bool dashed;

  const LineItem({required this.x, required this.y,
                  required this.width, required this.thickness,
                  required this.color, this.dashed = false});

  factory LineItem.fromJson(Map<String, dynamic> j) => LineItem(
        x: (j['x'] as num).toDouble(), y: (j['y'] as num).toDouble(),
        width: (j['width'] as num).toDouble(),
        thickness: (j['thickness'] as num).toDouble(),
        color: RaTeXColor.fromJson(j['color'] as Map<String, dynamic>),
        dashed: j['dashed'] as bool? ?? false,
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
        commands: (j['commands'] as List).map((e) => PathCommand.fromJson(e as Map<String, dynamic>)).toList(),
        fill: j['fill'] as bool,
        color: RaTeXColor.fromJson(j['color'] as Map<String, dynamic>),
      );
}

// MARK: - Path commands (internally tagged: {"type": "MoveTo", "x": ..., "y": ...})

sealed class PathCommand {
  const PathCommand();

  factory PathCommand.fromJson(Map<String, dynamic> json) {
    final type = json['type'] as String;
    switch (type) {
      case 'MoveTo':
        return MoveToCmd((json['x'] as num).toDouble(), (json['y'] as num).toDouble());
      case 'LineTo':
        return LineToCmd((json['x'] as num).toDouble(), (json['y'] as num).toDouble());
      case 'CubicTo':
        return CubicToCmd(
          (json['x1'] as num).toDouble(), (json['y1'] as num).toDouble(),
          (json['x2'] as num).toDouble(), (json['y2'] as num).toDouble(),
          (json['x']  as num).toDouble(), (json['y']  as num).toDouble(),
        );
      case 'QuadTo':
        return QuadToCmd(
          (json['x1'] as num).toDouble(), (json['y1'] as num).toDouble(),
          (json['x']  as num).toDouble(), (json['y']  as num).toDouble(),
        );
      case 'Close':
        return const CloseCmd();
      default:
        throw FormatException('Unknown PathCommand type: $type');
    }
  }
}

class MoveToCmd  extends PathCommand { final double x, y; const MoveToCmd(this.x, this.y); }
class LineToCmd  extends PathCommand { final double x, y; const LineToCmd(this.x, this.y); }
class CubicToCmd extends PathCommand {
  final double x1, y1, x2, y2, x, y;
  const CubicToCmd(this.x1, this.y1, this.x2, this.y2, this.x, this.y);
}
class QuadToCmd  extends PathCommand {
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

  /// Convert to a Flutter Color (32-bit ARGB int).
  int toFlutterColor() {
    final ai = (a * 255).round().clamp(0, 255);
    final ri = (r * 255).round().clamp(0, 255);
    final gi = (g * 255).round().clamp(0, 255);
    final bi = (b * 255).round().clamp(0, 255);
    return (ai << 24) | (ri << 16) | (gi << 8) | bi;
  }
}
