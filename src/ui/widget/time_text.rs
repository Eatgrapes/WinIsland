use std::cell::RefCell;
use std::time::Duration;

struct TimeText {
    hour: u16,
    minute: u16,
    value: String,
}

thread_local! {
    static TIME_TEXT: RefCell<TimeText> = const {
        RefCell::new(TimeText {
            hour: u16::MAX,
            minute: u16::MAX,
            value: String::new(),
        })
    };
}

pub(crate) fn with_current_time_text<T>(draw: impl FnOnce(&str) -> T) -> T {
    // SAFETY: GetLocalTime returns a fully initialized SYSTEMTIME value.
    let local_time = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    TIME_TEXT.with(|cell| {
        let mut cache = cell.borrow_mut();
        if cache.hour != local_time.wHour || cache.minute != local_time.wMinute {
            cache.hour = local_time.wHour;
            cache.minute = local_time.wMinute;
            cache.value = format!("{:02}:{:02}", local_time.wHour, local_time.wMinute);
        }
        draw(&cache.value)
    })
}

pub(crate) fn until_next_minute() -> Duration {
    // SAFETY: GetLocalTime returns a fully initialized SYSTEMTIME value.
    let local_time = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    let elapsed_ms = u64::from(local_time.wSecond) * 1_000 + u64::from(local_time.wMilliseconds);
    Duration::from_millis(60_000_u64.saturating_sub(elapsed_ms).max(1))
}
