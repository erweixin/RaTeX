// RaTeXEngine.kt — Kotlin JNI wrapper around libratex_ffi.so

package io.ratex

import android.graphics.Color
import androidx.annotation.ColorInt
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

// MARK: - Error type

class RaTeXException(message: String) : Exception(message)

// MARK: - Engine

/**
 * Entry point for RaTeX rendering on Android.
 *
 * ```kotlin
 * val displayList = RaTeXEngine.parse("""\frac{-b \pm \sqrt{b^2-4ac}}{2a}""")
 * ```
 *
 * Note: [parse] is a suspend function; call it from a coroutine.
 * For one-shot calls from non-coroutine code, use [parseBlocking].
 */
object RaTeXEngine {

    // MARK: - Parse cache
    //
    // Parsing is deterministic in (latex, displayMode, color) — the DisplayList is in
    // em units, so fontSize is applied later by RaTeXRenderer and is NOT part of the
    // key. The cache lets the Fabric measure pass (background layout thread) and the
    // view's render share one parse, and lets RaTeXView swap its renderer
    // synchronously on the main thread without ever parsing there.
    //
    // Sizing: entries are a few KB. The cap is generous because eviction between a
    // view's measure() and its setLatex (e.g. a batch re-committing many formulas)
    // would silently drop that view back to the async render path.

    private data class ParseKey(val latex: String, val displayMode: Boolean, val colorArgb: Int)

    private const val CACHE_MAX_ENTRIES = 128

    private val cacheLock = Any()
    private val cache = object : LinkedHashMap<ParseKey, DisplayList>(64, 0.75f, true) {
        override fun removeEldestEntry(eldest: MutableMap.MutableEntry<ParseKey, DisplayList>) =
            size > CACHE_MAX_ENTRIES
    }

    /**
     * Pure cache lookup — never parses. Safe and cheap on the main thread.
     * @return the cached [DisplayList], or null if this input has not been parsed yet.
     */
    fun lookupCached(
        latex: String,
        displayMode: Boolean = true,
        @ColorInt color: Int = Color.BLACK,
    ): DisplayList? = synchronized(cacheLock) { cache[ParseKey(latex, displayMode, color)] }

    /**
     * Like [parseBlocking], but consults the cache first and stores the result.
     * The parse itself runs outside the cache lock so a slow parse on one thread
     * never blocks [lookupCached] on another; racing duplicate parses of the same
     * key are acceptable (the output is identical, last put wins).
     */
    fun parseCached(
        latex: String,
        displayMode: Boolean = true,
        @ColorInt color: Int = Color.BLACK,
    ): DisplayList {
        lookupCached(latex, displayMode, color)?.let { return it }
        val displayList = parseBlocking(latex, displayMode, color)
        synchronized(cacheLock) { cache[ParseKey(latex, displayMode, color)] = displayList }
        return displayList
    }

    private fun rgbaFloatArray(@ColorInt color: Int): FloatArray = floatArrayOf(
        ((color ushr 16) and 0xff) / 255f,
        ((color ushr 8) and 0xff) / 255f,
        (color and 0xff) / 255f,
        ((color ushr 24) and 0xff) / 255f,
    )

    init {
        System.loadLibrary("ratex_ffi")
    }

    // -------------------------------------------------------------------------
    // JNI declarations (implemented in crates/ratex-ffi/src/jni.rs)
    // -------------------------------------------------------------------------

    /**
     * Parse and lay out a LaTeX string with explicit display mode.
     * @param displayMode true = display/block style, false = inline/text style.
     * @return JSON DisplayList string on success, or null on error.
     */
    @JvmStatic
    private external fun nativeParseAndLayout(
        latex: String,
        displayMode: Boolean,
        color: FloatArray,
    ): String?

    /**
     * Retrieve the last error message produced by a native layout call on this thread.
     */
    @JvmStatic private external fun nativeGetLastError(): String?

    // -------------------------------------------------------------------------
    // Public API
    // -------------------------------------------------------------------------

    /**
     * Parse [latex] and return a [DisplayList] decoded from the JSON result.
     * Runs on [Dispatchers.Default].
     *
     * @param displayMode `true` (default) for display/block style; `false` for inline/text style.
     * @throws RaTeXException on parse or decode error.
     */
    suspend fun parse(
        latex: String,
        displayMode: Boolean = true,
        @ColorInt color: Int = Color.BLACK,
    ): DisplayList = withContext(Dispatchers.Default) { parseCached(latex, displayMode, color) }

    /**
     * Blocking variant of [parse]. Safe to call on any background thread.
     * **Do not call on the main thread** — use [parse] instead.
     *
     * @param displayMode `true` (default) for display/block style; `false` for inline/text style.
     * @throws RaTeXException on parse or decode error.
     */
    fun parseBlocking(
        latex: String,
        displayMode: Boolean = true,
        @ColorInt color: Int = Color.BLACK,
    ): DisplayList {
        val json = nativeParseAndLayout(
            latex = latex,
            displayMode = displayMode,
            color = rgbaFloatArray(color),
        )
            ?: throw RaTeXException(nativeGetLastError() ?: "unknown error")
        return try {
            ratexJson.decodeFromString(DisplayList.serializer(), json)
        } catch (e: Exception) {
            throw RaTeXException("JSON decode failed: ${e.message}")
        }
    }
}
