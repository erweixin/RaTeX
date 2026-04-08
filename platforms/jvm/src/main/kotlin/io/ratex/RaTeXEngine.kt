// RaTeXEngine.kt — JVM wrapper around libratex_ffi via JNA

package io.ratex

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

// MARK: - Error type

class RaTeXException(message: String) : Exception(message)

// MARK: - Engine

/**
 * Entry point for RaTeX rendering on the JVM.
 *
 * ```kotlin
 * val displayList = RaTeXEngine.parse("""\frac{-b \pm \sqrt{b^2-4ac}}{2a}""")
 * ```
 *
 * Note: [parse] is a suspend function; call it from a coroutine.
 * For one-shot calls from non-coroutine code, use [parseBlocking].
 */
object RaTeXEngine {

    private val native: RaTeXNative = RaTeXNative.INSTANCE

    /**
     * Parse [latex] and return a [DisplayList] decoded from the JSON result.
     * Runs on [Dispatchers.Default].
     *
     * @param displayMode `true` (default) for display/block style; `false` for inline/text style.
     * @throws RaTeXException on parse or decode error.
     */
    suspend fun parse(latex: String, displayMode: Boolean = true): DisplayList =
        withContext(Dispatchers.Default) { parseBlocking(latex, displayMode) }

    /**
     * Blocking variant of [parse]. Safe to call on any thread.
     *
     * @param displayMode `true` (default) for display/block style; `false` for inline/text style.
     * @throws RaTeXException on parse or decode error.
     */
    fun parseBlocking(latex: String, displayMode: Boolean = true): DisplayList {
        val opts = RatexOptions().also { it.display_mode = if (displayMode) 1 else 0 }
        val result = native.ratex_parse_and_layout(latex, opts)
        val ptr = if (result.error_code == 0) result.data
            else throw RaTeXException(native.ratex_get_last_error() ?: "unknown error")
        val json: String
        try {
            json = ptr!!.getString(0, "UTF-8")
        } finally {
            native.ratex_free_display_list(ptr)
        }
        return try {
            ratexJson.decodeFromString(DisplayList.serializer(), json)
        } catch (e: Exception) {
            throw RaTeXException("JSON decode failed: ${e.message}")
        }
    }
}
