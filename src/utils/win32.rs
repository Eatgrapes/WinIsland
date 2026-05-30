use std::sync::atomic::{AtomicUsize, Ordering};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GWL_EXSTYLE, GWL_STYLE, GetWindowLongPtrW, HWND_TOPMOST, PostMessageW, SW_RESTORE,
    SWP_NOACTIVATE, SetForegroundWindow, SetWindowDisplayAffinity, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, WDA_EXCLUDEFROMCAPTURE, WDA_NONE, WM_CLOSE,
};
use windows::core::PCWSTR;

pub fn find_window(title: &str) -> Option<HWND> {
    let wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let hwnd = FindWindowW(None, PCWSTR::from_raw(wide.as_ptr()));
        if let Ok(hwnd) = hwnd
            && !hwnd.is_invalid()
        {
            Some(hwnd)
        } else {
            None
        }
    }
}

pub fn close_window(title: &str) {
    if let Some(hwnd) = find_window(title) {
        unsafe {
            let _ = PostMessageW(hwnd, WM_CLOSE, None, None);
        }
    }
}

pub fn bring_window_to_front(title: &str) {
    if let Some(hwnd) = find_window(title) {
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

pub fn modify_window_ex_style(hwnd: HWND, add_flags: isize, remove_flags: isize) {
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let new_style = (current | add_flags) & !remove_flags;
        let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
    }
}

pub fn modify_window_style(hwnd: HWND, add_flags: isize, remove_flags: isize) {
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let new_style = (current | add_flags) & !remove_flags;
        let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);
    }
}

pub fn set_window_topmost(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        let _ = SetWindowPos(hwnd, HWND_TOPMOST, x, y, w, h, SWP_NOACTIVATE);
    }
}

static ISLAND_HWND: AtomicUsize = AtomicUsize::new(0);

pub fn set_island_hwnd(hwnd: HWND) {
    ISLAND_HWND.store(hwnd.0 as usize, Ordering::Relaxed);
}

/// Temporarily exclude the island window from GDI capture, execute `f`,
/// then restore capture visibility.
///
/// This keeps the island visible to screenshot tools by default (WDA_NONE),
/// but hides it during glass-effect GDI screen capture to prevent self-feedback
/// (the island's own content being blurred into the background).
pub fn with_capture_exclusion<R>(f: impl FnOnce() -> R) -> R {
    let raw = ISLAND_HWND.load(Ordering::Relaxed);
    if raw != 0 {
        let hwnd = HWND(raw as *mut std::ffi::c_void);
        unsafe {
            let _ = SetWindowDisplayAffinity(hwnd, WDA_EXCLUDEFROMCAPTURE);
        }
    }
    let result = f();
    if raw != 0 {
        let hwnd = HWND(raw as *mut std::ffi::c_void);
        unsafe {
            let _ = SetWindowDisplayAffinity(hwnd, WDA_NONE);
        }
    }
    result
}
