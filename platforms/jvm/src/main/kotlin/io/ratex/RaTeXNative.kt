// RaTeXNative.kt — JNA interface to libratex_ffi

package io.ratex

import com.sun.jna.Library
import com.sun.jna.Native
import com.sun.jna.Pointer

/**
 * JNA mapping of the RaTeX C ABI (`ratex.h`).
 *
 * Functions:
 * - [ratex_parse_and_layout]: parse LaTeX → JSON DisplayList (caller must free with [ratex_free_display_list])
 * - [ratex_free_display_list]: free the JSON string
 * - [ratex_get_last_error]: retrieve last thread-local error message
 */
internal interface RaTeXNative : Library {

    /**
     * Parse a LaTeX string and compute its display list as JSON.
     * @return heap-allocated JSON string pointer, or null on error.
     */
    fun ratex_parse_and_layout(latex: String): Pointer?

    /**
     * Free a JSON string returned by [ratex_parse_and_layout].
     */
    fun ratex_free_display_list(json: Pointer?)

    /**
     * Return the last error message on this thread, or null.
     * The returned string is valid until the next call to [ratex_parse_and_layout].
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
         *
         * Set `jna.library.path` to point to the directory containing the native library
         * if it is not in a standard location.
         */
        val INSTANCE: RaTeXNative by lazy {
            Native.load(LIBRARY_NAME, RaTeXNative::class.java)
        }
    }
}
