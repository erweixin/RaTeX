// RaTeXModuleImpl.kt — arch-independent body of the RaTeXModule TurboModule.

package io.ratex

import com.facebook.react.bridge.Arguments
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.WritableMap

/**
 * Shared body of the RaTeXModule TurboModule. Sync by design
 * (callers need it in useLayoutEffect); [RaTeXMetrics] is parse-cache-backed.
 */
object RaTeXModuleImpl {
    const val NAME = "RaTeXModule"

    fun getTexMetrics(
        context: ReactApplicationContext,
        latex: String,
        fontSize: Double,
        displayMode: Boolean,
        color: Double,
    ): WritableMap? {
        val density = context.resources.displayMetrics.density
        // Render color → one color-keyed cache entry shared with measure/render.
        val metrics =
            RaTeXMetrics.metrics(
                latex,
                fontSize.toFloat(),
                displayMode,
                density,
                color.toLong().toInt(),
            ) ?: return null
        return Arguments.createMap().apply {
            putDouble("width", metrics[0].toDouble())
            putDouble("height", metrics[1].toDouble())
            putDouble("depth", metrics[2].toDouble())
        }
    }
}
