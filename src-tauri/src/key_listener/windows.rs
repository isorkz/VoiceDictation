use crate::{stop_recording_impl, toggle_recording_impl};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::AppHandle;
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{KBDLLHOOKSTRUCT, VK_LSHIFT, VK_LWIN, VK_RSHIFT};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage, UnhookWindowsHookEx,
    HHOOK, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

#[derive(Clone, Copy)]
struct Hotkey {
    win: bool,
    shift: bool,
    ctrl: bool,
    alt: bool,
    key_vk: u32,
}

#[derive(Clone, Copy)]
struct Thresholds {
    hold_ms: u64,
    double_click_ms: u64,
}

struct State {
    pressed_win: bool,
    pressed_shift: bool,
    pressed_ctrl: bool,
    pressed_alt: bool,
    pressed_key: bool,
    key_down_at: Option<Instant>,
    hold_fired: bool,
    last_tap_at: Option<Instant>,
}

static APP: OnceLock<AppHandle> = OnceLock::new();
static HOTKEY: OnceLock<Mutex<Hotkey>> = OnceLock::new();
static THRESHOLDS: OnceLock<Mutex<Thresholds>> = OnceLock::new();
static STATE: OnceLock<Mutex<State>> = OnceLock::new();
static HOOK: OnceLock<Mutex<Option<HHOOK>>> = OnceLock::new();

pub fn init(app: &AppHandle) -> Result<(), String> {
    APP.set(app.clone()).ok();
    let cfg = crate::config::load_or_default(app)?;
    HOTKEY.get_or_init(|| Mutex::new(parse_hotkey(&cfg.hotkey.windows)));
    THRESHOLDS.get_or_init(|| {
        Mutex::new(Thresholds {
            hold_ms: cfg.thresholds.hold_ms,
            double_click_ms: cfg.thresholds.double_click_ms,
        })
    });
    STATE.get_or_init(|| {
        Mutex::new(State {
            pressed_win: false,
            pressed_shift: false,
            pressed_ctrl: false,
            pressed_alt: false,
            pressed_key: false,
            key_down_at: None,
            hold_fired: false,
            last_tap_at: None,
        })
    });
    HOOK.get_or_init(|| Mutex::new(None));

    std::thread::spawn(move || unsafe {
        let module: HINSTANCE = GetModuleHandleW(None).unwrap_or(HINSTANCE(0));
        let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(hook_proc), module, 0);
        if hook.0 == 0 {
            return;
        }

        if let Ok(mut slot) = HOOK.get().unwrap().lock() {
            *slot = Some(hook);
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).into() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        let _ = UnhookWindowsHookEx(hook);
    });

    Ok(())
}

unsafe extern "system" fn hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    let msg = wparam.0 as u32;
    if msg != WM_KEYDOWN && msg != WM_KEYUP && msg != WM_SYSKEYDOWN && msg != WM_SYSKEYUP {
        return CallNextHookEx(None, code, wparam, lparam);
    }

    let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let kb = *(lparam.0 as *const KBDLLHOOKSTRUCT);
    let vk = kb.vkCode;

    let hotkey = HOTKEY
        .get()
        .and_then(|m| m.lock().ok())
        .map(|g| *g)
        .unwrap_or(parse_hotkey("Win+Shift+D"));
    let thresholds = THRESHOLDS
        .get()
        .and_then(|m| m.lock().ok())
        .map(|g| *g)
        .unwrap_or(Thresholds {
            hold_ms: 180,
            double_click_ms: 300,
        });
    let mut st = match STATE.get().and_then(|m| m.lock().ok()) {
        Some(v) => v,
        None => return CallNextHookEx(None, code, wparam, lparam),
    };

    match vk {
        0x5B | 0x5C => st.pressed_win = is_down,
        VK_LSHIFT.0 as u32 | VK_RSHIFT.0 as u32 => st.pressed_shift = is_down,
        0x11 => st.pressed_ctrl = is_down,
        0x12 => st.pressed_alt = is_down,
        _ => {}
    }

    if vk == hotkey.key_vk {
        if is_down && !st.pressed_key {
            st.pressed_key = true;
            let pressed_at = Instant::now();
            st.key_down_at = Some(pressed_at);
            st.hold_fired = false;

            if modifiers_match(&hotkey, &st) {
                spawn_hold_timer(thresholds.hold_ms, pressed_at);
            }
        } else if !is_down && st.pressed_key {
            st.pressed_key = false;
            let now = Instant::now();

            if st.hold_fired {
                st.key_down_at = None;
                st.hold_fired = false;
                if let Some(app) = APP.get() {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let _ = stop_recording_impl(app).await;
                    });
                }
            } else {
                let is_tap = st
                    .key_down_at
                    .is_some_and(|t| now.duration_since(t) < Duration::from_millis(thresholds.hold_ms));
                st.key_down_at = None;

                if is_tap && modifiers_match(&hotkey, &st) {
                    let double = st
                        .last_tap_at
                        .is_some_and(|t| now.duration_since(t) < Duration::from_millis(thresholds.double_click_ms));
                    st.last_tap_at = Some(now);
                    if double {
                        st.last_tap_at = None;
                        if let Some(app) = APP.get() {
                            let app = app.clone();
                            tauri::async_runtime::spawn(async move {
                                let _ = toggle_recording_impl(app).await;
                            });
                        }
                    }
                }
            }
        }
    }

    CallNextHookEx(None, code, wparam, lparam)
}

fn modifiers_match(hk: &Hotkey, st: &State) -> bool {
    (!hk.win || st.pressed_win)
        && (!hk.shift || st.pressed_shift)
        && (!hk.ctrl || st.pressed_ctrl)
        && (!hk.alt || st.pressed_alt)
}

fn spawn_hold_timer(hold_ms: u64, pressed_at: Instant) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(hold_ms));
        let mut st = match STATE.get().and_then(|m| m.lock().ok()) {
            Some(v) => v,
            None => return,
        };
        if st.pressed_key && !st.hold_fired && st.key_down_at == Some(pressed_at) {
            st.hold_fired = true;
            drop(st);
            if let Some(app) = APP.get() {
                let app = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = toggle_recording_impl(app).await;
                });
            }
        }
    });
}

fn parse_hotkey(input: &str) -> Hotkey {
    let mut hk = Hotkey {
        win: false,
        shift: false,
        ctrl: false,
        alt: false,
        key_vk: 0x44, // D
    };

    for part in input.split('+').map(|s| s.trim()).filter(|s| !s.is_empty()) {
        match part.to_ascii_lowercase().as_str() {
            "win" | "meta" | "super" => hk.win = true,
            "shift" => hk.shift = true,
            "ctrl" | "control" => hk.ctrl = true,
            "alt" => hk.alt = true,
            k if k.len() == 1 => {
                hk.key_vk = k.as_bytes()[0].to_ascii_uppercase() as u32;
            }
            _ => {}
        }
    }

    hk
}
