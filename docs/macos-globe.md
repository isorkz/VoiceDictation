# macOS Globe/Fn (Language key)

VoiceDictation supports:
- hold Globe/Fn to start recording, release to stop + transcribe + paste
- double-click Globe/Fn to toggle recording on/off

## Known limitations

The Globe/Fn key is handled by macOS for system features (input source switching, emoji picker, etc.).
Depending on your macOS version, keyboard model, and system settings, macOS may still react to the key even when VoiceDictation is listening.

VoiceDictation tries to **only swallow events after a hold/double-click trigger is detected**, so a single tap is still handled by macOS by default.

## Recommended system settings

If macOS input switching interferes with VoiceDictation triggers, change the Globe key behavior:
System Settings → Keyboard → (Keyboard Shortcuts / Press Globe key to …)

Pick an action that does not conflict with your usage (for example, "Do Nothing" if available).

