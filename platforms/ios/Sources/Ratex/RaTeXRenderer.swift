// RaTeXRenderer.swift — CoreGraphics + CoreText renderer for a RaTeX DisplayList.
//
// Key insight: GlyphPath.commands are PLACEHOLDER RECTANGLES (bounding boxes),
// not real glyph outlines. Glyphs must be drawn with CTFont using font + char_code.
// Non-glyph items (Line, Rect, Path) use their commands directly.
//
// Fonts are registered automatically on first use via RaTeXFontLoader.ensureLoaded(); or call loadFromBundle() at startup.

import CoreGraphics
import CoreText
import Foundation

public struct RaTeXRenderer {
    public let displayList: DisplayList
    /// Font size in points. All em-unit coordinates are multiplied by this value.
    public let fontSize: CGFloat

    public init(displayList: DisplayList, fontSize: CGFloat = 24) {
        self.displayList = displayList
        self.fontSize = fontSize
    }

    // MARK: - Dimensions (in points)

    public var width:       CGFloat { CGFloat(displayList.width)  * fontSize }
    public var height:      CGFloat { CGFloat(displayList.height) * fontSize }
    public var depth:       CGFloat { CGFloat(displayList.depth)  * fontSize }
    public var totalHeight: CGFloat { height + depth }

    // MARK: - Drawing

    /// Draw the formula into `context`.
    /// The origin is the top-left of the formula's bounding box.
    public func draw(in context: CGContext) {
        for item in displayList.items {
            switch item {
            case .glyphPath(let g): drawGlyph(g, in: context)
            case .line(let l):      drawLine(l, in: context)
            case .rect(let r):      drawRect(r, in: context)
            case .path(let p):      drawPath(p, in: context)
            case .unknown:
                // Forward compatibility: ignore items newer than this decoder.
                break
            }
        }
    }

    // MARK: - Private helpers

    /// Convert em units to points.
    private func pt(_ em: Double) -> CGFloat { CGFloat(em) * fontSize }

    private func cgColor(_ c: RaTeXColor) -> CGColor {
        CGColor(red: CGFloat(c.r), green: CGFloat(c.g),
                blue: CGFloat(c.b), alpha: CGFloat(c.a))
    }

    // MARK: Glyph (CoreText)

    /// Map RaTeX font ID ("Math-Italic") to the KaTeX PostScript name ("KaTeX_Math-Italic").
    private func postScriptName(for fontId: String) -> String {
        "KaTeX_\(fontId)"
    }

    private func drawGlyph(_ g: GlyphPathData, in ctx: CGContext) {
        // GlyphPath.commands are placeholder rects — ignore them.
        // Draw the actual character using CoreText.
        guard let scalar = Unicode.Scalar(g.charCode) else { return }
        let char = String(Character(scalar))

        let psName  = postScriptName(for: g.font)
        let ctFont  = CTFontCreateWithName(psName as CFString, pt(g.scale), nil)
        let color   = cgColor(g.color)

        let attrs: [CFString: Any] = [
            kCTFontAttributeName:                    ctFont,
            kCTForegroundColorAttributeName:         color,
        ]
        let attrStr = CFAttributedStringCreate(nil, char as CFString, attrs as CFDictionary)!
        let line    = CTLineCreateWithAttributedString(attrStr)

        ctx.saveGState()

        // In UIKit's draw(_:) context, Y increases downward (UIKit has already flipped the CTM).
        // The GlyphPath y-coordinate is the alphabetic baseline measured downward from the top.
        ctx.translateBy(x: pt(g.x), y: pt(g.y))

        // CoreText draws upward from the baseline.
        // UIKit's flipped CTM would make CoreText text appear upside-down.
        // textMatrix = (1,0,0,-1) counteracts the y-flip for text rendering.
        ctx.textMatrix = CGAffineTransform(a: 1, b: 0, c: 0, d: -1, tx: 0, ty: 0)

        CTLineDraw(line, ctx)

        ctx.restoreGState()
    }

    // MARK: Line

    private func drawLine(_ l: LineData, in ctx: CGContext) {
        ctx.saveGState()
        ctx.setFillColor(cgColor(l.color))
        let halfT = pt(l.thickness) / 2
        ctx.fill(CGRect(x: pt(l.x), y: pt(l.y) - halfT,
                        width: pt(l.width), height: pt(l.thickness)))
        ctx.restoreGState()
    }

    // MARK: Rect

    private func drawRect(_ r: RectData, in ctx: CGContext) {
        ctx.saveGState()
        ctx.setFillColor(cgColor(r.color))
        ctx.fill(CGRect(x: pt(r.x), y: pt(r.y),
                        width: pt(r.width), height: pt(r.height)))
        ctx.restoreGState()
    }

    // MARK: Path (radical signs, delimiters, etc.)

    private func makeCGPath(from commands: [PathCommand], dx: Double = 0, dy: Double = 0) -> CGPath {
        let path = CGMutablePath()
        let ox = pt(dx), oy = pt(dy)
        for cmd in commands {
            switch cmd {
            case .moveTo(let x, let y):
                path.move(to: CGPoint(x: ox + pt(x), y: oy + pt(y)))
            case .lineTo(let x, let y):
                path.addLine(to: CGPoint(x: ox + pt(x), y: oy + pt(y)))
            case .cubicTo(let x1, let y1, let x2, let y2, let x, let y):
                path.addCurve(to:       CGPoint(x: ox + pt(x),  y: oy + pt(y)),
                              control1: CGPoint(x: ox + pt(x1), y: oy + pt(y1)),
                              control2: CGPoint(x: ox + pt(x2), y: oy + pt(y2)))
            case .quadTo(let x1, let y1, let x, let y):
                path.addQuadCurve(to:      CGPoint(x: ox + pt(x),  y: oy + pt(y)),
                                  control: CGPoint(x: ox + pt(x1), y: oy + pt(y1)))
            case .close:
                path.closeSubpath()
            case .unknown:
                // Forward compatibility: ignore newer command variants.
                break
            }
        }
        return path
    }

    private func drawPath(_ p: PathData, in ctx: CGContext) {
        ctx.saveGState()
        let cgPath = makeCGPath(from: p.commands, dx: p.x, dy: p.y)
        ctx.addPath(cgPath)
        let color = cgColor(p.color)
        if p.fill {
            ctx.setFillColor(color)
            ctx.fillPath()
        } else {
            ctx.setStrokeColor(color)
            ctx.strokePath()
        }
        ctx.restoreGState()
    }
}
