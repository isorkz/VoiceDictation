# Permissions

VoiceDictation needs OS-level permissions to:
- record microphone audio
- listen for global key events (macOS Globe/Fn)
- paste text into the currently focused app

## macOS

Required:
- **Microphone**
- **Input Monitoring** (global key listening)
- **Accessibility** (simulate paste)

Note:
- The macOS app runs in tray-only mode (no Dock icon and not shown in Cmd+Tab). Use the tray icon/menu to interact with it.
- The first time you start recording, macOS should show a Microphone permission prompt. If it doesn't, check System Settings → Privacy & Security → Microphone.

If a feature is not working, check:
System Settings → Privacy & Security → (Microphone / Input Monitoring / Accessibility)

## macOS Globe/Fn (Language key)

The Globe/Fn key is also used by macOS for system features (e.g. input source switching). VoiceDictation listens for it via an event tap and only attempts to swallow it when a hold/double-click trigger is detected.

See:
- `docs/macos-globe.md`

## Windows

Required:
- **Microphone**

Pasting uses Windows input injection APIs. Some security software may block it; allow VoiceDictation if needed.
