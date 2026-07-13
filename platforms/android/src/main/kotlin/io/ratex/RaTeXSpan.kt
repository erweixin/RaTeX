// RaTeXSpan.kt — ReplacementSpan that renders a LaTeX formula inline with text.

package io.ratex

import android.content.Context
import android.graphics.Bitmap
import android.graphics.Canvas
import android.graphics.Paint
import android.text.style.ReplacementSpan
import android.graphics.Color
import androidx.annotation.ColorInt
import kotlin.math.ceil
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

/**
 * A [ReplacementSpan] that renders a LaTeX formula inline with surrounding text.
 *
 * The formula baseline is aligned to the text baseline, and the line height expands
 * automatically to accommodate the formula's ascent and descent.
 *
 * Usage (inside a coroutine):
 * ```kotlin
 * val span = RaTeXSpan.create(context, latex = """\frac{1}{2}""", fontSize = 18f)
 * val ssb = SpannableStringBuilder("Area = \u200B of the circle")
 * ssb.setSpan(span, 7, 8, Spannable.SPAN_EXCLUSIVE_EXCLUSIVE)
 * textView.text = ssb
 * ```
 *
 * @param bitmap   Pre-rendered formula bitmap (transparent background).
 * @param heightPx Formula height above the baseline, in pixels.
 * @param depthPx  Rendered depth below the baseline, in pixels.
 */
class RaTeXSpan private constructor(
    private val bitmap: Bitmap,
    private val heightPx: Float,
    private val depthPx: Float,
) : ReplacementSpan() {

    companion object {
        /**
         * Renders [latex] at [fontSize] dp and returns a ready-to-use [RaTeXSpan].
         *
         * Font loading and rendering run on [Dispatchers.IO]. Call from a coroutine or
         * `suspend` function; the result is delivered on the caller's dispatcher.
         *
         * @param context     Any context; used only for asset access during font loading.
         * @param latex       LaTeX math-mode string (no surrounding `$` or `\[…\]`).
         * @param fontSize    Font size in **dp** (density-independent pixels). Converted to px internally.
         * @param color       Text color as an ARGB integer (default [Color.BLACK]).
         * @param displayMode `true` (default) for display/block style; `false` for inline/text style.
         * @throws RaTeXException if the formula cannot be parsed.
         */
        @JvmOverloads
        suspend fun create(
            context: Context,
            latex: String,
            fontSize: Float,
            @ColorInt color: Int = Color.BLACK,
            displayMode: Boolean = true,
        ): RaTeXSpan =
            withContext(Dispatchers.IO) {
                RaTeXFontLoader.ensureLoaded(context)
                val dl = RaTeXEngine.parse(latex, displayMode, color)
                val fontSizePx = fontSize * context.resources.displayMetrics.density
                val renderer = RaTeXRenderer(dl, fontSizePx) { RaTeXFontLoader.getTypeface(it) }

                val w = ceil(renderer.widthPx).toInt().coerceAtLeast(1)
                val h = ceil(renderer.totalHeightPx).toInt().coerceAtLeast(1)
                val bitmap = Bitmap.createBitmap(w, h, Bitmap.Config.ARGB_8888)
                renderer.draw(Canvas(bitmap))

                RaTeXSpan(bitmap, renderer.heightPx, renderer.depthPx)
            }
    }

    // MARK: - ReplacementSpan

    override fun getSize(
        paint: Paint,
        text: CharSequence?,
        start: Int,
        end: Int,
        fm: Paint.FontMetricsInt?,
    ): Int {
        fm?.let {
            // Expand the line height to fully accommodate the formula.
            it.ascent  = -ceil(heightPx).toInt()
            it.descent =  ceil(depthPx).toInt()
            it.top     = it.ascent
            it.bottom  = it.descent
        }
        return bitmap.width
    }

    override fun draw(
        canvas: Canvas,
        text: CharSequence?,
        start: Int,
        end: Int,
        x: Float,
        top: Int,
        y: Int,       // y = text baseline Y coordinate
        bottom: Int,
        paint: Paint,
    ) {
        // Translate so the formula baseline (heightPx from bitmap top) sits on the text baseline.
        canvas.save()
        canvas.translate(x, y - heightPx)
        canvas.drawBitmap(bitmap, 0f, 0f, null)
        canvas.restore()
    }
}
