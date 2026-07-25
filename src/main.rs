#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]
mod core;
mod icons;
mod plugin;
mod ui;
mod utils;
mod window;
use crate::core::i18n::init_i18n;
use crate::utils::logger;
use crate::window::app::{App, HotkeyAction};
use std::env;
use std::mem::ManuallyDrop;
use std::sync::mpsc;
use std::sync::{Mutex, OnceLock};
use windows::Win32::Foundation::ERROR_ALREADY_EXISTS;
use windows::Win32::Foundation::{CloseHandle, GetLastError, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_CONTROL, VK_LCONTROL, VK_LEFT, VK_RCONTROL, VK_RIGHT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT, LLKHF_UP, SetWindowsHookExW,
    UnhookWindowsHookEx, WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
};
use windows::core::w;
use winit::event_loop::EventLoop;

struct KeyboardHookState {
    tx: mpsc::Sender<HotkeyAction>,
    ctrl_down: bool,
    s_down: bool,
    left_down: bool,
    right_down: bool,
}

static KEYBOARD_HOOK_STATE: OnceLock<Mutex<KeyboardHookState>> = OnceLock::new();
const VK_S: u32 = b'S' as u32;

unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        // SAFETY: Windows passes a valid KBDLLHOOKSTRUCT pointer for WH_KEYBOARD_LL callbacks
        // while this hook invocation is active.
        let keyboard = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
        let key_up = keyboard.flags.contains(LLKHF_UP);
        let key_down =
            !key_up && (wparam.0 as u32 == WM_KEYDOWN || wparam.0 as u32 == WM_SYSKEYDOWN);
        let mut handled = false;

        if let Some(state) = KEYBOARD_HOOK_STATE.get()
            && let Ok(mut state) = state.lock()
        {
            match keyboard.vkCode {
                key if key == VK_CONTROL.0 as u32
                    || key == VK_LCONTROL.0 as u32
                    || key == VK_RCONTROL.0 as u32 =>
                {
                    state.ctrl_down = key_down;
                }
                VK_S => {
                    state.s_down = key_down;
                }
                key if key == VK_LEFT.0 as u32 => {
                    if key_up {
                        state.left_down = false;
                    } else if key_down {
                        let was_down = std::mem::replace(&mut state.left_down, true);
                        if state.ctrl_down && state.s_down {
                            if !was_down {
                                let _ = state.tx.send(HotkeyAction::PreviousSource);
                                log::info!("Ctrl+S+Left island source shortcut triggered");
                            }
                            handled = true;
                        }
                    }
                }
                key if key == VK_RIGHT.0 as u32 => {
                    if key_up {
                        state.right_down = false;
                    } else if key_down {
                        let was_down = std::mem::replace(&mut state.right_down, true);
                        if state.ctrl_down && state.s_down {
                            if !was_down {
                                let _ = state.tx.send(HotkeyAction::NextSource);
                                log::info!("Ctrl+S+Right island source shortcut triggered");
                            }
                            handled = true;
                        }
                    }
                }
                _ => {}
            }
        }
        if handled {
            return LRESULT(1);
        }
    }

    // SAFETY: Unhandled hook messages must be passed to the next hook in the chain.
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

fn install_keyboard_hotkey_hook(tx: mpsc::Sender<HotkeyAction>) -> Option<HHOOK> {
    if KEYBOARD_HOOK_STATE
        .set(Mutex::new(KeyboardHookState {
            tx,
            ctrl_down: false,
            s_down: false,
            left_down: false,
            right_down: false,
        }))
        .is_err()
    {
        log::warn!("Unable to install fallback keyboard hook: hook state already initialized");
        return None;
    }

    // SAFETY: The low-level keyboard hook callback is a static function and remains valid for the
    // process lifetime. The hook is uninstalled before shutdown.
    match unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), None, 0) } {
        Ok(hook) => {
            log::info!("Installed Ctrl+S+Arrow island source keyboard hook");
            Some(hook)
        }
        Err(error) => {
            log::warn!("Unable to install Ctrl+S+Arrow keyboard hook: {error}");
            None
        }
    }
}

fn uninstall_keyboard_hotkey_hook(hook: Option<HHOOK>) {
    if let Some(hook) = hook {
        // SAFETY: This uninstalls the hook handle returned by SetWindowsHookExW.
        let _ = unsafe { UnhookWindowsHookEx(hook) };
    }
}

fn main() {
    let _ = logger::init();
    log::info!("WinIsland v{} starting", env!("CARGO_PKG_VERSION"));

    let config = core::persistence::load_config();
    let _ = utils::autostart::set_autostart(config.auto_start);
    logger::check_crash_flag();
    init_i18n(&config.language);

    let args: Vec<String> = env::args().collect();
    let is_restart = args.iter().any(|arg| arg == "--restart");
    log::info!("Args: {:?}", args);
    log::info!(
        "Config: style={:?}, scale={}, lang={}",
        config.island_style,
        config.global_scale,
        config.language
    );

    if is_restart {
        std::thread::sleep(std::time::Duration::from_millis(300));
    }
    let _single_mutex = {
        let start = std::time::Instant::now();
        loop {
            // SAFETY: CreateMutexW creates a named mutex for single-instance lock.
            // The name is a static string literal. On success with no ERROR_ALREADY_EXISTS,
            // the handle is kept in ManuallyDrop to hold the lock for the process lifetime.
            // On ERROR_ALREADY_EXISTS, the handle is closed and we retry or exit.
            unsafe {
                let h = CreateMutexW(None, true, w!("Local\\WinIsland_SingleInstance_Mutex"));
                match h {
                    Ok(handle) => {
                        if GetLastError() != ERROR_ALREADY_EXISTS {
                            break ManuallyDrop::new(handle);
                        }
                        let _ = CloseHandle(handle);
                    }
                    Err(_) => {
                        if !is_restart {
                            return;
                        }
                    }
                }
            }
            if !is_restart || start.elapsed() > std::time::Duration::from_secs(10) {
                if is_restart {
                    let own_pid = std::process::id();
                    if let Ok(output) = std::process::Command::new("powershell")
                        .args([
                            "-NoProfile",
                            "-Command",
                            &format!(
                                "Get-Process WinIsland -ErrorAction SilentlyContinue | Where-Object {{$_.Id -ne {own_pid}}} | Stop-Process -Force"
                            ),
                        ])
                        .output()
                        && output.status.success()
                    {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        continue;
                    }
                }
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
    };

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let _guard = runtime.enter();

    utils::updater::start_update_checker();

    let (hotkey_tx, hotkey_rx) = mpsc::channel();
    let event_loop = EventLoop::builder().build().unwrap();
    let keyboard_hook = install_keyboard_hotkey_hook(hotkey_tx);
    let mut app = App::with_hotkey_receiver(hotkey_rx);
    let run_result = event_loop.run_app(&mut app);
    uninstall_keyboard_hotkey_hook(keyboard_hook);
    run_result.unwrap();
    log::info!("Application event loop exited, shutting down");
}
