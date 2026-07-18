// RaTeXContentSizeEvent.kt — Fabric event for RaTeXView onContentSizeChange.

package io.ratex

import com.facebook.react.bridge.Arguments
import com.facebook.react.uimanager.events.Event

class RaTeXContentSizeEvent(
    surfaceId: Int,
    viewId: Int,
    private val widthDp: Double,
    private val heightDp: Double,
) : Event<RaTeXContentSizeEvent>(surfaceId, viewId) {

    override fun getEventName(): String = EVENT_NAME

    override fun getEventData() = Arguments.createMap().apply {
        putDouble("width", widthDp)
        putDouble("height", heightDp)
    }

    companion object {
        const val EVENT_NAME = "topContentSizeChange"
    }
}
