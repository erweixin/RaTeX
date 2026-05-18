// RaTeXInlineView.kt — Android View that renders text mixed with inline LaTeX formulas.
//
// Content is parsed for $...$ delimiters. Each formula is laid out via RaTeXEngine,
// wrapped in a RaTeXFormulaSpan (ReplacementSpan), and embedded in a SpannableString.
// Android's StaticLayout handles word-wrapping and line-breaking at character level.

package io.ratex

import android.content.Context
import android.content.res.AssetManager
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Typeface
import android.text.Layout
import android.text.SpannableStringBuilder
import android.text.StaticLayout
import android.text.TextPaint
import android.util.AttributeSet
import android.view.View
import androidx.annotation.ColorInt
import com.facebook.react.common.assets.ReactFontManager
import java.util.concurrent.ConcurrentHashMap
import java.util.Locale
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
            updateTextLayout()
        }

    var textFontSize: Float = 16f
        set(value) {
            if (field == value) return
            field = value
            updateTextLayout()
        }

    var textFontFamily: String? = null
        set(value) {
            val normalized = value?.trim()?.takeIf { it.isNotEmpty() }
            if (field == normalized) return
            field = normalized
            updateTextLayout()
        }

    var textItalic: Boolean = false
        set(value) {
            if (field == value) return
            field = value
            updateTextLayout()
        }

    var textUnderline: Boolean = false
        set(value) {
            if (field == value) return
            field = value
            updateTextLayout()
        }

    var textLineThrough: Boolean = false
        set(value) {
            if (field == value) return
            field = value
            updateTextLayout()
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
        val layout = ensureMeasuredLayout(widthMeasureSpec)
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
            reportContentSize(0.0, 0.0)
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
        val availWidth = width - paddingLeft - paddingRight
        if (availWidth <= 0) {
            requestLayout()
            invalidate()
            return
        }
        if (availWidth == lastLayoutWidth) return
        lastLayoutWidth = availWidth

        val density = context.resources.displayMetrics.density
        applyTextPaint(density)

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
        reportContentSize(widthDp, heightDp)
    }

    private fun ensureMeasuredLayout(widthMeasureSpec: Int): StaticLayout? {
        val spannable = currentSpannable ?: return null
        val widthMode = MeasureSpec.getMode(widthMeasureSpec)
        val widthSize = MeasureSpec.getSize(widthMeasureSpec)
        val horizontalPadding = paddingLeft + paddingRight
        val density = context.resources.displayMetrics.density
        applyTextPaint(density)

        val targetWidth = when (widthMode) {
            MeasureSpec.EXACTLY -> (widthSize - horizontalPadding).coerceAtLeast(1)
            MeasureSpec.AT_MOST -> {
                val maxWidth = (widthSize - horizontalPadding).coerceAtLeast(1)
                val desired = Layout.getDesiredWidth(spannable, textPaint).toInt().coerceAtLeast(1)
                minOf(desired, maxWidth)
            }
            else -> {
                val desired = Layout.getDesiredWidth(spannable, textPaint)
                desired.toInt().coerceAtLeast(1)
            }
        }

        val cachedLayout = staticLayout
        if (cachedLayout != null && lastLayoutWidth == targetWidth) {
            return cachedLayout
        }

        val measuredLayout = StaticLayout.Builder
            .obtain(spannable, 0, spannable.length, textPaint, targetWidth)
            .setAlignment(android.text.Layout.Alignment.ALIGN_NORMAL)
            .setLineSpacing(0f, 1f)
            .setIncludePad(true)
            .build()

        staticLayout = measuredLayout
        lastLayoutWidth = targetWidth

        val widthDp = measuredLayout.width.toDouble() / density
        val heightDp = measuredLayout.height.toDouble() / density
        reportContentSize(widthDp, heightDp)
        return measuredLayout
    }

    private fun reportContentSize(widthDp: Double, heightDp: Double) {
        val size = Pair(widthDp, heightDp)
        if (size != lastReportedSize) {
            lastReportedSize = size
            onContentSizeChange?.invoke(widthDp, heightDp)
        }
    }

    private fun updateTextLayout() {
        lastLayoutWidth = -1
        if (currentSpannable == null) return
        rebuildLayout()
    }

    private fun applyTextPaint(density: Float) {
        textPaint.textSize = textFontSize * density
        textPaint.color = inlineTextColor
        val style = if (textItalic) Typeface.ITALIC else Typeface.NORMAL
        textPaint.typeface = resolveTextTypeface(style)
        textPaint.isUnderlineText = textUnderline
        textPaint.isStrikeThruText = textLineThrough
    }

    private fun resolveTextTypeface(style: Int): Typeface {
        val family = textFontFamily ?: return Typeface.defaultFromStyle(style)
        loadTextTypefaceFromAssets(context.assets, family, style)?.let { return it }
        // ReactFontManager handles runtime-registered fonts (e.g. expo-font) and
        // falls back to Typeface.create() internally — it does not throw.
        return ReactFontManager.getInstance().getTypeface(family, style, context.assets)
    }

    private fun loadTextTypefaceFromAssets(
        assetManager: AssetManager,
        family: String,
        style: Int,
    ): Typeface? {
        val cacheKey = "$family:$style"
        textTypefaceCache[cacheKey]?.let { return it }

        val candidates = textFontAssetCandidates(family, style)
        for (dir in TEXT_FONT_ASSET_DIRS) {
            for (candidate in candidates) {
                for (extension in TEXT_FONT_FILE_EXTENSIONS) {
                    val path = "$dir/${candidate.baseName}$extension"
                    loadTypefaceFromAssetPath(assetManager, path, style, candidate.exactStyle)?.let {
                        textTypefaceCache[cacheKey] = it
                        return it
                    }
                }
            }
        }

        for (dir in TEXT_FONT_ASSET_DIRS) {
            val files = try {
                assetManager.list(dir)?.toList().orEmpty()
            } catch (_: Exception) {
                emptyList()
            }
            if (files.isEmpty()) continue

            for (candidate in candidates) {
                val normalizedCandidate = normalizeFontAssetName(candidate.baseName)
                val compactCandidate = compactFontAssetName(candidate.baseName)
                val file = files.firstOrNull { assetName ->
                    val extension = TEXT_FONT_FILE_EXTENSIONS.firstOrNull {
                        assetName.endsWith(it, ignoreCase = true)
                    }
                    if (extension == null) {
                        false
                    } else {
                        val assetBaseName = assetName.dropLast(extension.length)
                        assetBaseName.equals(candidate.baseName, ignoreCase = true) ||
                            (normalizedCandidate.isNotEmpty() &&
                                normalizeFontAssetName(assetBaseName) == normalizedCandidate) ||
                            (compactCandidate.isNotEmpty() &&
                                compactFontAssetName(assetBaseName) == compactCandidate)
                    }
                } ?: continue

                loadTypefaceFromAssetPath(assetManager, "$dir/$file", style, candidate.exactStyle)?.let {
                    textTypefaceCache[cacheKey] = it
                    return it
                }
            }
        }
        return null
    }

    private fun loadTypefaceFromAssetPath(
        assetManager: AssetManager,
        path: String,
        style: Int,
        exactStyle: Boolean,
    ): Typeface? = try {
        val typeface = Typeface.createFromAsset(assetManager, path)
        if (exactStyle || style == Typeface.NORMAL) {
            typeface
        } else {
            Typeface.create(typeface, style)
        }
    } catch (_: RuntimeException) {
        null
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
                        builder.append("\$${segment.content}\$")
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
        private val TEXT_FONT_ASSET_DIRS = listOf("font", "fonts")
        private val TEXT_FONT_FILE_EXTENSIONS = listOf(".ttf", ".otf")
        // Indices map to Android Typeface style constants: NORMAL=0, BOLD=1, ITALIC=2, BOLD_ITALIC=3.
        private val TEXT_FONT_STYLE_SUFFIXES = listOf("", "_bold", "_italic", "_bold_italic")
        private val textTypefaceCache = ConcurrentHashMap<String, Typeface>()
        private val NON_FONT_ASSET_CHARS = Regex("[^a-z0-9]+")

        private data class TextFontAssetCandidate(
            val baseName: String,
            val exactStyle: Boolean,
        )

        private fun textFontAssetCandidates(
            family: String,
            style: Int,
        ): List<TextFontAssetCandidate> {
            val suffix = TEXT_FONT_STYLE_SUFFIXES.getOrElse(style) { "" }
            val baseNames = textFontBaseNameVariants(family)
            val candidates = mutableListOf<TextFontAssetCandidate>()
            if (suffix.isNotEmpty()) {
                baseNames.forEach { candidates += TextFontAssetCandidate("$it$suffix", true) }
            }
            baseNames.forEach { candidates += TextFontAssetCandidate(it, suffix.isEmpty()) }
            return candidates.distinctBy { "${it.baseName}:${it.exactStyle}" }
        }

        private fun textFontBaseNameVariants(family: String): List<String> {
            val trimmed = family.trim()
            val underscored = trimmed.replace('-', '_').replace(' ', '_')
            val androidResourceName = normalizeFontAssetName(trimmed)
            return listOf(trimmed, underscored, androidResourceName)
                .filter { it.isNotEmpty() }
                .distinct()
        }

        private fun normalizeFontAssetName(value: String): String =
            NON_FONT_ASSET_CHARS
                .replace(value.trim().lowercase(Locale.US), "_")
                .trim('_')

        private fun compactFontAssetName(value: String): String =
            NON_FONT_ASSET_CHARS.replace(value.trim().lowercase(Locale.US), "")

        fun parseContent(content: String): List<Segment> {
            val segments = mutableListOf<Segment>()
            val current = StringBuilder()
            var inFormula = false
            var index = 0

            while (index < content.length) {
                val ch = content[index]
                if (ch == '\\' && index + 1 < content.length && content[index + 1] == '$') {
                    if (inFormula) {
                        current.append("\\$")
                    } else {
                        current.append('$')
                    }
                    index += 2
                    continue
                }

                if (ch == '$') {
                    if (inFormula) {
                        if (current.isNotEmpty()) {
                            segments.add(Segment.Formula(current.toString()))
                        } else {
                            segments.add(Segment.Text("\$\$"))
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
                index += 1
            }

            if (inFormula) {
                segments.add(Segment.Text("\$$current"))
            } else if (current.isNotEmpty()) {
                segments.add(Segment.Text(current.toString()))
            }

            return segments
        }
    }
}
