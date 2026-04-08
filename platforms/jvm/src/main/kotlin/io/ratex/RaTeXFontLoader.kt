// RaTeXFontLoader.kt — Load KaTeX fonts for glyph rendering on the JVM.
//
// Fonts are loaded either from a filesystem directory or from classpath resources.
// Call loadFromDirectory() or loadFromResources() once at startup; RaTeXRenderer
// will then use getFont() to look up loaded fonts by ID.

package io.ratex

import java.awt.Font
import java.io.File
import java.io.InputStream
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicBoolean
import java.util.logging.Logger

object RaTeXFontLoader {

    private val log = Logger.getLogger(RaTeXFontLoader::class.java.name)
    private val fontsLoaded = AtomicBoolean(false)

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

    private val cache = ConcurrentHashMap<String, Font>()

    /**
     * Ensure KaTeX fonts are loaded from [directory]. No-op if already loaded.
     * @return Number of fonts loaded (0 if already loaded)
     */
    @JvmStatic
    fun ensureLoaded(directory: File): Int {
        if (fontsLoaded.get()) return 0
        synchronized(this) {
            if (fontsLoaded.get()) return 0
            return loadFromDirectory(directory).also { fontsLoaded.set(true) }
        }
    }

    /**
     * Load KaTeX fonts from a filesystem directory.
     * @param directory Directory containing KaTeX_*.ttf files
     * @return Number of fonts successfully loaded
     */
    @JvmStatic
    fun loadFromDirectory(directory: File): Int {
        var loaded = 0
        for ((fontId, fileName) in fontFileNames) {
            val file = File(directory, fileName)
            if (!file.exists()) continue
            try {
                val font = Font.createFont(Font.TRUETYPE_FONT, file)
                cache[fontId] = font
                loaded++
            } catch (e: Exception) {
                log.warning("Failed to load font $fileName from ${directory.path}: ${e.message}")
            }
        }
        return loaded
    }

    /**
     * Load KaTeX fonts from classpath resources.
     * @param resourcePrefix Classpath prefix, e.g. "/fonts" for /fonts/KaTeX_*.ttf
     * @return Number of fonts successfully loaded
     */
    @JvmStatic
    fun loadFromResources(resourcePrefix: String = "/fonts"): Int {
        val prefix = resourcePrefix.trimEnd('/')
        var loaded = 0
        for ((fontId, fileName) in fontFileNames) {
            val path = "$prefix/$fileName"
            val stream: InputStream = RaTeXFontLoader::class.java.getResourceAsStream(path) ?: continue
            try {
                val font = Font.createFont(Font.TRUETYPE_FONT, stream)
                cache[fontId] = font
                loaded++
            } catch (e: Exception) {
                log.warning("Failed to load font $fileName from resource $path: ${e.message}")
            } finally {
                stream.close()
            }
        }
        return loaded
    }

    /**
     * Get cached base Font for a font ID (e.g. "Main-Regular", "Math-Italic").
     * Returns null if not loaded.
     */
    @JvmStatic
    fun getFont(fontId: String): Font? = cache[fontId]

    /** Clear cache (e.g. for tests). */
    @JvmStatic
    fun clear() {
        cache.clear()
        fontsLoaded.set(false)
    }
}
