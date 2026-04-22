// RaTeXViewManager.kt (Old Architecture) — SimpleViewManager for RaTeXView.

package io.ratex

import android.graphics.Color
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.WritableNativeMap
import com.facebook.react.uimanager.SimpleViewManager
import com.facebook.react.uimanager.ThemedReactContext
import com.facebook.react.uimanager.annotations.ReactProp
import com.facebook.react.uimanager.events.RCTEventEmitter

class RaTeXViewManager(private val reactContext: ReactApplicationContext) :
    SimpleViewManager<RaTeXView>() {

    companion object {
        const val NAME = "RaTeXView"
    }

    override fun getName(): String = NAME

    override fun createViewInstance(ctx: ThemedReactContext): RaTeXView {
        val view = RaTeXView(ctx)
        view.onError = { exception ->
            val event = WritableNativeMap().apply {
                putString("error", exception.message ?: "unknown error")
            }
            ctx.getJSModule(RCTEventEmitter::class.java)
                .receiveEvent(view.id, "topError", event)
        }
        view.onContentSizeChange = { width, height ->
            val event = WritableNativeMap().apply {
                putDouble("width", width)
                putDouble("height", height)
            }
            ctx.getJSModule(RCTEventEmitter::class.java)
                .receiveEvent(view.id, "topContentSizeChange", event)
        }
        return view
    }

    @ReactProp(name = "latex")
    fun setLatex(view: RaTeXView, value: String?) {
        view.latex = value ?: ""
    }

    @ReactProp(name = "fontSize", defaultFloat = 24f)
    fun setFontSize(view: RaTeXView, value: Float) {
        view.fontSize = value
    }

    @ReactProp(name = "displayMode", defaultBoolean = true)
    fun setDisplayMode(view: RaTeXView, value: Boolean) {
        view.displayMode = value
    }

    @ReactProp(name = "color", customType = "Color")
    fun setColor(view: RaTeXView, value: Int?) {
        view.color = value ?: Color.BLACK
    }

    override fun getExportedCustomDirectEventTypeConstants(): Map<String, Any> =
        mapOf(
            "topError" to mapOf("registrationName" to "onError"),
            "topContentSizeChange" to mapOf("registrationName" to "onContentSizeChange"),
        )
}
