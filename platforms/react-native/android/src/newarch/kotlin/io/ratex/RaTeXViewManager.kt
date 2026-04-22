// RaTeXViewManager.kt (New Architecture) — implements Codegen-generated RaTeXViewManagerInterface.

package io.ratex

import android.graphics.Color
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.module.annotations.ReactModule
import com.facebook.react.uimanager.SimpleViewManager
import com.facebook.react.uimanager.ThemedReactContext
import com.facebook.react.uimanager.UIManagerHelper
import com.facebook.react.uimanager.annotations.ReactProp
import com.facebook.react.viewmanagers.RaTeXViewManagerDelegate
import com.facebook.react.viewmanagers.RaTeXViewManagerInterface

@ReactModule(name = RaTeXViewManager.NAME)
class RaTeXViewManager(private val reactContext: ReactApplicationContext) :
    SimpleViewManager<RaTeXView>(),
    RaTeXViewManagerInterface<RaTeXView> {

    companion object {
        const val NAME = "RaTeXView"
    }

    private val delegate = RaTeXViewManagerDelegate(this)

    override fun getDelegate() = delegate

    override fun getName(): String = NAME

    override fun createViewInstance(ctx: ThemedReactContext): RaTeXView {
        val view = RaTeXView(ctx)
        view.onError = { exception ->
            val dispatcher = UIManagerHelper.getEventDispatcherForReactTag(ctx, view.id)
            val surfaceId = UIManagerHelper.getSurfaceId(ctx)
            dispatcher?.dispatchEvent(
                RaTeXErrorEvent(surfaceId, view.id, exception.message ?: "unknown error")
            )
        }
        view.onContentSizeChange = { width, height ->
            val dispatcher = UIManagerHelper.getEventDispatcherForReactTag(ctx, view.id)
            val surfaceId = UIManagerHelper.getSurfaceId(ctx)
            dispatcher?.dispatchEvent(
                RaTeXContentSizeEvent(surfaceId, view.id, width, height)
            )
        }
        return view
    }

    @ReactProp(name = "latex")
    override fun setLatex(view: RaTeXView, value: String?) {
        view.latex = value ?: ""
    }

    @ReactProp(name = "fontSize", defaultFloat = 24f)
    override fun setFontSize(view: RaTeXView, value: Float) {
        view.fontSize = value
    }

    @ReactProp(name = "displayMode", defaultBoolean = true)
    override fun setDisplayMode(view: RaTeXView, value: Boolean) {
        view.displayMode = value
    }

    @ReactProp(name = "color", customType = "Color")
    override fun setColor(view: RaTeXView, value: Int?) {
        view.color = value ?: Color.BLACK
    }
}
