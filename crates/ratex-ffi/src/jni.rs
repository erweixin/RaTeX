//! Android JNI bridge for RaTeX.
//!
//! Exposes `nativeParseAndLayout` and `nativeGetLastError` as JNI methods on
//! the `io.ratex.RaTeXEngine` Kotlin object.
//!
//! Compiled only when `target_os = "android"` (see lib.rs).

use jni::objects::{JClass, JString};
use jni::sys::{jstring, jobject};
use jni::JNIEnv;

use crate::{ratex_parse_and_layout, ratex_get_last_error};
use std::ffi::CString;
use std::os::raw::c_char;

/// JNI entry point for `RaTeXEngine.nativeParseAndLayout(latex: String): String?`
///
/// Returns a Java `String` on success, or `null` on error.
/// Call `nativeGetLastError()` to retrieve the error message.
#[no_mangle]
pub extern "system" fn Java_io_ratex_RaTeXEngine_nativeParseAndLayout(
    mut env: JNIEnv,
    _class: JClass,
    latex: JString,
) -> jobject {
    // Convert Java String → Rust &str
    let latex_str: String = match env.get_string(&latex) {
        Ok(s) => s.into(),
        Err(e) => {
            let _ = env.throw_new("java/lang/IllegalArgumentException",
                                  format!("invalid latex string: {e}"));
            return std::ptr::null_mut();
        }
    };

    // Build a C string to reuse the existing C ABI
    let c_latex = match CString::new(latex_str) {
        Ok(cs) => cs,
        Err(e) => {
            let _ = env.throw_new("java/lang/IllegalArgumentException",
                                  format!("latex contains null byte: {e}"));
            return std::ptr::null_mut();
        }
    };

    // Call the C ABI function
    let ptr: *mut c_char = unsafe { ratex_parse_and_layout(c_latex.as_ptr()) };

    if ptr.is_null() {
        return std::ptr::null_mut(); // caller checks null and calls nativeGetLastError
    }

    // Convert the JSON C string → Java String, then free the C allocation
    let json = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy().into_owned() };
    unsafe { crate::ratex_free_display_list(ptr) };

    match env.new_string(json) {
        Ok(s) => s.into_raw(),
        Err(e) => {
            let _ = env.throw_new("java/lang/RuntimeException",
                                  format!("failed to create Java string: {e}"));
            std::ptr::null_mut()
        }
    }
}

/// JNI entry point for `RaTeXEngine.nativeGetLastError(): String?`
///
/// Returns the last error message as a Java `String`, or `null` if no error.
#[no_mangle]
pub extern "system" fn Java_io_ratex_RaTeXEngine_nativeGetLastError(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let ptr = ratex_get_last_error();
    if ptr.is_null() {
        return std::ptr::null_mut();
    }
    let msg = unsafe { std::ffi::CStr::from_ptr(ptr).to_string_lossy() };
    match env.new_string(msg.as_ref()) {
        Ok(s) => s.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}
