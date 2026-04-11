//! RaTeX C ABI FFI exports for native platform integration.
//!
//! Platform-specific modules:
//! - `jni` — Android JNI bridge (compiled only on `target_os = "android"`)
//!
//! ## DisplayList JSON protocol
//!
//! The primary output of this crate is a UTF-8 JSON string representing a `DisplayList`.
//! Treat this JSON as a **public protocol**: decoders should ignore unknown fields and
//! tolerate missing optional fields for forward/backward compatibility.
//!
//! See `docs/DISPLAYLIST_JSON_PROTOCOL.md` in the repository for the full schema and
//! change policy.
//!
//! # Usage (C)
//! ```c
//! RatexOptions opts = { sizeof(RatexOptions), 1 };  // display_mode=1 (block)
//! RatexResult result = ratex_parse_and_layout("\\frac{1}{2}", &opts);
//! if (result.error_code == 0) {
//!     // consume result.data ...
//!     ratex_free_display_list(result.data);
//! } else {
//!     const char* err = ratex_get_last_error();
//!     // handle error...
//! }
//! ```

#[cfg(target_os = "android")]
pub mod jni;

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_int};

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parse;
use ratex_types::math_style::MathStyle;
use serde_json::Value;

// Thread-local storage for the last error message.
thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_last_error(msg: &str) {
    let bytes: Vec<u8> = msg.bytes().filter(|&b| b != 0).collect();
    let stored = CString::new(bytes).unwrap_or_else(|_| {
        CString::new("(error message could not be encoded)").expect("static C string")
    });
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = Some(stored);
    });
}

fn clear_last_error() {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = None;
    });
}

/// Replace non-finite floats with 0 to produce valid JSON.
fn sanitize_json_numbers(v: Value) -> Value {
    match v {
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f.is_finite() {
                    Value::Number(n)
                } else {
                    Value::Number(serde_json::Number::from_f64(0.0).unwrap())
                }
            } else {
                Value::Number(n)
            }
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(sanitize_json_numbers).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, sanitize_json_numbers(v)))
                .collect(),
        ),
        other => other,
    }
}

fn do_layout(latex_str: &str, style: MathStyle) -> Result<String, String> {
    let nodes = parse(latex_str).map_err(|e| format!("parse error: {e}"))?;
    let options = LayoutOptions::default().with_style(style);
    let layout_box = layout(&nodes, &options);
    let display_list = to_display_list(&layout_box);
    let value = serde_json::to_value(&display_list).map_err(|e| format!("serialization error: {e}"))?;
    let mut sanitized = sanitize_json_numbers(value);
    // Add a protocol version at the top level for forward-compatible decoding.
    if let Value::Object(ref mut map) = sanitized {
        map.insert("version".to_string(), Value::Number(1.into()));
    }
    serde_json::to_string(&sanitized).map_err(|e| format!("JSON stringify error: {e}"))
}

// ---------------------------------------------------------------------------
// Public structs
// ---------------------------------------------------------------------------

/// Options for [`ratex_parse_and_layout`].
///
/// Always set `struct_size = sizeof(RatexOptions)` before passing to the function.
/// Fields beyond `struct_size` are ignored, enabling forward compatibility.
#[repr(C)]
pub struct RatexOptions {
    /// Must be set to `sizeof(RatexOptions)` by the caller.
    pub struct_size: usize,
    /// Rendering mode:
    /// - `0` — inline (text style, equivalent to `$...$`)
    /// - `1` — display block (display style, equivalent to `$$...$$`)
    pub display_mode: c_int,
}

/// Result returned by [`ratex_parse_and_layout`].
///
/// On success: `error_code == 0` and `data` is a heap-allocated JSON string;
/// free it with [`ratex_free_display_list`].
/// On error: `error_code != 0`, `data` is NULL; call [`ratex_get_last_error`] for details.
#[repr(C)]
pub struct RatexResult {
    /// JSON display list on success, NULL on error.
    pub data: *mut c_char,
    /// `0` on success, non-zero on error.
    pub error_code: c_int,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a LaTeX string and compute its display list with explicit rendering options.
///
/// Pass `opts = NULL` to use display-mode defaults.
///
/// # Safety
/// - `latex` must be a valid non-null null-terminated UTF-8 C string.
/// - `opts` may be NULL. If non-null it must point to a valid `RatexOptions` whose
///   `struct_size` field is set correctly.
#[no_mangle]
pub unsafe extern "C" fn ratex_parse_and_layout(
    latex: *const c_char,
    opts: *const RatexOptions,
) -> RatexResult {
    let err_result = |msg: &str| -> RatexResult {
        set_last_error(msg);
        RatexResult { data: std::ptr::null_mut(), error_code: 1 }
    };

    clear_last_error();

    if latex.is_null() {
        return err_result("ratex_parse_and_layout: latex pointer is null");
    }

    let latex_str = match unsafe { CStr::from_ptr(latex) }.to_str() {
        Ok(s) => s,
        Err(e) => return err_result(&format!("invalid UTF-8 in latex string: {e}")),
    };

    let style = if opts.is_null() {
        MathStyle::Display
    } else {
        let opts_ref = unsafe { &*opts };
        let min_size = std::mem::offset_of!(RatexOptions, display_mode)
            + std::mem::size_of::<c_int>();
        if opts_ref.struct_size >= min_size && opts_ref.display_mode == 0 {
            MathStyle::Text
        } else {
            MathStyle::Display
        }
    };

    match do_layout(latex_str, style) {
        Ok(json) => match CString::new(json) {
            Ok(cs) => RatexResult { data: cs.into_raw(), error_code: 0 },
            Err(e) => err_result(&format!("JSON contains interior null byte: {e}")),
        },
        Err(e) => err_result(&e),
    }
}

/// Free a display list JSON string returned by [`ratex_parse_and_layout`].
///
/// Passing NULL is a no-op.
///
/// # Safety
/// `ptr` must have been returned by [`ratex_parse_and_layout`] and must not be freed twice.
#[no_mangle]
pub unsafe extern "C" fn ratex_free_display_list(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { drop(CString::from_raw(ptr)) };
    }
}

/// Return the last error message set by any layout function on this thread.
///
/// # Returns
/// - A pointer to a null-terminated error string, valid until the next layout call on this thread.
/// - NULL if no error has occurred on this thread.
///
/// # Safety
/// The returned pointer is only valid for the lifetime of the current thread and until the
/// next call to a layout function on this thread.
#[no_mangle]
pub extern "C" fn ratex_get_last_error() -> *const c_char {
    LAST_ERROR.with(|cell| {
        cell.borrow()
            .as_ref()
            .map(|cs| cs.as_ptr())
            .unwrap_or(std::ptr::null())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn call(latex: &str, display_mode: c_int) -> Option<String> {
        let input = CString::new(latex).unwrap();
        let opts = RatexOptions {
            struct_size: std::mem::size_of::<RatexOptions>(),
            display_mode,
        };
        let result = unsafe { ratex_parse_and_layout(input.as_ptr(), &opts) };
        if result.error_code != 0 || result.data.is_null() {
            return None;
        }
        let json = unsafe { CStr::from_ptr(result.data) }
            .to_str()
            .unwrap()
            .to_owned();
        unsafe { ratex_free_display_list(result.data) };
        Some(json)
    }

    #[test]
    fn display_fraction() {
        let json = call(r"\frac{1}{2}", 1).expect("should not fail");
        assert!(json.starts_with('{'));
        assert!(json.contains("items"));
    }

    #[test]
    fn inline_fraction() {
        let json = call(r"\frac{1}{2}", 0).expect("should not fail");
        assert!(json.contains("items"));
    }

    #[test]
    fn display_expression() {
        let json = call("x^2 + y^2 = z^2", 1).expect("should not fail");
        assert!(json.contains("items"));
    }

    #[test]
    fn null_latex_returns_error() {
        let opts = RatexOptions {
            struct_size: std::mem::size_of::<RatexOptions>(),
            display_mode: 1,
        };
        let result = unsafe { ratex_parse_and_layout(std::ptr::null(), &opts) };
        assert_ne!(result.error_code, 0);
        assert!(result.data.is_null());
        let err = ratex_get_last_error();
        assert!(!err.is_null());
        let msg = unsafe { CStr::from_ptr(err) }.to_str().unwrap();
        assert!(msg.contains("null"));
    }

    #[test]
    fn null_opts_defaults_to_display() {
        let input = CString::new(r"x^2").unwrap();
        let result = unsafe { ratex_parse_and_layout(input.as_ptr(), std::ptr::null()) };
        assert_eq!(result.error_code, 0);
        assert!(!result.data.is_null());
        unsafe { ratex_free_display_list(result.data) };
    }

    #[test]
    fn free_null_is_noop() {
        unsafe { ratex_free_display_list(std::ptr::null_mut()) };
    }

    #[test]
    fn error_on_bad_latex() {
        let result = call(r"\undefined{x}", 1);
        if result.is_none() {
            let err = ratex_get_last_error();
            assert!(!err.is_null());
        }
    }
}
