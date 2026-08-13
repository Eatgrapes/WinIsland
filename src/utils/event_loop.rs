use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use winit::event_loop::EventLoopProxy;

static PROXY: OnceLock<EventLoopProxy<()>> = OnceLock::new();
static WAKE_PENDING: AtomicBool = AtomicBool::new(false);

pub(crate) fn set_proxy(proxy: EventLoopProxy<()>) {
    let _ = PROXY.set(proxy);
}

pub(crate) fn acknowledge_wake() {
    WAKE_PENDING.store(false, Ordering::Release);
}

pub(crate) fn wake() {
    let Some(proxy) = PROXY.get() else {
        return;
    };
    if !WAKE_PENDING.swap(true, Ordering::AcqRel) && proxy.send_event(()).is_err() {
        WAKE_PENDING.store(false, Ordering::Release);
    }
}
