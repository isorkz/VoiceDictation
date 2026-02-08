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

If a feature is not working, check:
System Settings → Privacy & Security → (Microphone / Input Monitoring / Accessibility)

## Windows

Required:
- **Microphone**

Pasting uses Windows input injection APIs. Some security software may block it; allow VoiceDictation if needed.

