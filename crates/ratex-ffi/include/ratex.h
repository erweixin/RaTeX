/**
 * ratex.h — RaTeX C ABI public header
 *
 * Provides LaTeX-to-DisplayList rendering for iOS, Android, Flutter, and React Native.
 *
 * Usage:
 *   RatexOptions opts = { sizeof(RatexOptions), 1 };  // display_mode=1 (block)
 *   RatexResult r = ratex_parse_and_layout("\\frac{1}{2}", &opts);
 *   if (r.error_code == 0) {
 *       // r.data is a heap-allocated UTF-8 JSON string
 *       ratex_free_display_list(r.data);
 *   } else {
 *       const char* err = ratex_get_last_error();
 *       fprintf(stderr, "RaTeX error: %s\n", err ? err : "(unknown)");
 *   }
 *
 * display_mode values:
 *   1 — display (block) style, equivalent to $$...$$
 *   0 — inline (text) style,   equivalent to $...$
 *
 * Thread safety:
 *   ratex_parse_and_layout and ratex_get_last_error use thread-local storage for
 *   error state, so they are safe to call concurrently from multiple threads.
 *   Each thread has its own last-error slot.
 *
 * DisplayList JSON format:
 *   {
 *     "version": 1,         // protocol version (optional; decoders may treat missing as 0)
 *     "width":  <number>,   // total width in em units
 *     "height": <number>,   // ascent above baseline in em units
 *     "depth":  <number>,   // descent below baseline in em units
 *     "items":  [           // array of drawing commands (see below)
 *       { "type": "GlyphPath", "x": <f64>, "y": <f64>, "scale": <f64>,
 *         "font": <string>, "char_code": <u32>, "color": {"r":<f32>,"g":<f32>,"b":<f32>,"a":<f32>} },
 *       { "type": "Line", "x": <f64>, "y": <f64>, "width": <f64>, "thickness": <f64>,
 *         "color": {...}, "dashed": <bool?> },
 *       { "type": "Rect", "x": <f64>, "y": <f64>, "width": <f64>, "height": <f64>,
 *         "color": {...} },
 *       { "type": "Path", "x": <f64>, "y": <f64>, "commands": [<PathCommand>, ...],
 *         "fill": <bool>, "color": {...} }
 *     ]
 *   }
 *
 * PathCommand variants:
 *   { "type": "MoveTo",  "x":<f64>,"y":<f64> }
 *   { "type": "LineTo",  "x":<f64>,"y":<f64> }
 *   { "type": "CubicTo", "x1":<f64>,"y1":<f64>,"x2":<f64>,"y2":<f64>,"x":<f64>,"y":<f64> }
 *   { "type": "QuadTo",  "x1":<f64>,"y1":<f64>,"x":<f64>,"y":<f64> }
 *   { "type": "Close" }
 *
 * Protocol and compatibility:
 *   Treat this JSON as a public protocol. Decoders should ignore unknown fields
 *   and tolerate missing optional fields for forward/backward compatibility.
 *   See docs/DISPLAYLIST_JSON_PROTOCOL.md for the full schema and change policy.
 *
 * Coordinate system:
 *   All coordinates are in em units. Multiply by font_size (pt or px) to get
 *   screen coordinates. X increases rightward; Y increases downward. The baseline
 *   is at y = height (measured from the top of the bounding box).
 */

#ifndef RATEX_H
#define RATEX_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stddef.h>

/**
 * Options for ratex_parse_and_layout().
 *
 * Always set struct_size = sizeof(RatexOptions) before use.
 * Fields beyond struct_size are ignored, allowing forward compatibility.
 */
typedef struct {
    size_t struct_size;
    int display_mode; /* 0 = inline ($...$), 1 = display block ($$...$$) */
} RatexOptions;

/**
 * Result returned by ratex_parse_and_layout().
 *
 * On success: error_code == 0 and data is a heap-allocated JSON string;
 * free it with ratex_free_display_list().
 * On error: error_code != 0, data is NULL; call ratex_get_last_error() for details.
 */
typedef struct {
    char* data;      /* JSON display list on success, NULL on error */
    int error_code;  /* 0 on success, non-zero on error */
} RatexResult;

/**
 * Parse a LaTeX string and compute its display list.
 *
 * @param latex  Null-terminated UTF-8 LaTeX string. Must not be NULL.
 * @param opts   Pointer to RatexOptions with struct_size set. May be NULL (defaults to display mode).
 * @return       RatexResult. On success error_code == 0 and data is a JSON string
 *               (free with ratex_free_display_list). On error error_code != 0 and data is NULL.
 */
RatexResult ratex_parse_and_layout(const char* latex, const RatexOptions* opts);

/**
 * Free a JSON string returned by ratex_parse_and_layout().
 *
 * @param json  Pointer to free. Passing NULL is a no-op.
 */
void ratex_free_display_list(char* json);

/**
 * Return the last error message produced by ratex_parse_and_layout() on this thread.
 *
 * @return  A null-terminated UTF-8 error string, or NULL if no error has occurred.
 *          The pointer is valid until the next call to ratex_parse_and_layout() on
 *          this thread. Do NOT free this pointer.
 */
const char* ratex_get_last_error(void);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* RATEX_H */
