// RaTeXRenderer.kt — Android Canvas renderer for a RaTeX DisplayList.

package io.ratex

import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path as AndroidPath
import android.graphics.Typeface
import android.util.Log

/**
 * Renders a [DisplayList] onto an Android [Canvas].
 *
 * All em-unit coordinates are multiplied by [fontSize] (sp/px) to get screen coordinates.
 *
 * GlyphPath items are **placeholder rectangles** from Rust; glyphs are drawn with
 * KaTeX fonts. [RaTeXView] calls [RaTeXFontLoader.ensureLoaded] on first use. Pass
 * [typefaceLoader] (e.g. [RaTeXFontLoader.getTypeface]). If null, glyphs are drawn as black rectangles.
 *
 * @param displayList   The layout output from [RaTeXEngine.parse].
 * @param fontSize      Font size in pixels. Matches the desired display size on screen.
 * @param typefaceLoader Optional (fontId: String) -> Typeface? for glyph rendering.
 */
class RaTeXRenderer(
    val displayList: DisplayList,
    val fontSize: Float,
    private val typefaceLoader: ((String) -> Typeface?)? = null,
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
                is DisplayItem.GlyphPath -> drawGlyph(canvas, item)
                is DisplayItem.Line      -> drawLine(canvas, item)
                is DisplayItem.Rect      -> drawRect(canvas, item)
                is DisplayItem.Path      -> drawPath(canvas, item)
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
    private val textPaint = Paint(Paint.ANTI_ALIAS_FLAG).apply { isAntiAlias = true }

    private fun drawGlyph(canvas: Canvas, g: DisplayItem.GlyphPath) {
        val typeface = typefaceLoader?.invoke(g.font) ?: return
        val codePoint = g.charCode.toInt()
        if (!Character.isValidCodePoint(codePoint)) {
            Log.w("RaTeXRenderer", "Invalid Unicode code point: $codePoint (0x${codePoint.toString(16)}) in font '${g.font}' — skipping glyph")
            return
        }
        canvas.save()
        canvas.translate(g.x.em(), g.y.em())
        val str = String(Character.toChars(codePoint))
        textPaint.typeface = typeface
        textPaint.textSize = fontSize * g.scale.toFloat()
        textPaint.color = g.color.toArgb()
        textPaint.style = Paint.Style.FILL
        canvas.drawText(str, 0f, 0f, textPaint)
        canvas.restore()
    }

    private fun drawLine(canvas: Canvas, l: DisplayItem.Line) {
        val t = maxOf(0.5f, (l.thickness * fontSize).toFloat())
        val halfT = t / 2f
        val left   = l.x.em()
        val top    = l.y.em() - halfT
        val right  = (l.x + l.width).em()
        val bottom = l.y.em() + halfT
        paint.applyColor(l.color)
        if (l.dashed) {
            val dashLen = t * 3f
            paint.style = Paint.Style.STROKE
            paint.strokeWidth = t
            paint.strokeCap = Paint.Cap.BUTT
            paint.pathEffect = android.graphics.DashPathEffect(floatArrayOf(dashLen, dashLen), 0f)
            val path = android.graphics.Path()
            path.moveTo(left, l.y.em())
            path.lineTo(right, l.y.em())
            canvas.drawPath(path, paint)
            paint.pathEffect = null
        } else {
            paint.style = Paint.Style.FILL
            canvas.drawRect(left, top, right, bottom, paint)
        }
    }

    private fun drawRect(canvas: Canvas, r: DisplayItem.Rect) {
        paint.style = Paint.Style.FILL
        paint.applyColor(r.color)
        canvas.drawRect(
            r.x.em(), r.y.em(),
            (r.x + r.width).em(), (r.y + r.height).em(),
            paint)
    }

    private fun drawPath(canvas: Canvas, p: DisplayItem.Path) {
        val path = buildAndroidPath(p.commands, p.x.em(), p.y.em())
        paint.applyColor(p.color)
        paint.style = if (p.fill) Paint.Style.FILL else Paint.Style.STROKE
        canvas.drawPath(path, paint)
    }
}
