// RaTeXInlineViewManager.kt (New Architecture) — Codegen-generated interface.

package io.ratex

import android.graphics.Color
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.module.annotations.ReactModule
import com.facebook.react.uimanager.SimpleViewManager
import com.facebook.react.uimanager.ThemedReactContext
import com.facebook.react.uimanager.UIManagerHelper
import com.facebook.react.uimanager.annotations.ReactProp
import com.facebook.react.viewmanagers.RaTeXInlineViewManagerDelegate
import com.facebook.react.viewmanagers.RaTeXInlineViewManagerInterface

@ReactModule(name = RaTeXInlineViewManager.NAME)
class RaTeXInlineViewManager(private val reactContext: ReactApplicationContext) :
    SimpleViewManager<RaTeXInlineView>(),
    RaTeXInlineViewManagerInterface<RaTeXInlineView> {

    companion object {
        const val NAME = "RaTeXInlineView"
    }

    private val delegate = RaTeXInlineViewManagerDelegate(this)

    override fun getDelegate() = delegate

    override fun getName(): String = NAME

    override fun createViewInstance(ctx: ThemedReactContext): RaTeXInlineView {
        val view = RaTeXInlineView(ctx)
        view.onContentSizeChange = { width, height ->
            val dispatcher = UIManagerHelper.getEventDispatcherForReactTag(ctx, view.id)
            val surfaceId = UIManagerHelper.getSurfaceId(ctx)
            dispatcher?.dispatchEvent(
                RaTeXContentSizeEvent(surfaceId, view.id, width, height)
            )
        }
        return view
    }

    @ReactProp(name = "content")
    override fun setContent(view: RaTeXInlineView, value: String?) {
        view.content = value ?: ""
    }

    @ReactProp(name = "fontSize", defaultFloat = 16f)
    override fun setFontSize(view: RaTeXInlineView, value: Float) {
        view.formulaFontSize = value
    }

    @ReactProp(name = "color", customType = "Color")
    override fun setColor(view: RaTeXInlineView, value: Int?) {
        view.formulaColor = value ?: Color.BLACK
    }

    @ReactProp(name = "textColor", customType = "Color")
    override fun setTextColor(view: RaTeXInlineView, value: Int?) {
        view.inlineTextColor = value ?: Color.BLACK
    }

    @ReactProp(name = "textFontSize", defaultFloat = 16f)
    override fun setTextFontSize(view: RaTeXInlineView, value: Float) {
        view.textFontSize = value
    }

    @ReactProp(name = "textFontFamily")
    override fun setTextFontFamily(view: RaTeXInlineView, value: String?) {
        view.textFontFamily = value
    }

    @ReactProp(name = "textItalic", defaultBoolean = false)
    override fun setTextItalic(view: RaTeXInlineView, value: Boolean) {
        view.textItalic = value
    }

    @ReactProp(name = "textUnderline", defaultBoolean = false)
    override fun setTextUnderline(view: RaTeXInlineView, value: Boolean) {
        view.textUnderline = value
    }

    @ReactProp(name = "textLineThrough", defaultBoolean = false)
    override fun setTextLineThrough(view: RaTeXInlineView, value: Boolean) {
        view.textLineThrough = value
    }
}
