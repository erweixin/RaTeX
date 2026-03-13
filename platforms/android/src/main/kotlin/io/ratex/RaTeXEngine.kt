// RaTeXEngine.kt — Kotlin JNI wrapper around libratex_ffi.so

package io.ratex

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

    init {
        System.loadLibrary("ratex_ffi")
    }

    // -------------------------------------------------------------------------
    // JNI declarations (implemented in crates/ratex-ffi/src/jni.rs)
    // -------------------------------------------------------------------------

    /**
     * Parse and lay out a LaTeX string.
     * @return JSON DisplayList string on success, or null on error.
     */
    @JvmStatic private external fun nativeParseAndLayout(latex: String): String?

    /**
     * Retrieve the last error message produced by nativeParseAndLayout on this thread.
     */
    @JvmStatic private external fun nativeGetLastError(): String?

    // -------------------------------------------------------------------------
    // Public API
    // -------------------------------------------------------------------------

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
     * Blocking variant of [parse]. Safe to call on any background thread.
     * **Do not call on the main thread** — use [parse] instead.
     *
     * @throws RaTeXException on parse or decode error.
     */
    fun parseBlocking(latex: String): DisplayList {
        val json = nativeParseAndLayout(latex)
            ?: throw RaTeXException(nativeGetLastError() ?: "unknown error")
        return try {
            ratexJson.decodeFromString(DisplayList.serializer(), json)
        } catch (e: Exception) {
            throw RaTeXException("JSON decode failed: ${e.message}")
        }
    }
}
