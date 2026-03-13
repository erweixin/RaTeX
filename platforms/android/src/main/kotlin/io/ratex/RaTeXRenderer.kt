// RaTeXRenderer.kt — Android Canvas renderer for a RaTeX DisplayList.

package io.ratex

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path as AndroidPath

/**
 * Renders a [DisplayList] onto an Android [Canvas].
 *
 * All em-unit coordinates are multiplied by [fontSize] (sp/px) to get screen coordinates.
 *
 * @param displayList  The layout output from [RaTeXEngine.parse].
 * @param fontSize     Font size in pixels. Matches the desired display size on screen.
 */
class RaTeXRenderer(
    val displayList: DisplayList,
    val fontSize: Float,
) {
    // MARK: - Dimensions in pixels

    val widthPx:       Float get() = (displayList.width  * fontSize).toFloat()
    val heightPx:      Float get() = (displayList.height * fontSize).toFloat()
    val depthPx:       Float get() = (displayList.depth  * fontSize).toFloat()
    val totalHeightPx: Float get() = heightPx + depthPx

    // MARK: - Drawing

    /** Draw the formula into [canvas]. The canvas origin is the top-left of the bounding box. */
    fun draw(canvas: Canvas) {
        for (item in displayList.items) {
            when (item) {
                is DisplayItem.GlyphPath -> drawGlyph(canvas, item.data)
                is DisplayItem.Line      -> drawLine(canvas, item.data)
                is DisplayItem.Rect      -> drawRect(canvas, item.data)
                is DisplayItem.Path      -> drawPath(canvas, item.data)
            }
        }
    }

    // MARK: - Private helpers

    private fun Float.em() = this * fontSize
    private fun Double.em() = (this * fontSize).toFloat()

    private fun Paint.applyColor(c: RaTeXColor) { color = c.toArgb() }

    private fun buildAndroidPath(commands: List<PathCommand>, dx: Float = 0f, dy: Float = 0f): AndroidPath {
        val path = AndroidPath()
        for (cmd in commands) {
            when (cmd) {
                is PathCommand.MoveTo  -> path.moveTo(dx + cmd.x.em(), dy + cmd.y.em())
                is PathCommand.LineTo  -> path.lineTo(dx + cmd.x.em(), dy + cmd.y.em())
                is PathCommand.CubicTo -> path.cubicTo(
                    dx + cmd.x1.em(), dy + cmd.y1.em(),
                    dx + cmd.x2.em(), dy + cmd.y2.em(),
                    dx + cmd.x.em(),  dy + cmd.y.em())
                is PathCommand.QuadTo  -> path.quadTo(
                    dx + cmd.x1.em(), dy + cmd.y1.em(),
                    dx + cmd.x.em(),  dy + cmd.y.em())
                PathCommand.Close      -> path.close()
            }
        }
        return path
    }

    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)

    private fun drawGlyph(canvas: Canvas, g: GlyphPathData) {
        canvas.save()
        canvas.translate(g.x.em(), g.y.em())
        canvas.scale(g.scale.toFloat(), g.scale.toFloat())
        val path = buildAndroidPath(g.commands)
        paint.style = Paint.Style.FILL
        paint.applyColor(g.color)
        canvas.drawPath(path, paint)
        canvas.restore()
    }

    private fun drawLine(canvas: Canvas, l: LineData) {
        val halfT = (l.thickness * fontSize / 2).toFloat()
        val left   = l.x.em()
        val top    = l.y.em() - halfT
        val right  = (l.x + l.width).em()
        val bottom = l.y.em() + halfT
        paint.style = Paint.Style.FILL
        paint.applyColor(l.color)
        canvas.drawRect(left, top, right, bottom, paint)
    }

    private fun drawRect(canvas: Canvas, r: RectData) {
        paint.style = Paint.Style.FILL
        paint.applyColor(r.color)
        canvas.drawRect(
            r.x.em(), r.y.em(),
            (r.x + r.width).em(), (r.y + r.height).em(),
            paint)
    }

    private fun drawPath(canvas: Canvas, p: PathData) {
        val path = buildAndroidPath(p.commands, p.x.em(), p.y.em())
        paint.applyColor(p.color)
        paint.style = if (p.fill) Paint.Style.FILL else Paint.Style.STROKE
        canvas.drawPath(path, paint)
    }
}
