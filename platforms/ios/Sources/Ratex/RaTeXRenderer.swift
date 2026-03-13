// RaTeXRenderer.swift — CoreGraphics renderer for a RaTeX DisplayList.
//
// Usage:
//   let renderer = RaTeXRenderer(displayList: displayList, fontSize: 24)
//   renderer.draw(in: context, bounds: rect)

import CoreGraphics
import Foundation

public struct RaTeXRenderer {
    public let displayList: DisplayList
    /// Font size in points (px on screen). All em-unit coordinates are multiplied by this.
    public let fontSize: CGFloat

    public init(displayList: DisplayList, fontSize: CGFloat = 24) {
        self.displayList = displayList
        self.fontSize = fontSize
    }

    // MARK: - Dimensions (in points)

    /// Width of the rendered formula in points.
    public var width: CGFloat { CGFloat(displayList.width) * fontSize }
    /// Height above baseline in points.
    public var height: CGFloat { CGFloat(displayList.height) * fontSize }
    /// Depth below baseline in points.
    public var depth: CGFloat { CGFloat(displayList.depth) * fontSize }
    /// Total bounding box height (height + depth) in points.
    public var totalHeight: CGFloat { height + depth }

    // MARK: - Drawing

    /// Draw the formula into `context`.
    ///
    /// The context's origin is assumed to be at the top-left of the formula's
    /// bounding box (i.e. the top-left corner maps to (0, 0) in em space).
    public func draw(in context: CGContext) {
        for item in displayList.items {
            switch item {
            case .glyphPath(let g): drawGlyph(g, in: context)
            case .line(let l):      drawLine(l, in: context)
            case .rect(let r):      drawRect(r, in: context)
            case .path(let p):      drawPath(p, in: context)
            }
        }
    }

    // MARK: - Private helpers

    private func pt(_ em: Double) -> CGFloat { CGFloat(em) * fontSize }

    private func cgColor(_ c: RaTeXColor) -> CGColor {
        CGColor(red: CGFloat(c.r), green: CGFloat(c.g),
                blue: CGFloat(c.b), alpha: CGFloat(c.a))
    }

    private func cgPath(from commands: [PathCommand], dx: Double = 0, dy: Double = 0) -> CGPath {
        let path = CGMutablePath()
        let ox = pt(dx), oy = pt(dy)
        for cmd in commands {
            switch cmd {
            case .moveTo(let x, let y):
                path.move(to: CGPoint(x: ox + pt(x), y: oy + pt(y)))
            case .lineTo(let x, let y):
                path.addLine(to: CGPoint(x: ox + pt(x), y: oy + pt(y)))
            case .cubicTo(let x1, let y1, let x2, let y2, let x, let y):
                path.addCurve(
                    to:          CGPoint(x: ox + pt(x),  y: oy + pt(y)),
                    control1:    CGPoint(x: ox + pt(x1), y: oy + pt(y1)),
                    control2:    CGPoint(x: ox + pt(x2), y: oy + pt(y2)))
            case .quadTo(let x1, let y1, let x, let y):
                path.addQuadCurve(
                    to:       CGPoint(x: ox + pt(x),  y: oy + pt(y)),
                    control:  CGPoint(x: ox + pt(x1), y: oy + pt(y1)))
            case .close:
                path.closeSubpath()
            }
        }
        return path
    }

    private func drawGlyph(_ g: GlyphPathData, in ctx: CGContext) {
        ctx.saveGState()
        ctx.setFillColor(cgColor(g.color))
        // Apply glyph scale around the glyph origin
        ctx.translateBy(x: pt(g.x), y: pt(g.y))
        ctx.scaleBy(x: CGFloat(g.scale), y: CGFloat(g.scale))
        let path = cgPath(from: g.commands)
        ctx.addPath(path)
        ctx.fillPath()
        ctx.restoreGState()
    }

    private func drawLine(_ l: LineData, in ctx: CGContext) {
        ctx.saveGState()
        ctx.setFillColor(cgColor(l.color))
        let halfT = pt(l.thickness) / 2
        let rect = CGRect(
            x:      pt(l.x),
            y:      pt(l.y) - halfT,
            width:  pt(l.width),
            height: pt(l.thickness))
        ctx.fill(rect)
        ctx.restoreGState()
    }

    private func drawRect(_ r: RectData, in ctx: CGContext) {
        ctx.saveGState()
        ctx.setFillColor(cgColor(r.color))
        ctx.fill(CGRect(x: pt(r.x), y: pt(r.y), width: pt(r.width), height: pt(r.height)))
        ctx.restoreGState()
    }

    private func drawPath(_ p: PathData, in ctx: CGContext) {
        ctx.saveGState()
        let color = cgColor(p.color)
        let path = cgPath(from: p.commands, dx: p.x, dy: p.y)
        ctx.addPath(path)
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
