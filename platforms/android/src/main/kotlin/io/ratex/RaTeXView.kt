// RaTeXView.kt — Android custom View that renders a LaTeX formula.

package io.ratex

import android.content.Context
import android.graphics.Canvas
import android.util.AttributeSet
import android.view.View
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.cancel
import kotlinx.coroutines.launch

/**
 * A custom [View] that renders a LaTeX math formula using the RaTeX engine.
 *
 * XML usage:
 * ```xml
 * <io.ratex.RaTeXView
 *     android:id="@+id/mathView"
 *     android:layout_width="wrap_content"
 *     android:layout_height="wrap_content"
 *     app:latex="\frac{1}{2}"
 *     app:fontSize="24" />
 * ```
 *
 * Kotlin usage:
 * ```kotlin
 * binding.mathView.latex   = """\frac{-b \pm \sqrt{b^2-4ac}}{2a}"""
 * binding.mathView.fontSize = 28f
 * ```
 */
class RaTeXView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyle: Int = 0,
) : View(context, attrs, defStyle) {

    // MARK: - Public properties

    /** LaTeX math-mode string to render. Setting this triggers an async re-render. */
    var latex: String = ""
        set(value) {
            if (field == value) return
            field = value
            rerender()
        }

    /** Font size in pixels. Setting this triggers an async re-render. */
    var fontSize: Float = 24f
        set(value) {
            if (field == value) return
            field = value
            rerender()
        }

    /** Called on the main thread when a render error occurs. */
    var onError: ((RaTeXException) -> Unit)? = null

    // MARK: - Private state

    private var renderer: RaTeXRenderer? = null
    private val scope = CoroutineScope(Dispatchers.Main)
    private var renderJob: Job? = null

    // MARK: - Measure

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val r = renderer
        if (r == null) {
            setMeasuredDimension(0, 0)
        } else {
            setMeasuredDimension(r.widthPx.toInt(), r.totalHeightPx.toInt())
        }
    }

    // MARK: - Draw

    override fun onDraw(canvas: Canvas) {
        renderer?.draw(canvas)
    }

    // MARK: - Lifecycle

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        scope.cancel()
    }

    // MARK: - Private

    private fun rerender() {
        renderJob?.cancel()
        renderJob = scope.launch {
            try {
                val dl = RaTeXEngine.parse(latex)
                renderer = RaTeXRenderer(dl, fontSize)
                requestLayout()
                invalidate()
            } catch (e: RaTeXException) {
                onError?.invoke(e)
            }
        }
    }
}
