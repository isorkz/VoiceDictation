import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./App.css";

type InsertPostfix = "none";

type Config = {
  azure: {
    endpoint: string;
    deployment: string;
    apiVersion: string;
  };
  hotkey: {
    windows: string;
  };
  thresholds: {
    holdMs: number;
    doubleClickMs: number;
  };
  recording: {
    maxSeconds: number;
  };
  insert: {
    restoreClipboard: boolean;
    postfix: InsertPostfix;
  };
};

type Status = {
  state: string;
  lastError?: string | null;
};

const defaultConfig: Config = {
  azure: { endpoint: "", deployment: "", apiVersion: "2025-03-01-preview" },
  hotkey: { windows: "Win+Shift+D" },
  thresholds: { holdMs: 180, doubleClickMs: 300 },
  recording: { maxSeconds: 120 },
  insert: { restoreClipboard: true, postfix: "none" },
};

function App() {
  const [config, setConfig] = useState<Config>(defaultConfig);
  const [apiKeyPresent, setApiKeyPresent] = useState<boolean | null>(null);
  const [status, setStatus] = useState<Status>({ state: "Idle", lastError: null });
  const [autostartEnabled, setAutostartEnabled] = useState<boolean | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [testResult, setTestResult] = useState<string | null>(null);

  const canSave = useMemo(() => !loading && !saving, [loading, saving]);

  async function reload() {
    setLoading(true);
    setError(null);
    setTestResult(null);
    try {
      const [loadedConfig, keyStatus, loadedStatus] = await Promise.all([
        invoke<Config>("get_config"),
        invoke<{ present: boolean }>("check_api_key"),
        invoke<Status>("get_status"),
      ]);
      setConfig(loadedConfig);
      setApiKeyPresent(keyStatus.present);
      setStatus(loadedStatus);
      const enabled = await invoke<boolean>("get_autostart_enabled");
      setAutostartEnabled(enabled);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function save() {
    setSaving(true);
    setError(null);
    setTestResult(null);
    try {
      await invoke("set_config", { config });
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function testTranscription() {
    setError(null);
    setTestResult(null);
    try {
      const text = await invoke<string>("test_transcription");
      setTestResult(text);
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    void reload();
  }, []);

  useEffect(() => {
    const unlistenStatus = listen<Status>("status_changed", (event) => {
      setStatus(event.payload);
    });
    const unlistenTranscript = listen<string>("transcript_ready", (event) => {
      setTestResult(event.payload);
    });
    const unlistenError = listen<string>("error", (event) => {
      setError(event.payload);
    });

    return () => {
      void unlistenStatus.then((f) => f());
      void unlistenTranscript.then((f) => f());
      void unlistenError.then((f) => f());
    };
  }, []);

  async function toggleRecording() {
    setError(null);
    try {
      await invoke("toggle_recording");
    } catch (e) {
      setError(String(e));
    }
  }

  async function setAutostart(next: boolean) {
    setError(null);
    try {
      await invoke("set_autostart_enabled", { enabled: next });
      setAutostartEnabled(next);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <main className="container">
      <h1>VoiceDictation</h1>

      <p>Configuration (API key is read from env only).</p>
      <p>
        Status: <strong>{status.state}</strong>
        {status.lastError ? <span style={{ color: "crimson" }}> ({status.lastError})</span> : null}
      </p>

      {error ? <p style={{ color: "crimson" }}>{error}</p> : null}

      <section style={{ textAlign: "left", width: "min(860px, 100%)" }}>
        <h2>Azure</h2>
        <label>
          Endpoint
          <input
            value={config.azure.endpoint}
            onChange={(e) =>
              setConfig((prev) => ({
                ...prev,
                azure: { ...prev.azure, endpoint: e.currentTarget.value },
              }))
            }
            placeholder="https://<resource>.openai.azure.com"
          />
        </label>
        <label>
          Deployment
          <input
            value={config.azure.deployment}
            onChange={(e) =>
              setConfig((prev) => ({
                ...prev,
                azure: { ...prev.azure, deployment: e.currentTarget.value },
              }))
            }
            placeholder="gpt-4o-mini-transcribe (deployment name)"
          />
        </label>
        <label>
          API version
          <input
            value={config.azure.apiVersion}
            onChange={(e) =>
              setConfig((prev) => ({
                ...prev,
                azure: { ...prev.azure, apiVersion: e.currentTarget.value },
              }))
            }
            placeholder="2025-03-01-preview"
          />
        </label>
        <p>
          API key: <code>AZURE_OPENAI_API_KEY</code>{" "}
          {apiKeyPresent === null ? "(checking...)" : apiKeyPresent ? "(detected)" : "(not detected)"}
        </p>

        <h2>Startup</h2>
        <label style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <input
            type="checkbox"
            checked={autostartEnabled ?? false}
            disabled={autostartEnabled === null || loading || saving}
            onChange={(e) => void setAutostart(e.currentTarget.checked)}
          />
          Launch at login
        </label>

        <h2>Hotkey</h2>
        <label>
          Windows default hotkey
          <input
            value={config.hotkey.windows}
            onChange={(e) =>
              setConfig((prev) => ({
                ...prev,
                hotkey: { ...prev.hotkey, windows: e.currentTarget.value },
              }))
            }
            placeholder="Win+Shift+D"
          />
        </label>

        <h2>Thresholds</h2>
        <label>
          Hold (ms)
          <input
            type="number"
            value={config.thresholds.holdMs}
            min={0}
            onChange={(e) =>
              setConfig((prev) => ({
                ...prev,
                thresholds: { ...prev.thresholds, holdMs: Number(e.currentTarget.value) },
              }))
            }
          />
        </label>
        <label>
          Double-click (ms)
          <input
            type="number"
            value={config.thresholds.doubleClickMs}
            min={0}
            onChange={(e) =>
              setConfig((prev) => ({
                ...prev,
                thresholds: { ...prev.thresholds, doubleClickMs: Number(e.currentTarget.value) },
              }))
            }
          />
        </label>

        <h2>Recording</h2>
        <label>
          Max seconds
          <input
            type="number"
            value={config.recording.maxSeconds}
            min={1}
            onChange={(e) =>
              setConfig((prev) => ({
                ...prev,
                recording: { ...prev.recording, maxSeconds: Number(e.currentTarget.value) },
              }))
            }
          />
        </label>

        <h2>Insert</h2>
        <label style={{ display: "flex", gap: 8, alignItems: "center" }}>
          <input
            type="checkbox"
            checked={config.insert.restoreClipboard}
            onChange={(e) =>
              setConfig((prev) => ({
                ...prev,
                insert: { ...prev.insert, restoreClipboard: e.currentTarget.checked },
              }))
            }
          />
          Restore clipboard after paste
        </label>
      </section>

      <div className="row" style={{ gap: 12 }}>
        <button type="button" onClick={toggleRecording} disabled={loading || saving}>
          {status.state === "Recording" ? "Stop" : "Start"}
        </button>
        <button type="button" onClick={reload} disabled={loading || saving}>
          Reload
        </button>
        <button type="button" onClick={save} disabled={!canSave}>
          Save
        </button>
        <button type="button" onClick={testTranscription} disabled={loading || saving}>
          Test transcription
        </button>
      </div>

      {testResult ? (
        <section style={{ textAlign: "left", width: "min(860px, 100%)", marginTop: 16 }}>
          <h2>Transcript</h2>
          <pre style={{ whiteSpace: "pre-wrap" }}>{testResult}</pre>
        </section>
      ) : null}
    </main>
  );
}

export default App;
