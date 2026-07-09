// RaTeXViewManager.kt (New Architecture) — implements Codegen-generated RaTeXViewManagerInterface.

package io.ratex

import android.content.Context
import android.graphics.Color
import com.facebook.react.bridge.ReactApplicationContext
import com.facebook.react.bridge.ReadableMap
import com.facebook.react.module.annotations.ReactModule
import com.facebook.react.uimanager.SimpleViewManager
import com.facebook.react.uimanager.ThemedReactContext
import com.facebook.react.uimanager.UIManagerHelper
import com.facebook.react.uimanager.annotations.ReactProp
import com.facebook.react.viewmanagers.RaTeXViewManagerDelegate
import com.facebook.react.viewmanagers.RaTeXViewManagerInterface
import com.facebook.yoga.YogaMeasureMode
import com.facebook.yoga.YogaMeasureOutput

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

    // Synchronous intrinsic measure for Fabric, invoked by the custom C++ shadow
    // node's measureContent via FabricUIManager.measure. Gives the view its real
    // size on the first commit (e.g. at JS useLayoutEffect) instead of only after
    // the async onContentSizeChange event. Parsing is thread-safe and fonts are not
    // needed for measurement. Color does not affect size, but it is part of the
    // parse-cache key — parsing with the view's actual color makes this entry
    // reusable by RaTeXView's synchronous render on the main thread.
    override fun measure(
        context: Context,
        localData: ReadableMap?,
        props: ReadableMap?,
        state: ReadableMap?,
        width: Float,
        widthMode: YogaMeasureMode?,
        height: Float,
        heightMode: YogaMeasureMode?,
        attachmentsPositions: FloatArray?,
    ): Long {
        val latex = props?.getString("latex").orEmpty()
        val fontSize =
            if (props?.hasKey("fontSize") == true) props.getDouble("fontSize").toFloat() else 24f
        if (latex.isBlank() || fontSize <= 0f) {
            return YogaMeasureOutput.make(0f, 0f)
        }
        val displayMode =
            if (props?.hasKey("displayMode") == true) props.getBoolean("displayMode") else true
        val color =
            if (props?.hasKey("color") == true && !props.isNull("color")) props.getInt("color")
            else Color.BLACK
        return try {
            val density = context.resources.displayMetrics.density
            val displayList = RaTeXEngine.parseCached(latex, displayMode, color)
            val renderer = RaTeXRenderer(displayList, fontSize * density)
            YogaMeasureOutput.make(renderer.widthPx / density, renderer.totalHeightPx / density)
        } catch (e: Throwable) {
            YogaMeasureOutput.make(0f, 0f)
        }
    }
}
