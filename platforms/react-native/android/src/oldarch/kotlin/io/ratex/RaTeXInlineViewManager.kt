// RaTeXInlineViewManager.kt (Old Architecture) — SimpleViewManager for RaTeXInlineView.

package io.ratex

import android.graphics.Color
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.WritableNativeMap
import com.facebook.react.uimanager.SimpleViewManager
import com.facebook.react.uimanager.ThemedReactContext
import com.facebook.react.uimanager.annotations.ReactProp
import com.facebook.react.uimanager.events.RCTEventEmitter

class RaTeXInlineViewManager(private val reactContext: ReactApplicationContext) :
    SimpleViewManager<RaTeXInlineView>() {

    companion object {
        const val NAME = "RaTeXInlineView"
    }

    override fun getName(): String = NAME

    override fun createViewInstance(ctx: ThemedReactContext): RaTeXInlineView {
        val view = RaTeXInlineView(ctx)
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

    @ReactProp(name = "content")
    fun setContent(view: RaTeXInlineView, value: String?) {
        view.content = value ?: ""
    }

    @ReactProp(name = "fontSize", defaultFloat = 16f)
    fun setFontSize(view: RaTeXInlineView, value: Float) {
        view.formulaFontSize = value
    }

    @ReactProp(name = "color", customType = "Color")
    fun setColor(view: RaTeXInlineView, value: Int?) {
        view.formulaColor = value ?: Color.BLACK
    }

    @ReactProp(name = "textColor", customType = "Color")
    fun setTextColor(view: RaTeXInlineView, value: Int?) {
        view.inlineTextColor = value ?: Color.BLACK
    }

    @ReactProp(name = "textFontSize", defaultFloat = 16f)
    fun setTextFontSize(view: RaTeXInlineView, value: Float) {
        view.textFontSize = value
    }

    @ReactProp(name = "textFontFamily")
    fun setTextFontFamily(view: RaTeXInlineView, value: String?) {
        view.textFontFamily = value
    }

    @ReactProp(name = "textItalic", defaultBoolean = false)
    fun setTextItalic(view: RaTeXInlineView, value: Boolean) {
        view.textItalic = value
    }

    @ReactProp(name = "textUnderline", defaultBoolean = false)
    fun setTextUnderline(view: RaTeXInlineView, value: Boolean) {
        view.textUnderline = value
    }

    @ReactProp(name = "textLineThrough", defaultBoolean = false)
    fun setTextLineThrough(view: RaTeXInlineView, value: Boolean) {
        view.textLineThrough = value
    }

    override fun getExportedCustomDirectEventTypeConstants(): Map<String, Any> =
        mapOf(
            "topContentSizeChange" to mapOf("registrationName" to "onContentSizeChange"),
        )
}
