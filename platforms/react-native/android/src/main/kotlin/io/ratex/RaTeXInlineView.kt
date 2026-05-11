// RaTeXInlineView.kt — Android View that renders text mixed with inline LaTeX formulas.
//
// Content is parsed for $...$ delimiters. Each formula is laid out via RaTeXEngine,
// wrapped in a RaTeXFormulaSpan (ReplacementSpan), and embedded in a SpannableString.
// Android's StaticLayout handles word-wrapping and line-breaking at character level.

package io.ratex

import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.text.SpannableStringBuilder
import android.text.StaticLayout
import android.text.TextPaint
import android.util.AttributeSet
import android.view.View
import androidx.annotation.ColorInt
import kotlin.math.max
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext

class RaTeXInlineView @JvmOverloads constructor(
    context: Context,
    attrs: AttributeSet? = null,
    defStyle: Int = 0,
) : View(context, attrs, defStyle) {

    // MARK: - Public properties

    var content: String = ""
        set(value) {
            if (field == value) return
            field = value
            rebuild()
        }

    var formulaFontSize: Float = 16f
        set(value) {
            if (field == value) return
            field = value
            rebuild()
        }

    @ColorInt
    var formulaColor: Int = Color.BLACK
        set(value) {
            if (field == value) return
            field = value
            rebuild()
        }

    @ColorInt
    var inlineTextColor: Int = Color.BLACK
        set(value) {
            if (field == value) return
            field = value
            rebuild()
        }

    var textFontSize: Float = 16f
        set(value) {
            if (field == value) return
            field = value
            rebuild()
        }

    var onContentSizeChange: ((width: Double, height: Double) -> Unit)? = null

    // MARK: - Private state

    private var currentSpannable: SpannableStringBuilder? = null
    private var staticLayout: StaticLayout? = null
    private val textPaint = TextPaint(TextPaint.ANTI_ALIAS_FLAG)
    private val scope = CoroutineScope(Dispatchers.Main + SupervisorJob())
    private var buildJob: Job? = null
    private var lastReportedSize = Pair(0.0, 0.0)
    private var lastLayoutWidth = -1

    // MARK: - Measure

    override fun onMeasure(widthMeasureSpec: Int, heightMeasureSpec: Int) {
        val layout = staticLayout
        val desiredWidth = max(
            (layout?.width ?: 0) + paddingLeft + paddingRight,
            suggestedMinimumWidth,
        )
        val desiredHeight = max(
            (layout?.height ?: 0) + paddingTop + paddingBottom,
            suggestedMinimumHeight,
        )
        setMeasuredDimension(
            resolveSize(desiredWidth, widthMeasureSpec),
            resolveSize(desiredHeight, heightMeasureSpec),
        )
    }

    override fun onSizeChanged(w: Int, h: Int, oldw: Int, oldh: Int) {
        super.onSizeChanged(w, h, oldw, oldh)
        if (w != oldw) {
            rebuildLayout()
        }
    }

    // MARK: - Draw

    override fun onDraw(canvas: Canvas) {
        val layout = staticLayout ?: return
        canvas.save()
        canvas.translate(paddingLeft.toFloat(), paddingTop.toFloat())
        layout.draw(canvas)
        canvas.restore()
    }

    override fun onDetachedFromWindow() {
        super.onDetachedFromWindow()
        buildJob?.cancel()
        buildJob = null
    }

    // MARK: - Private

    private fun rebuild() {
        buildJob?.cancel()
        if (content.isBlank()) {
            currentSpannable = null
            staticLayout = null
            lastLayoutWidth = -1
            lastReportedSize = Pair(0.0, 0.0)
            requestLayout()
            invalidate()
            return
        }
        buildJob = scope.launch {
            withContext(Dispatchers.IO) { RaTeXFontLoader.ensureLoaded(context) }
            val spannable = buildSpannable()
            currentSpannable = spannable
            lastLayoutWidth = -1
            lastReportedSize = Pair(0.0, 0.0)
            rebuildLayout()
        }
    }

    private fun rebuildLayout() {
        val spannable = currentSpannable ?: return
        val availWidth = (width - paddingLeft - paddingRight).coerceAtLeast(1)
        if (availWidth == lastLayoutWidth) return
        lastLayoutWidth = availWidth

        val density = context.resources.displayMetrics.density
        textPaint.textSize = textFontSize * density
        textPaint.color = inlineTextColor

        val layout = StaticLayout.Builder
            .obtain(spannable, 0, spannable.length, textPaint, availWidth)
            .setAlignment(android.text.Layout.Alignment.ALIGN_NORMAL)
            .setLineSpacing(0f, 1f)
            .setIncludePad(true)
            .build()

        staticLayout = layout
        requestLayout()
        invalidate()

        val widthDp = layout.width.toDouble() / density
        val heightDp = layout.height.toDouble() / density
        val size = Pair(widthDp, heightDp)
        if (size != lastReportedSize) {
            lastReportedSize = size
            onContentSizeChange?.invoke(widthDp, heightDp)
        }
    }

    private suspend fun buildSpannable(): SpannableStringBuilder {
        val segments = parseContent(content)
        val builder = SpannableStringBuilder()
        val density = context.resources.displayMetrics.density
        val formulaFontSizePx = formulaFontSize * density

        for (segment in segments) {
            when (segment) {
                is Segment.Text -> builder.append(segment.content)
                is Segment.Formula -> {
                    val renderer = try {
                        val dl = withContext(Dispatchers.Default) {
                            RaTeXEngine.parseBlocking(
                                segment.content,
                                displayMode = false,
                                color = formulaColor,
                            )
                        }
                        RaTeXRenderer(dl, formulaFontSizePx) { RaTeXFontLoader.getTypeface(it) }
                    } catch (_: Exception) {
                        null
                    }
                    if (renderer != null && renderer.widthPx > 0) {
                        val start = builder.length
                        builder.append("\uFFFC")
                        val end = builder.length
                        builder.setSpan(
                            RaTeXFormulaSpan(renderer),
                            start, end,
                            SpannableStringBuilder.SPAN_EXCLUSIVE_EXCLUSIVE,
                        )
                    } else {
                        builder.append("$${segment.content}$")
                    }
                }
            }
        }
        return builder
    }

    // MARK: - Parsing

    sealed class Segment {
        data class Text(val content: String) : Segment()
        data class Formula(val content: String) : Segment()
    }

    companion object {
        fun parseContent(content: String): List<Segment> {
            val segments = mutableListOf<Segment>()
            val current = StringBuilder()
            var inFormula = false
            var prevWasBackslash = false

            for (ch in content) {
                if (prevWasBackslash) {
                    prevWasBackslash = false
                    if (ch == '$') {
                        current.append('$')
                        continue
                    }
                    current.append('\\')
                }

                if (ch == '\\' && !inFormula) {
                    prevWasBackslash = true
                    continue
                }

                if (ch == '$') {
                    if (inFormula) {
                        if (current.isNotEmpty()) {
                            segments.add(Segment.Formula(current.toString()))
                        }
                        current.clear()
                        inFormula = false
                    } else {
                        if (current.isNotEmpty()) {
                            segments.add(Segment.Text(current.toString()))
                        }
                        current.clear()
                        inFormula = true
                    }
                } else {
                    current.append(ch)
                }
            }

            if (prevWasBackslash) {
                current.append('\\')
            }

            if (current.isNotEmpty()) {
                if (inFormula) {
                    segments.add(Segment.Text("$$current"))
                } else {
                    segments.add(Segment.Text(current.toString()))
                }
            }

            return segments
        }
    }
}
