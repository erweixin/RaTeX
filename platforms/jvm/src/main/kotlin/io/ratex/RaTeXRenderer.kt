// RaTeXRenderer.kt — AWT Graphics2D renderer for a RaTeX DisplayList.

package io.ratex

import java.awt.BasicStroke
import java.awt.Font
import java.awt.Graphics2D
import java.awt.RenderingHints
import java.awt.geom.GeneralPath
import java.awt.geom.Line2D
import java.awt.geom.Rectangle2D
import java.awt.image.BufferedImage

/**
 * Renders a [DisplayList] using Java AWT [Graphics2D].
 *
 * All em-unit coordinates are multiplied by [fontSize] to get pixel coordinates.
 *
 * @param displayList  The layout output from [RaTeXEngine.parseBlocking].
 * @param fontSize     Font size in pixels.
 * @param fontLoader   Optional (fontId: String) -> Font? for glyph rendering.
 */
class RaTeXRenderer(
    val displayList: DisplayList,
    val fontSize: Float,
    private val fontLoader: ((String) -> Font?)? = null,
) {
    // MARK: - Dimensions in pixels

    val widthPx:       Float get() = (displayList.width  * fontSize).toFloat()
    val heightPx:      Float get() = (displayList.height * fontSize).toFloat()
    val depthPx:       Float get() = (displayList.depth  * fontSize).toFloat()
    val totalHeightPx: Float get() = heightPx + depthPx

    // MARK: - Drawing

    /** Draw the formula into [g2]. The graphics origin is the top-left of the bounding box. */
    fun draw(g2: Graphics2D) {
        g2.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON)
        g2.setRenderingHint(RenderingHints.KEY_TEXT_ANTIALIASING, RenderingHints.VALUE_TEXT_ANTIALIAS_ON)
        g2.setRenderingHint(RenderingHints.KEY_FRACTIONALMETRICS, RenderingHints.VALUE_FRACTIONALMETRICS_ON)

        for (item in displayList.items) {
            when (item) {
                is DisplayItem.GlyphPath -> drawGlyph(g2, item)
                is DisplayItem.Line      -> drawLine(g2, item)
                is DisplayItem.Rect      -> drawRect(g2, item)
                is DisplayItem.Path      -> drawPath(g2, item)
            }
        }
    }

    /**
     * Render to a [BufferedImage] with transparent background.
     * @param padding Extra padding in pixels around the formula.
     */
    fun renderToImage(padding: Int = 4): BufferedImage {
        val w = (widthPx + padding * 2).toInt().coerceAtLeast(1)
        val h = (totalHeightPx + padding * 2).toInt().coerceAtLeast(1)
        val image = BufferedImage(w, h, BufferedImage.TYPE_INT_ARGB)
        val g2 = image.createGraphics()
        g2.translate(padding, padding)
        draw(g2)
        g2.dispose()
        return image
    }

    // MARK: - Private helpers

    private fun Double.em() = (this * fontSize).toFloat()

    private fun drawGlyph(g2: Graphics2D, g: DisplayItem.GlyphPath) {
        val baseFont = fontLoader?.invoke(g.font) ?: return
        val codePoint = g.charCode
        if (!Character.isValidCodePoint(codePoint)) return

        val str = String(Character.toChars(codePoint))
        val derivedFont = baseFont.deriveFont(fontSize * g.scale.toFloat())

        g2.color = g.color.toAwtColor()
        g2.font = derivedFont
        g2.drawString(str, g.x.em(), g.y.em())
    }

    private fun drawLine(g2: Graphics2D, l: DisplayItem.Line) {
        val halfT = (l.thickness * fontSize / 2).toFloat()
        val left   = l.x.em()
        val centerY = l.y.em()
        val width  = l.width.em()
        val height = halfT * 2

        g2.color = l.color.toAwtColor()
        if (l.dashed) {
            val dash = floatArrayOf(fontSize * 0.15f, fontSize * 0.1f)
            g2.stroke = BasicStroke(height, BasicStroke.CAP_BUTT, BasicStroke.JOIN_MITER, 10f, dash, 0f)
            g2.draw(Line2D.Float(left, centerY, left + width, centerY))
        } else {
            g2.fill(Rectangle2D.Float(left, centerY - halfT, width.coerceAtLeast(1f / fontSize), height.coerceAtLeast(1f / fontSize)))
        }
    }

    private fun drawRect(g2: Graphics2D, r: DisplayItem.Rect) {
        g2.color = r.color.toAwtColor()
        g2.fill(Rectangle2D.Float(
            r.x.em(), r.y.em(),
            r.width.em().coerceAtLeast(1f / fontSize),
            r.height.em().coerceAtLeast(1f / fontSize),
        ))
    }

    private fun drawPath(g2: Graphics2D, p: DisplayItem.Path) {
        val path = buildPath(p.commands, p.x.em(), p.y.em())
        g2.color = p.color.toAwtColor()
        if (p.fill) {
            g2.fill(path)
        } else {
            g2.stroke = BasicStroke(1f)
            g2.draw(path)
        }
    }

    private fun buildPath(commands: List<PathCommand>, dx: Float = 0f, dy: Float = 0f): GeneralPath {
        val path = GeneralPath()
        for (cmd in commands) {
            when (cmd) {
                is PathCommand.MoveTo  -> path.moveTo(dx + cmd.x.em(), dy + cmd.y.em())
                is PathCommand.LineTo  -> path.lineTo(dx + cmd.x.em(), dy + cmd.y.em())
                is PathCommand.CubicTo -> path.curveTo(
                    dx + cmd.x1.em(), dy + cmd.y1.em(),
                    dx + cmd.x2.em(), dy + cmd.y2.em(),
                    dx + cmd.x.em(),  dy + cmd.y.em(),
                )
                is PathCommand.QuadTo  -> path.quadTo(
                    dx + cmd.x1.em(), dy + cmd.y1.em(),
                    dx + cmd.x.em(),  dy + cmd.y.em(),
                )
                PathCommand.Close      -> path.closePath()
            }
        }
        return path
    }
}
