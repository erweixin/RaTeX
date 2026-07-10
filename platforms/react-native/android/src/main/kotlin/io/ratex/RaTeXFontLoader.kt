// RaTeXFontLoader.kt — Load KaTeX fonts for glyph rendering (mirrors iOS RaTeXFontLoader).
//
// GlyphPath from Rust contains placeholder rectangles, not outline paths. Glyphs are drawn
// with Typeface + Canvas.drawText. RaTeXView calls ensureLoaded() on first use so fonts load
// automatically from assets/fonts; optionally call loadFromAssets() at startup to load earlier.

package io.ratex

import android.content.Context
import android.graphics.Typeface
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicBoolean

object RaTeXFontLoader {

    private val fontsLoaded = AtomicBoolean(false)
    private val loadLock = Any()

    /** Whether fonts have been loaded (cheap atomic read; safe on the main thread). */
    @JvmStatic
    val isLoaded: Boolean get() = fontsLoaded.get()

    /** KaTeX font IDs (Rust FontId.as_str()) → TTF filename without path. */
    private val fontFileNames = listOf(
        "AMS-Regular" to "KaTeX_AMS-Regular.ttf",
        "Caligraphic-Regular" to "KaTeX_Caligraphic-Regular.ttf",
        "Fraktur-Regular" to "KaTeX_Fraktur-Regular.ttf",
        "Fraktur-Bold" to "KaTeX_Fraktur-Bold.ttf",
        "Main-Bold" to "KaTeX_Main-Bold.ttf",
        "Main-BoldItalic" to "KaTeX_Main-BoldItalic.ttf",
        "Main-Italic" to "KaTeX_Main-Italic.ttf",
        "Main-Regular" to "KaTeX_Main-Regular.ttf",
        "Math-BoldItalic" to "KaTeX_Math-BoldItalic.ttf",
        "Math-Italic" to "KaTeX_Math-Italic.ttf",
        "SansSerif-Bold" to "KaTeX_SansSerif-Bold.ttf",
        "SansSerif-Italic" to "KaTeX_SansSerif-Italic.ttf",
        "SansSerif-Regular" to "KaTeX_SansSerif-Regular.ttf",
        "Script-Regular" to "KaTeX_Script-Regular.ttf",
        "Size1-Regular" to "KaTeX_Size1-Regular.ttf",
        "Size2-Regular" to "KaTeX_Size2-Regular.ttf",
        "Size3-Regular" to "KaTeX_Size3-Regular.ttf",
        "Size4-Regular" to "KaTeX_Size4-Regular.ttf",
        "Typewriter-Regular" to "KaTeX_Typewriter-Regular.ttf",
    )

    private val cache = ConcurrentHashMap<String, Typeface>()

    /**
     * Ensure KaTeX fonts are loaded; if not, load from [assetPath]. Call this on first use
     * if you did not call [loadFromAssets] at startup. No-op if already loaded.
     * @param assetPath Path under assets, e.g. "fonts" for assets/fonts/KaTeX_*.ttf
     * @return Number of fonts loaded (0 if already loaded)
     */
    @JvmStatic
    fun ensureLoaded(context: Context, assetPath: String = "fonts"): Int {
        if (fontsLoaded.get()) return 0
        synchronized(loadLock) {
            if (fontsLoaded.get()) return 0
            return loadFromAssets(context, assetPath).also { fontsLoaded.set(true) }
        }
    }

    /**
     * Load KaTeX fonts from app assets. Optional: call once at startup to load early;
     * otherwise [RaTeXView] will load on first use via [ensureLoaded].
     * @param assetPath Path under assets, e.g. "fonts" for assets/fonts/KaTeX_*.ttf
     * @return Number of fonts successfully loaded
     */
    @JvmStatic
    fun loadFromAssets(context: Context, assetPath: String = "fonts"): Int {
        val prefix = assetPath.trimEnd('/')
        var loaded = 0
        for ((fontId, fileName) in fontFileNames) {
            val path = if (prefix.isEmpty()) fileName else "$prefix/$fileName"
            try {
                val typeface = Typeface.createFromAsset(context.assets, path)
                cache[fontId] = typeface
                loaded++
            } catch (_: Exception) {
                // Font file not present — skip
            }
        }
        return loaded
    }

    /**
     * Get cached Typeface for a font ID (e.g. "Main-Regular", "Math-Italic").
     * For CJK/emoji fallback IDs that have no bundled KaTeX font, resolves to the
     * system default Typeface which has broad Unicode coverage on all Android versions.
     * Returns null if not loaded and not a known fallback; Renderer will skip drawing.
     */
    @JvmStatic
    fun getTypeface(fontId: String): Typeface? {
        cache[fontId]?.let { return it }
        val systemFallback = resolveSystemFallback(fontId) ?: return null
        cache.putIfAbsent(fontId, systemFallback)
        return systemFallback
    }

    /**
     * Map CJK/emoji font IDs to [Typeface.DEFAULT], which provides system CJK + emoji
     * coverage on all Android versions (Noto / Roboto / HarmonyOS Sans / MiLan Pro).
     */
    private fun resolveSystemFallback(fontId: String): Typeface? = when (fontId) {
        "CJK-Regular", "CJK-Fallback", "Emoji-Fallback" -> Typeface.DEFAULT
        else -> null
    }

    /** Clear cache (e.g. for tests). */
    @JvmStatic
    fun clear() {
        cache.clear()
        fontsLoaded.set(false)
    }
}
