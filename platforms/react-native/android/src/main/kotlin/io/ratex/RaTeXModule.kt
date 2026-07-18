// RaTeXModule.kt (New Architecture) — TurboModule over the codegen spec.

package io.ratex

import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.WritableMap
import com.facebook.react.module.annotations.ReactModule

@ReactModule(name = RaTeXModuleImpl.NAME)
class RaTeXModule(reactContext: ReactApplicationContext) :
    NativeRaTeXModuleSpec(reactContext) {

    override fun getTexMetrics(
        latex: String,
        fontSize: Double,
        displayMode: Boolean,
        color: Double,
    ): WritableMap? =
        RaTeXModuleImpl.getTexMetrics(reactApplicationContext, latex, fontSize, displayMode, color)
}
