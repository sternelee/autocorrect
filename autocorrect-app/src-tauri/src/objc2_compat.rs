//! Objective-C types and utilities using objc2
//!
//! This module provides compatibility types and functions for working with
//! Objective-C APIs on macOS using the objc2 crate instead of the deprecated
//! cocoa crate.

#![allow(dead_code)]

use objc2::runtime::{AnyClass, AnyObject};

/// Type alias for Objective-C object pointers
pub type Id = *mut AnyObject;

/// Null pointer constant for id types
pub const NIL: Id = std::ptr::null_mut();

/// Boolean constants for Objective-C
pub const YES: bool = true;
pub const NO: bool = false;

/// Get an Objective-C class by name
pub fn class(name: &str) -> &'static AnyClass {
    AnyClass::get(name).unwrap_or_else(|| panic!("Class {} not found", name))
}

/// Create an NSString from a UTF-8 string
pub fn ns_string(s: &str) -> Id {
    unsafe {
        let c_string = std::ffi::CString::new(s).expect("Invalid string");
        msg_send![class("NSString"), stringWithUTF8String: c_string.as_ptr()]
    }
}

/// Get a Rust String from an NSString
pub fn from_ns_string(ns_string: Id) -> String {
    if ns_string.is_null() {
        return String::new();
    }
    unsafe {
        let c_str: *const std::os::raw::c_char = msg_send![ns_string, UTF8String];
        if c_str.is_null() {
            String::new()
        } else {
            std::ffi::CStr::from_ptr(c_str)
                .to_string_lossy()
                .into_owned()
        }
    }
}

/// Re-export msg_send! macro from objc2
pub use objc2::msg_send;
