use std::{
    ffi::{CString, c_char, c_void},
    ptr,
};
type CF = *const c_void;
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    static kAXTrustedCheckOptionPrompt: CF;
    fn AXUIElementCreateApplication(pid: i32) -> CF;
    fn AXUIElementCopyAttributeValue(element: CF, attribute: CF, value: *mut CF) -> i32;
    fn AXUIElementSetMessagingTimeout(element: CF, timeout: f32) -> i32;
    fn AXIsProcessTrustedWithOptions(options: CF) -> u8;
}
#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    static kCFBooleanTrue: CF;
    fn CFDictionaryCreate(
        allocator: CF,
        keys: *const CF,
        values: *const CF,
        count: isize,
        key_callbacks: CF,
        value_callbacks: CF,
    ) -> CF;
    fn CFStringCreateWithCString(allocator: CF, text: *const c_char, encoding: u32) -> CF;
    fn CFStringGetCString(string: CF, buffer: *mut c_char, size: isize, encoding: u32) -> bool;
    fn CFArrayGetCount(array: CF) -> isize;
    fn CFArrayGetTypeID() -> usize;
    fn CFArrayGetValueAtIndex(array: CF, idx: isize) -> CF;
    fn CFBooleanGetTypeID() -> usize;
    fn CFBooleanGetValue(boolean: CF) -> bool;
    fn CFDictionaryGetTypeID() -> usize;
    fn CFDictionaryGetValueIfPresent(dictionary: CF, key: CF, value: *mut CF) -> bool;
    fn CFNumberGetTypeID() -> usize;
    fn CFNumberGetValue(number: CF, number_type: i32, value: *mut c_void) -> bool;
    fn CFGetTypeID(value: CF) -> usize;
    fn CFStringGetTypeID() -> usize;
    fn CFRelease(value: CF);
}
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOServiceMatching(name: *const c_char) -> CF;
    fn IOServiceGetMatchingService(master_port: u32, matching: CF) -> u32;
    fn IORegistryEntryCreateCFProperty(entry: u32, key: CF, allocator: CF, options: u32) -> CF;
    fn IOObjectRelease(object: u32) -> i32;
}
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to_window: u32) -> CF;
}
const UTF8: u32 = 0x08000100;
const K_CF_NUMBER_SINT64_TYPE: i32 = 4;
const K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY: u32 = 1;
pub fn accessibility_trusted(prompt: bool) -> bool {
    unsafe {
        if !prompt {
            return AXIsProcessTrustedWithOptions(ptr::null()) != 0;
        }
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let options = CFDictionaryCreate(
            ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            ptr::null(),
            ptr::null(),
        );
        if options.is_null() {
            return false;
        }
        let trusted = AXIsProcessTrustedWithOptions(options) != 0;
        CFRelease(options);
        trusted
    }
}
unsafe fn attribute(element: CF, key: &str) -> Option<CF> {
    let key = CString::new(key).ok()?;
    unsafe {
        let name = CFStringCreateWithCString(ptr::null(), key.as_ptr(), UTF8);
        let mut value = ptr::null();
        let result = AXUIElementCopyAttributeValue(element, name, &mut value);
        CFRelease(name);
        if result == 0 && !value.is_null() {
            Some(value)
        } else {
            None
        }
    }
}
unsafe fn string_value(value: CF) -> Option<String> {
    unsafe {
        let mut buffer = vec![0u8; 32768];
        let valid = CFGetTypeID(value) == CFStringGetTypeID()
            && CFStringGetCString(
                value,
                buffer.as_mut_ptr().cast(),
                buffer.len() as isize,
                UTF8,
            );
        if !valid {
            return None;
        }
        let end = buffer.iter().position(|&b| b == 0)?;
        String::from_utf8(buffer[..end].to_vec())
            .ok()
            .filter(|s| !s.is_empty())
    }
}
unsafe fn title_attribute(element: CF) -> Option<String> {
    unsafe {
        let title = attribute(element, "AXTitle")?;
        let value = string_value(title);
        CFRelease(title);
        value
    }
}
unsafe fn number_value(value: CF) -> Option<i64> {
    unsafe {
        if CFGetTypeID(value) != CFNumberGetTypeID() {
            return None;
        }
        let mut out = 0_i64;
        CFNumberGetValue(
            value,
            K_CF_NUMBER_SINT64_TYPE,
            (&mut out as *mut i64).cast(),
        )
        .then_some(out)
    }
}
unsafe fn dictionary_value(dictionary: CF, key: CF) -> Option<CF> {
    unsafe {
        if CFGetTypeID(dictionary) != CFDictionaryGetTypeID() {
            return None;
        }
        let mut value = ptr::null();
        CFDictionaryGetValueIfPresent(dictionary, key, &mut value)
            .then_some(value)
            .filter(|value| !value.is_null())
    }
}
unsafe fn cf_string(text: &str) -> Option<CF> {
    let text = CString::new(text).ok()?;
    unsafe {
        let value = CFStringCreateWithCString(ptr::null(), text.as_ptr(), UTF8);
        (!value.is_null()).then_some(value)
    }
}
pub fn active_window() -> Option<super::ActiveWindow> {
    unsafe {
        let windows = CGWindowListCopyWindowInfo(K_CG_WINDOW_LIST_OPTION_ON_SCREEN_ONLY, 0);
        if windows.is_null() || CFGetTypeID(windows) != CFArrayGetTypeID() {
            if !windows.is_null() {
                CFRelease(windows);
            }
            return None;
        }
        if std::env::var_os("WORKLOG_DEBUG_CAPTURE").is_some() {
            eprintln!(
                "worklog capture: coregraphics windows={}",
                CFArrayGetCount(windows)
            );
        }
        let owner_pid_key = cf_string("kCGWindowOwnerPID")?;
        let owner_name_key = cf_string("kCGWindowOwnerName")?;
        let window_name_key = cf_string("kCGWindowName")?;
        let layer_key = cf_string("kCGWindowLayer")?;
        let mut out = None;
        for i in 0..CFArrayGetCount(windows) {
            let window = CFArrayGetValueAtIndex(windows, i);
            if window.is_null() {
                continue;
            }
            let layer = dictionary_value(window, layer_key).and_then(|v| number_value(v));
            if layer != Some(0) {
                continue;
            }
            let app = dictionary_value(window, owner_name_key).and_then(|v| string_value(v));
            let process_id = dictionary_value(window, owner_pid_key).and_then(|v| number_value(v));
            let Some(app) = app else {
                continue;
            };
            let Some(process_id) = process_id else {
                continue;
            };
            let title = dictionary_value(window, window_name_key).and_then(|v| string_value(v));
            if std::env::var_os("WORKLOG_DEBUG_CAPTURE").is_some() {
                eprintln!(
                    "worklog capture: coregraphics candidate app={app} pid={process_id} title={:?}",
                    title
                );
            }
            out = Some(super::ActiveWindow {
                app,
                title: title.unwrap_or_default(),
                process_id: process_id as i32,
            });
            break;
        }
        CFRelease(owner_pid_key);
        CFRelease(owner_name_key);
        CFRelease(window_name_key);
        CFRelease(layer_key);
        CFRelease(windows);
        out
    }
}
unsafe fn bool_attribute(element: CF, key: &str) -> Option<bool> {
    unsafe {
        let value = attribute(element, key)?;
        let valid = CFGetTypeID(value) == CFBooleanGetTypeID();
        let out = valid.then(|| CFBooleanGetValue(value));
        CFRelease(value);
        out
    }
}
pub fn title(pid: i32) -> Option<String> {
    unsafe {
        let app = AXUIElementCreateApplication(pid);
        if app.is_null() {
            return None;
        }
        AXUIElementSetMessagingTimeout(app, 1.0);
        let window = attribute(app, "AXFocusedWindow");
        if let Some(window) = window {
            let title = title_attribute(window);
            CFRelease(window);
            if title.is_some() {
                CFRelease(app);
                return title;
            }
        }
        let windows = attribute(app, "AXWindows");
        CFRelease(app);
        let windows = windows?;
        if CFGetTypeID(windows) != CFArrayGetTypeID() {
            CFRelease(windows);
            return None;
        }
        let mut first_title = None;
        for i in 0..CFArrayGetCount(windows) {
            let window = CFArrayGetValueAtIndex(windows, i);
            if window.is_null() {
                continue;
            }
            let title = title_attribute(window);
            if first_title.is_none() {
                first_title = title.clone();
            }
            if title.is_some()
                && (bool_attribute(window, "AXFocused").unwrap_or(false)
                    || bool_attribute(window, "AXMain").unwrap_or(false))
            {
                CFRelease(windows);
                return title;
            }
        }
        CFRelease(windows);
        first_title
    }
}
pub fn idle_seconds() -> f64 {
    unsafe {
        let service_name = CString::new("IOHIDSystem").ok();
        let Some(service_name) = service_name else {
            return 0.0;
        };
        let matching = IOServiceMatching(service_name.as_ptr());
        if matching.is_null() {
            return 0.0;
        }
        let service = IOServiceGetMatchingService(0, matching);
        if service == 0 {
            return 0.0;
        }
        let key = CString::new("HIDIdleTime").ok();
        let Some(key) = key else {
            let _ = IOObjectRelease(service);
            return 0.0;
        };
        let key = CFStringCreateWithCString(ptr::null(), key.as_ptr(), UTF8);
        if key.is_null() {
            let _ = IOObjectRelease(service);
            return 0.0;
        }
        let value = IORegistryEntryCreateCFProperty(service, key, ptr::null(), 0);
        CFRelease(key);
        let _ = IOObjectRelease(service);
        if value.is_null() {
            return 0.0;
        }
        let mut idle_ns: i64 = 0;
        let valid = CFGetTypeID(value) == CFNumberGetTypeID()
            && CFNumberGetValue(
                value,
                K_CF_NUMBER_SINT64_TYPE,
                (&mut idle_ns as *mut i64).cast(),
            );
        CFRelease(value);
        if valid && idle_ns > 0 {
            idle_ns as f64 / 1_000_000_000.0
        } else {
            0.0
        }
    }
}
