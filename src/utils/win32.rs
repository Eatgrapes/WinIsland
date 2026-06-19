use windows::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, GWL_EXSTYLE, GWL_STYLE, GetWindowLongPtrW, HWND_TOPMOST, PostMessageW, SW_RESTORE,
    SWP_NOACTIVATE, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos, ShowWindow, WM_CLOSE,
};
use windows::core::PCWSTR;

// SAFETY: FindWindowW is called with a null-terminated wide string derived
// from the title parameter. The function returns an HWND that may be invalid
// or null, which we check via is_invalid() before returning.
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

// SAFETY: PostMessageW sends WM_CLOSE to a target HWND. The hwnd is obtained
// from find_window which validates its validity. No memory is accessed through
// the HWND — it's a message-only operation.
pub fn close_window(title: &str) {
    if let Some(hwnd) = find_window(title) {
        unsafe {
            let _ = PostMessageW(Some(hwnd), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
    }
}

// SAFETY: ShowWindow and SetForegroundWindow are called on a validated HWND.
// These are UI operations that may fail silently if the window is in a
// different input state (e.g., UIPI blocked), which we accept by discarding
// the result.
pub fn bring_window_to_front(title: &str) {
    if let Some(hwnd) = find_window(title) {
        unsafe {
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

// SAFETY: GetWindowLongPtrW reads and SetWindowLongPtrW writes the extended
// window style of a validated HWND. Bitwise operations on the style flags are
// safe and the updated style takes effect immediately.
pub fn modify_window_ex_style(hwnd: HWND, add_flags: isize, remove_flags: isize) {
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let new_style = (current | add_flags) & !remove_flags;
        let _ = SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
    }
}

// SAFETY: GetWindowLongPtrW reads and SetWindowLongPtrW writes the window
// style of a validated HWND. Bitwise operations on the style flags are safe
// and the updated style takes effect immediately.
pub fn modify_window_style(hwnd: HWND, add_flags: isize, remove_flags: isize) {
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_STYLE);
        let new_style = (current | add_flags) & !remove_flags;
        let _ = SetWindowLongPtrW(hwnd, GWL_STYLE, new_style);
    }
}

// SAFETY: SetWindowPos repositions a validated HWND with SWP_NOACTIVATE to
// prevent focus stealing. The HWND_TOPMOST flag ensures the window stays
// above other windows. All parameters are provided by the caller and assumed
// valid.
pub fn set_window_topmost(hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    unsafe {
        let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), x, y, w, h, SWP_NOACTIVATE);
    }
}
