/**
 * ratex.h — RaTeX C ABI public header
 *
 * Provides LaTeX-to-DisplayList rendering for iOS, Android, Flutter, and React Native.
 *
 * Usage:
 *   const char* json = ratex_parse_and_layout("\\frac{1}{2}");
 *   if (json) {
 *       // json is a UTF-8 JSON string describing the display list
 *       ratex_free_display_list(json);
 *   } else {
 *       const char* err = ratex_get_last_error();
 *       fprintf(stderr, "RaTeX error: %s\n", err ? err : "(unknown)");
 *   }
 *
 * Thread safety:
 *   ratex_parse_and_layout and ratex_get_last_error use thread-local storage for
 *   error state, so they are safe to call concurrently from multiple threads.
 *   Each thread has its own last-error slot.
 *
 * DisplayList JSON format:
 *   {
 *     "width":  <number>,   // total width in em units
 *     "height": <number>,   // ascent above baseline in em units
 *     "depth":  <number>,   // descent below baseline in em units
 *     "items":  [           // array of drawing commands (see below)
 *       { "GlyphPath": { "x": <f64>, "y": <f64>, "scale": <f64>,
 *                        "font": <string>, "char_code": <u32>,
 *                        "commands": [<PathCommand>, ...],
 *                        "color": {"r":<f32>,"g":<f32>,"b":<f32>,"a":<f32>} } },
 *       { "Line":      { "x": <f64>, "y": <f64>, "width": <f64>, "thickness": <f64>,
 *                        "color": {...} } },
 *       { "Rect":      { "x": <f64>, "y": <f64>, "width": <f64>, "height": <f64>,
 *                        "color": {...} } },
 *       { "Path":      { "x": <f64>, "y": <f64>, "commands": [...],
 *                        "fill": <bool>, "color": {...} } }
 *     ]
 *   }
 *
 * PathCommand variants:
 *   { "MoveTo": {"x":<f64>,"y":<f64>} }
 *   { "LineTo": {"x":<f64>,"y":<f64>} }
 *   { "CubicTo": {"x1":<f64>,"y1":<f64>,"x2":<f64>,"y2":<f64>,"x":<f64>,"y":<f64>} }
 *   { "QuadTo": {"x1":<f64>,"y1":<f64>,"x":<f64>,"y":<f64>} }
 *   { "Close": null }
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
 * Parse a LaTeX string and compute its display list as JSON.
 *
 * @param latex  Null-terminated UTF-8 LaTeX string. Must not be NULL.
 * @return       On success: a heap-allocated, null-terminated JSON string.
 *               The caller is responsible for freeing it with ratex_free_display_list().
 *               On error: NULL. Call ratex_get_last_error() for the reason.
 */
const char* ratex_parse_and_layout(const char* latex);

/**
 * Free a JSON string returned by ratex_parse_and_layout().
 *
 * @param json  Pointer returned by ratex_parse_and_layout(). Passing NULL is a no-op.
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
