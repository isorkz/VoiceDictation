# Environment variables

## `AZURE_OPENAI_API_KEY`

VoiceDictation reads the Azure OpenAI API key from `AZURE_OPENAI_API_KEY` only. The key is not stored in `config.json` and is never written to disk by this app.

### macOS

There are two common scenarios:

1) **Terminal-launched (development)**

If you start the app from a terminal, exporting the variable in the same terminal session works:
```bash
export AZURE_OPENAI_API_KEY="..."
npm run tauri dev
```

2) **GUI-launched (Finder / autostart)**

GUI apps may not inherit your shell environment. For a reliable setup, use a LaunchAgent.

Create `~/Library/LaunchAgents/com.intzero.voicedictation.env.plist`:
```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
  <dict>
    <key>Label</key>
    <string>com.intzero.voicedictation.env</string>
    <key>ProgramArguments</key>
    <array>
      <string>/bin/launchctl</string>
      <string>setenv</string>
      <string>AZURE_OPENAI_API_KEY</string>
      <string>YOUR_KEY_HERE</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
  </dict>
</plist>
```

Load it:
```bash
launchctl load -w ~/Library/LaunchAgents/com.intzero.voicedictation.env.plist
```

Log out/in (or reboot), then start VoiceDictation normally.

### Windows

Set a **user** environment variable `AZURE_OPENAI_API_KEY` in:
System Properties → Advanced → Environment Variables…

After setting it, restart VoiceDictation (you may need to log out/in).

## Security note

Environment variables are not a perfect secret storage mechanism. If you need stronger protection, use OS credential storage (Keychain / Credential Manager). VoiceDictation v1 intentionally does not implement that.

