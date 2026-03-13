// DisplayList.swift — Swift mirror of ratex-types DisplayList / DisplayItem
// All types are Codable so they can be decoded from the JSON returned by ratex_parse_and_layout.

import Foundation

// MARK: - Top-level output

public struct DisplayList: Codable {
    /// Total width in em units.
    public let width: Double
    /// Ascent above baseline in em units.
    public let height: Double
    /// Descent below baseline in em units.
    public let depth: Double
    /// Ordered list of drawing commands.
    public let items: [DisplayItem]
}

// MARK: - Drawing commands

public enum DisplayItem: Codable {
    case glyphPath(GlyphPathData)
    case line(LineData)
    case rect(RectData)
    case path(PathData)

    // Serde-tagged union: { "GlyphPath": { ... } }
    private enum CodingKeys: String, CodingKey {
        case GlyphPath, Line, Rect, Path
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        if let data = try container.decodeIfPresent(GlyphPathData.self, forKey: .GlyphPath) {
            self = .glyphPath(data)
        } else if let data = try container.decodeIfPresent(LineData.self, forKey: .Line) {
            self = .line(data)
        } else if let data = try container.decodeIfPresent(RectData.self, forKey: .Rect) {
            self = .rect(data)
        } else if let data = try container.decodeIfPresent(PathData.self, forKey: .Path) {
            self = .path(data)
        } else {
            throw DecodingError.dataCorrupted(
                DecodingError.Context(codingPath: decoder.codingPath,
                                      debugDescription: "Unknown DisplayItem variant"))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .glyphPath(let d): try container.encode(d, forKey: .GlyphPath)
        case .line(let d):      try container.encode(d, forKey: .Line)
        case .rect(let d):      try container.encode(d, forKey: .Rect)
        case .path(let d):      try container.encode(d, forKey: .Path)
        }
    }
}

// MARK: - Item payloads

public struct GlyphPathData: Codable {
    /// X position in em units (from left edge of the bounding box).
    public let x: Double
    /// Y position in em units (from top edge of the bounding box).
    public let y: Double
    /// Uniform scale applied to path coordinates before placement.
    public let scale: Double
    /// Font family name, e.g. "KaTeX_Main-Regular".
    public let font: String
    /// Unicode code point of the glyph.
    public let charCode: UInt32
    /// Outline commands that define the glyph shape (in glyph-local coordinates).
    public let commands: [PathCommand]
    /// Fill color.
    public let color: RaTeXColor

    enum CodingKeys: String, CodingKey {
        case x, y, scale, font
        case charCode = "char_code"
        case commands, color
    }
}

public struct LineData: Codable {
    /// Left edge of the line in em units.
    public let x: Double
    /// Vertical position of the line centerline in em units.
    public let y: Double
    /// Length of the line in em units.
    public let width: Double
    /// Stroke thickness in em units.
    public let thickness: Double
    public let color: RaTeXColor
}

public struct RectData: Codable {
    /// Left edge in em units.
    public let x: Double
    /// Top edge in em units.
    public let y: Double
    public let width: Double
    public let height: Double
    public let color: RaTeXColor
}

public struct PathData: Codable {
    /// Translation applied to all path commands.
    public let x: Double
    public let y: Double
    public let commands: [PathCommand]
    /// true = fill, false = stroke.
    public let fill: Bool
    public let color: RaTeXColor
}

// MARK: - Path commands

public enum PathCommand: Codable {
    case moveTo(x: Double, y: Double)
    case lineTo(x: Double, y: Double)
    case cubicTo(x1: Double, y1: Double, x2: Double, y2: Double, x: Double, y: Double)
    case quadTo(x1: Double, y1: Double, x: Double, y: Double)
    case close

    private enum CodingKeys: String, CodingKey {
        case MoveTo, LineTo, CubicTo, QuadTo, Close
    }

    private struct XY: Codable { let x, y: Double }
    private struct Cubic: Codable { let x1, y1, x2, y2, x, y: Double }
    private struct Quad: Codable { let x1, y1, x, y: Double }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        if let d = try container.decodeIfPresent(XY.self, forKey: .MoveTo) {
            self = .moveTo(x: d.x, y: d.y)
        } else if let d = try container.decodeIfPresent(XY.self, forKey: .LineTo) {
            self = .lineTo(x: d.x, y: d.y)
        } else if let d = try container.decodeIfPresent(Cubic.self, forKey: .CubicTo) {
            self = .cubicTo(x1: d.x1, y1: d.y1, x2: d.x2, y2: d.y2, x: d.x, y: d.y)
        } else if let d = try container.decodeIfPresent(Quad.self, forKey: .QuadTo) {
            self = .quadTo(x1: d.x1, y1: d.y1, x: d.x, y: d.y)
        } else if container.contains(.Close) {
            self = .close
        } else {
            throw DecodingError.dataCorrupted(
                DecodingError.Context(codingPath: decoder.codingPath,
                                      debugDescription: "Unknown PathCommand variant"))
        }
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        switch self {
        case .moveTo(let x, let y):
            try container.encode(XY(x: x, y: y), forKey: .MoveTo)
        case .lineTo(let x, let y):
            try container.encode(XY(x: x, y: y), forKey: .LineTo)
        case .cubicTo(let x1, let y1, let x2, let y2, let x, let y):
            try container.encode(Cubic(x1: x1, y1: y1, x2: x2, y2: y2, x: x, y: y), forKey: .CubicTo)
        case .quadTo(let x1, let y1, let x, let y):
            try container.encode(Quad(x1: x1, y1: y1, x: x, y: y), forKey: .QuadTo)
        case .close:
            try container.encodeNil(forKey: .Close)
        }
    }
}

// MARK: - Color

/// RGBA color with components in [0, 1].
public struct RaTeXColor: Codable {
    public let r: Float
    public let g: Float
    public let b: Float
    public let a: Float

    public static let black = RaTeXColor(r: 0, g: 0, b: 0, a: 1)
}
