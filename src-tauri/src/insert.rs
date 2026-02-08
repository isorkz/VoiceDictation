use std::thread;
use std::time::Duration;

pub struct ClipboardGuard {
    original_text: Option<String>,
}

impl ClipboardGuard {
    pub fn restore(self) -> Result<(), String> {
        if let Some(text) = self.original_text {
            let mut clipboard = arboard::Clipboard::new()
                .map_err(|e| format!("failed to open clipboard for restore: {e}"))?;
            clipboard
                .set_text(text)
                .map_err(|e| format!("failed to restore clipboard: {e}"))?;
        }
        Ok(())
    }
}

pub fn set_clipboard_text_with_guard(text: &str, restore_original: bool) -> Result<ClipboardGuard, String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|e| format!("failed to open clipboard: {e}"))?;

    let original_text = if restore_original {
        clipboard.get_text().ok()
    } else {
        None
    };

    clipboard
        .set_text(text.to_string())
        .map_err(|e| format!("failed to set clipboard text: {e}"))?;

    Ok(ClipboardGuard { original_text })
}

pub fn paste() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        return macos_paste();
    }
    #[cfg(windows)]
    {
        return windows_paste();
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        return Err("paste injection is not supported on this platform".to_string());
    }
}

pub fn clipboard_paste_restore(text: &str, restore_original: bool) -> Result<(), String> {
    let guard = set_clipboard_text_with_guard(text, restore_original)?;
    paste()?;
    thread::sleep(Duration::from_millis(150));
    guard.restore()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn macos_paste() -> Result<(), String> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    const KEY_V: u16 = 0x09;

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|e| format!("failed to create event source: {e:?}"))?;

    let mut key_down =
        CGEvent::new_keyboard_event(source.clone(), KEY_V, true).ok_or("failed to create keydown")?;
    key_down.set_flags(CGEventFlags::CGEventFlagMaskCommand);
    key_down.post(CGEventTapLocation::HID);

    let mut key_up =
        CGEvent::new_keyboard_event(source, KEY_V, false).ok_or("failed to create keyup")?;
    key_up.set_flags(CGEventFlags::CGEventFlagMaskCommand);
    key_up.post(CGEventTapLocation::HID);

    Ok(())
}

#[cfg(windows)]
fn windows_paste() -> Result<(), String> {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY,
    };

    const VK_CONTROL: VIRTUAL_KEY = VIRTUAL_KEY(0x11);
    const VK_V: VIRTUAL_KEY = VIRTUAL_KEY(0x56);

    unsafe fn key_input(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
        let flags = if key_up { KEYEVENTF_KEYUP } else { Default::default() };
        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: flags,
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        }
    }

    let inputs = [
        unsafe { key_input(VK_CONTROL, false) },
        unsafe { key_input(VK_V, false) },
        unsafe { key_input(VK_V, true) },
        unsafe { key_input(VK_CONTROL, true) },
    ];

    let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
    if sent as usize != inputs.len() {
        return Err(format!("SendInput sent {sent} events (expected {})", inputs.len()));
    }
    Ok(())
}

