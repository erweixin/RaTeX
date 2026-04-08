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
     * @throws RaTeXException on parse or decode error.
     */
    suspend fun parse(latex: String): DisplayList = withContext(Dispatchers.Default) {
        parseBlocking(latex)
    }

    /**
     * Blocking variant of [parse]. Safe to call on any thread.
     *
     * @throws RaTeXException on parse or decode error.
     */
    fun parseBlocking(latex: String): DisplayList {
        val ptr = native.ratex_parse_and_layout(latex)
            ?: throw RaTeXException(native.ratex_get_last_error() ?: "unknown error")
        val json: String
        try {
            json = ptr.getString(0, "UTF-8")
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
