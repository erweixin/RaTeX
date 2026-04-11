// RaTeXNative.kt — JNA interface to libratex_ffi

package io.ratex

import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer
import com.sun.jna.Structure

/**
 * JNA mapping of `RatexOptions` from `ratex.h`.
 *
 * Always pass `struct_size = Structure.size(RatexOptions::class.java)`.
 */
@Structure.FieldOrder("struct_size", "display_mode")
class RatexOptions : Structure() {
    /** Must be set to the size of this struct. */
    @JvmField var struct_size: Long = Structure.size(RatexOptions::class.java).toLong()
    /**
     * Rendering mode:
     *   0 = inline / text style  ($...$)
     *   1 = display / block style ($$...$$)
     */
    @JvmField var display_mode: Int = 1
}

/**
 * JNA mapping of `RatexResult` from `ratex.h`.
 *
 * `data` is a heap-allocated JSON string on success (free with [RaTeXNative.ratex_free_display_list]);
 * null on error. `error_code` is 0 on success, non-zero on error.
 */
@Structure.FieldOrder("data", "error_code")
class RatexResult : Structure() {
    @JvmField var data: Pointer? = null
    @JvmField var error_code: Int = 0
}

/**
 * JNA mapping of the RaTeX C ABI (`ratex.h`).
 *
 * Functions:
 * - [ratex_parse_and_layout]: parse LaTeX → JSON DisplayList with explicit display/inline mode
 * - [ratex_free_display_list]: free the JSON string
 * - [ratex_get_last_error]: retrieve last thread-local error message
 */
internal interface RaTeXNative : Library {

    /**
     * Parse a LaTeX string with explicit display mode.
     * @param opts pointer to [RatexOptions]; pass null for display-mode defaults.
     * @return [RatexResult] — on success `data` is a JSON string pointer and `error_code` is 0.
     */
    fun ratex_parse_and_layout(latex: String, opts: RatexOptions?): RatexResult

    /**
     * Free a JSON string returned by [ratex_parse_and_layout].
     */
    fun ratex_free_display_list(json: Pointer?)

    /**
     * Return the last error message on this thread, or null.
     */
    fun ratex_get_last_error(): String?

    companion object {
        /** Default library name (without platform prefix/suffix). */
        const val LIBRARY_NAME = "ratex_ffi"

        /**
         * Load the native library. Searches in this order:
         * 1. `jna.library.path` system property
         * 2. `java.library.path` system property
         * 3. System library paths
         */
        val INSTANCE: RaTeXNative by lazy {
            Native.load(LIBRARY_NAME, RaTeXNative::class.java)
        }
    }
}
