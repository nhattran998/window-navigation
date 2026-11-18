//! Raw Accessibility FFI (HIServices). Stable C symbols since 10.4; used
//! directly instead of generated bindings for name stability (SRS §4.2).
//! CFStringRef parameters are toll-free-bridged NSString pointers.

#![allow(non_camel_case_types)]

use std::ptr;

/// Core Foundation string reference (toll-free bridged with NSString).
pub type CFStringRef = *const std::os::raw::c_void;
/// Handle to an Accessibility element (app, window, or system-wide).
pub type AXUIElementRef = *const std::os::raw::c_void;
/// AX value wrapper (CGPoint/CGSize payload).
pub type AXValueRef = *const std::os::raw::c_void;
/// Core Foundation dictionary reference.
pub type CFDictionaryRef = *const std::os::raw::c_void;
/// Untyped Core Foundation object reference.
pub type CFTypeRef = *const std::os::raw::c_void;
/// Darwin Boolean (1 byte, 0 = false).
pub type Boolean = u8;
/// Core Foundation array reference.
pub type CFArrayRef = *const std::os::raw::c_void;
/// Core Foundation number reference.
pub type CFNumberRef = *const std::os::raw::c_void;

/// `kAXValueCGPointType` (HIServices AXValueGetType enum).
pub const K_AX_VALUE_CGPOINT_TYPE: i32 = 1;
/// `kAXValueCGSizeType`.
pub const K_AX_VALUE_CGSIZE_TYPE: i32 = 2;

/// A point in global screen coordinates.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CGPoint {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate.
    pub y: f64,
}

/// A size in points.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct CGSize {
    /// Horizontal extent.
    pub width: f64,
    /// Vertical extent.
    pub height: f64,
}

extern "C" {
    // Foundation — linked from CoreFoundation; stable ABI.
    /// Create a CFString from a C string.
    pub fn CFStringCreateWithCString(
        alloc: *const std::os::raw::c_void,
        c_str: *const std::os::raw::c_char,
        encoding: u32,
    ) -> CFStringRef;
    /// kCFStringEncodingUTF8
    pub fn CFStringGetCString(
        s: CFStringRef,
        buffer: *mut std::os::raw::c_char,
        buffer_size: isize,
        encoding: u32,
    ) -> Boolean;
    /// Decrement the retain count, freeing at zero.
    pub fn CFRelease(cf: *const std::os::raw::c_void);

    // HIServices — Accessibility
    /// Element representing "whichever app has focus".
    pub fn AXUIElementCreateSystemWide() -> AXUIElementRef;
    /// Element for the app with the given pid.
    pub fn AXUIElementCreateApplication(pid: i32) -> AXUIElementRef;
    /// Copy an attribute value (title, position, windows…); 0 on success.
    pub fn AXUIElementCopyAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: *mut CFTypeRef,
    ) -> i32;
    /// Set an attribute value; 0 on success.
    pub fn AXUIElementSetAttributeValue(
        element: AXUIElementRef,
        attribute: CFStringRef,
        value: CFTypeRef,
    ) -> i32;
    /// Perform an AX action (e.g. AXRaise); 0 on success.
    pub fn AXUIElementPerformAction(element: AXUIElementRef, action: CFStringRef) -> i32;
    /// Wrap a CGPoint/CGSize into an AXValue.
    pub fn AXValueCreate(value_type: i32, value: *const std::os::raw::c_void) -> AXValueRef;
    /// Unpack an AXValue into the caller struct; 1 on success.
    pub fn AXValueGetValue(
        value: AXValueRef,
        value_type: i32,
        into: *mut std::os::raw::c_void,
    ) -> Boolean;
    /// Whether this process holds the Accessibility permission.
    pub fn AXIsProcessTrusted() -> Boolean;
    /// Same, optionally showing the System Settings prompt.
    pub fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> Boolean;
}

/// AX call succeeded (HIServices AXError).
pub const K_AX_ERROR_SUCCESS: i32 = 0;
/// The attribute exists but is not settable (AXError -2520).
pub const K_AX_ERROR_ATTRIBUTE_UNSUPPORTED: i32 = -2520;
/// The AX call could not complete (AXError -25204).
pub const K_AX_ERROR_CANNOT_COMPLETE: i32 = -25204;

/// Create a CFString from a Rust str (UTF-8, kCFStringEncodingUTF8).
#[must_use]
pub fn cfstring(s: &str) -> CFStringRef {
    let c = std::ffi::CString::new(s).expect("no interior NUL");
    unsafe { CFStringCreateWithCString(ptr::null(), c.as_ptr(), 0x0800_0100) }
}

/// Read a CFString back from a CFStringRef.
///
/// # Safety
///
/// `s` must be a valid CFStringRef or null.
/// Read a CFString back into an owned Rust String.
///
/// # Safety
///
/// `s` must be a valid CFStringRef or null.
pub unsafe fn from_cfstring(s: CFStringRef) -> Option<String> {
    if s.is_null() {
        return None;
    }
    let mut buf = [0i8; 4096];
    let ok = CFStringGetCString(s, buf.as_mut_ptr(), buf.len().cast_signed(), 0x0800_0100);
    if ok == 0 {
        return None;
    }
    let bytes: Vec<u8> = buf
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| c.cast_unsigned())
        .collect();
    String::from_utf8(bytes).ok()
}
