use crate::{stop_recording_impl, toggle_recording_impl};
use core_foundation::runloop::{kCFRunLoopDefaultMode, CFRunLoop};
use core_graphics::event::{
    CGEvent, CGEventFlags, CGEventTap, CGEventTapLocation, CGEventTapOptions,
    CGEventTapPlacement, CGEventType,
};
use core_graphics::event::EventField;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::AppHandle;

const FN_KEYCODE: i64 = 63;

#[derive(Debug)]
struct State {
    down: bool,
    down_at: Option<Instant>,
    hold_fired: bool,
    last_tap_at: Option<Instant>,
}

pub fn init(app: &AppHandle) -> Result<(), String> {
    let cfg = crate::config::load_or_default(app)?;
    let hold_ms = cfg.thresholds.hold_ms;
    let double_click_ms = cfg.thresholds.double_click_ms;

    let app = app.clone();
    std::thread::spawn(move || {
        let shared = Arc::new(Mutex::new(State {
            down: false,
            down_at: None,
            hold_fired: false,
            last_tap_at: None,
        }));

        let shared_cb = Arc::clone(&shared);
        let app_cb = app.clone();

        let tap = CGEventTap::new(
            CGEventTapLocation::HID,
            CGEventTapPlacement::HeadInsertEventTap,
            CGEventTapOptions::Default,
            vec![CGEventType::FlagsChanged],
            move |_proxy, _etype, event| handle_event(&app_cb, &shared_cb, event, hold_ms, double_click_ms),
        );

        let Ok(tap) = tap else {
            eprintln!("VoiceDictation: failed to create CGEventTap (missing Input Monitoring permission?)");
            return;
        };

        let Ok(source) = tap.mach_port.create_runloop_source(0) else {
            eprintln!("VoiceDictation: failed to create runloop source for event tap");
            return;
        };

        let run_loop = CFRunLoop::get_current();
        unsafe {
            run_loop.add_source(&source, kCFRunLoopDefaultMode);
        }
        tap.enable();

        CFRunLoop::run_current();
    });

    Ok(())
}

fn handle_event(
    app: &AppHandle,
    shared: &Arc<Mutex<State>>,
    event: &CGEvent,
    hold_ms: u64,
    double_click_ms: u64,
) -> Option<CGEvent> {
    let keycode = event.get_integer_value_field(EventField::KEYBOARD_EVENT_KEYCODE);
    if keycode != FN_KEYCODE {
        return Some(event.clone());
    }

    let flags = event.get_flags();
    let is_down = flags.contains(CGEventFlags::CGEventFlagSecondaryFn);
    let now = Instant::now();

    let mut st = shared.lock().ok()?;

    if is_down && !st.down {
        st.down = true;
        st.down_at = Some(now);
        st.hold_fired = false;

        let app2 = app.clone();
        let shared2 = Arc::clone(shared);
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(hold_ms));
            let mut st = match shared2.lock() {
                Ok(v) => v,
                Err(_) => return,
            };
            if st.down && !st.hold_fired {
                st.hold_fired = true;
                drop(st);
                tauri::async_runtime::spawn(async move {
                    let _ = toggle_recording_impl(app2).await;
                });
            }
        });

        return Some(event.clone());
    }

    if !is_down && st.down {
        st.down = false;

        if st.hold_fired {
            st.down_at = None;
            st.hold_fired = false;
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                let _ = stop_recording_impl(app2).await;
            });
            return None;
        }

        let was_tap = st
            .down_at
            .is_some_and(|t| now.duration_since(t) < Duration::from_millis(hold_ms));
        st.down_at = None;

        if was_tap {
            let is_double = st
                .last_tap_at
                .is_some_and(|t| now.duration_since(t) < Duration::from_millis(double_click_ms));

            st.last_tap_at = Some(now);

            if is_double {
                st.last_tap_at = None;
                let app2 = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = toggle_recording_impl(app2).await;
                });
                return None;
            }
        }
    }

    Some(event.clone())
}
