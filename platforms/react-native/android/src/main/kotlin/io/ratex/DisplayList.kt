// DisplayList.kt — Kotlin mirror of ratex-types DisplayList / DisplayItem
// Decoded from the JSON returned by ratex_parse_and_layout via kotlinx.serialization.

package io.ratex

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonClassDiscriminator

// MARK: - Top-level output

@Serializable
data class DisplayList(
    val width: Double,
    val height: Double,
    val depth: Double,
    val items: List<DisplayItem>,
)

// MARK: - Drawing commands (flat structure matching Rust serde tag = "type")

@Serializable
sealed class DisplayItem {
    @Serializable @SerialName("GlyphPath")
    data class GlyphPath(
        val x: Double,
        val y: Double,
        val scale: Double,
        val font: String,
        @SerialName("char_code") val charCode: Int,
        val commands: List<PathCommand> = emptyList(),
        val color: RaTeXColor,
    ) : DisplayItem()

    @Serializable @SerialName("Line")
    data class Line(
        val x: Double,
        val y: Double,
        val width: Double,
        val thickness: Double,
        val color: RaTeXColor,
        val dashed: Boolean = false,
    ) : DisplayItem()

    @Serializable @SerialName("Rect")
    data class Rect(
        val x: Double,
        val y: Double,
        val width: Double,
        val height: Double,
        val color: RaTeXColor,
    ) : DisplayItem()

    @Serializable @SerialName("Path")
    data class Path(
        val x: Double,
        val y: Double,
        val commands: List<PathCommand>,
        val fill: Boolean,
        val color: RaTeXColor,
    ) : DisplayItem()
}

// MARK: - Path commands

@Serializable
sealed class PathCommand {
    @Serializable @SerialName("MoveTo")  data class MoveTo(val x: Double, val y: Double) : PathCommand()
    @Serializable @SerialName("LineTo")  data class LineTo(val x: Double, val y: Double) : PathCommand()
    @Serializable @SerialName("CubicTo") data class CubicTo(
        val x1: Double, val y1: Double,
        val x2: Double, val y2: Double,
        val x: Double,  val y: Double,
    ) : PathCommand()
    @Serializable @SerialName("QuadTo")  data class QuadTo(
        val x1: Double, val y1: Double,
        val x: Double,  val y: Double,
    ) : PathCommand()
    @Serializable @SerialName("Close")   data object Close : PathCommand()
}

// MARK: - Color

@Serializable
data class RaTeXColor(
    val r: Float,
    val g: Float,
    val b: Float,
    val a: Float,
) {
    companion object {
        val Black = RaTeXColor(r = 0f, g = 0f, b = 0f, a = 1f)
    }

    /** Convert to an Android ARGB int (for use with Paint.color, Canvas, etc.) */
    fun toArgb(): Int {
        val ai = (a * 255).toInt().coerceIn(0, 255)
        val ri = (r * 255).toInt().coerceIn(0, 255)
        val gi = (g * 255).toInt().coerceIn(0, 255)
        val bi = (b * 255).toInt().coerceIn(0, 255)
        return (ai shl 24) or (ri shl 16) or (gi shl 8) or bi
    }
}

// MARK: - JSON decoder (shared instance)

internal val ratexJson = Json {
    ignoreUnknownKeys = true
    classDiscriminator = "type"
}
