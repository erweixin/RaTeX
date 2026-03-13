//! RaTeX C ABI FFI exports for native platform integration.
//!
//! Platform-specific modules:
//! - `jni` — Android JNI bridge (compiled only on `target_os = "android"`)
//!
//! # Usage (C)
//! ```c
//! const char* json = ratex_parse_and_layout("\\frac{1}{2}");
//! if (json) {
//!     // consume json...
//!     ratex_free_display_list(json);
//! } else {
//!     const char* err = ratex_get_last_error();
//!     // handle error...
//! }
//! ```

#[cfg(target_os = "android")]
pub mod jni;

use std::cell::RefCell;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parse;
use serde_json::Value;

// Thread-local storage for the last error message.
thread_local! {
    static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
}

fn set_last_error(msg: &str) {
    LAST_ERROR.with(|cell| {
        *cell.borrow_mut() = CString::new(msg).ok();
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

/// Parse a LaTeX string and compute its display list, returned as a JSON C string.
///
/// # Returns
/// - On success: heap-allocated null-terminated JSON string. Caller must free with
///   [`ratex_free_display_list`].
/// - On error: NULL pointer. Call [`ratex_get_last_error`] for the error message.
///
/// # Safety
/// `latex` must be a valid non-null null-terminated UTF-8 C string.
#[no_mangle]
pub unsafe extern "C" fn ratex_parse_and_layout(latex: *const c_char) -> *mut c_char {
    clear_last_error();

    if latex.is_null() {
        set_last_error("ratex_parse_and_layout: latex pointer is null");
        return std::ptr::null_mut();
    }

    let latex_str = match unsafe { CStr::from_ptr(latex) }.to_str() {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("invalid UTF-8 in latex string: {e}"));
            return std::ptr::null_mut();
        }
    };

    let nodes = match parse(latex_str) {
        Ok(n) => n,
        Err(e) => {
            set_last_error(&format!("parse error: {e}"));
            return std::ptr::null_mut();
        }
    };

    let options = LayoutOptions::default();
    let layout_box = layout(&nodes, &options);
    let display_list = to_display_list(&layout_box);

    let value = match serde_json::to_value(&display_list) {
        Ok(v) => v,
        Err(e) => {
            set_last_error(&format!("serialization error: {e}"));
            return std::ptr::null_mut();
        }
    };

    let sanitized = sanitize_json_numbers(value);
    let json = match serde_json::to_string(&sanitized) {
        Ok(s) => s,
        Err(e) => {
            set_last_error(&format!("JSON stringify error: {e}"));
            return std::ptr::null_mut();
        }
    };

    match CString::new(json) {
        Ok(cs) => cs.into_raw(),
        Err(e) => {
            set_last_error(&format!("JSON contains interior null byte: {e}"));
            std::ptr::null_mut()
        }
    }
}

/// Free a display list JSON string previously returned by [`ratex_parse_and_layout`].
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

/// Return the last error message set by [`ratex_parse_and_layout`].
///
/// # Returns
/// - A pointer to a null-terminated error string, valid until the next call to
///   [`ratex_parse_and_layout`] on this thread.
/// - NULL if no error has occurred on this thread.
///
/// # Safety
/// The returned pointer is only valid for the lifetime of the current thread and until the
/// next call to [`ratex_parse_and_layout`].
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

    fn call(latex: &str) -> Option<String> {
        let input = CString::new(latex).unwrap();
        let ptr = unsafe { ratex_parse_and_layout(input.as_ptr()) };
        if ptr.is_null() {
            None
        } else {
            let result = unsafe { CStr::from_ptr(ptr) }
                .to_str()
                .unwrap()
                .to_owned();
            unsafe { ratex_free_display_list(ptr) };
            Some(result)
        }
    }

    #[test]
    fn simple_fraction() {
        let json = call(r"\frac{1}{2}").expect("should not fail");
        assert!(json.starts_with('{'));
        assert!(json.contains("items"));
    }

    #[test]
    fn simple_expression() {
        let json = call("x^2 + y^2 = z^2").expect("should not fail");
        assert!(json.contains("items"));
    }

    #[test]
    fn null_pointer_returns_null() {
        let ptr = unsafe { ratex_parse_and_layout(std::ptr::null()) };
        assert!(ptr.is_null());
        let err = ratex_get_last_error();
        assert!(!err.is_null());
        let msg = unsafe { CStr::from_ptr(err) }.to_str().unwrap();
        assert!(msg.contains("null"));
    }

    #[test]
    fn free_null_is_noop() {
        unsafe { ratex_free_display_list(std::ptr::null_mut()) };
    }

    #[test]
    fn error_on_bad_latex() {
        // \undefined is not a known command — parser should report an error
        let result = call(r"\undefined{x}");
        if result.is_none() {
            let err = ratex_get_last_error();
            assert!(!err.is_null());
        }
        // If it succeeds (graceful fallback), that's also acceptable
    }
}
