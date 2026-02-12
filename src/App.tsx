import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

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

function Button({
  children,
  onClick,
  disabled,
  variant = "secondary",
}: {
  children: ReactNode;
  onClick: () => void;
  disabled?: boolean;
  variant?: "primary" | "secondary" | "danger";
}) {
  const base =
    "inline-flex items-center justify-center gap-2 rounded-xl px-4 py-2 text-sm font-medium transition focus:outline-none focus-visible:ring-2 focus-visible:ring-sky-500 focus-visible:ring-offset-2 focus-visible:ring-offset-slate-50 dark:focus-visible:ring-offset-slate-950 disabled:cursor-not-allowed disabled:opacity-50";
  const styles =
    variant === "primary"
      ? "bg-sky-600 text-white shadow-sm hover:bg-sky-700"
      : variant === "danger"
        ? "bg-rose-600 text-white shadow-sm hover:bg-rose-700"
        : "border border-slate-200 bg-white text-slate-900 shadow-sm hover:bg-slate-50 dark:border-slate-800 dark:bg-slate-900 dark:text-slate-50 dark:hover:bg-slate-800";
  return (
    <button type="button" onClick={onClick} disabled={disabled} className={`${base} ${styles}`}>
      {children}
    </button>
  );
}

function Card({
  title,
  description,
  children,
}: {
  title: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <section className="rounded-2xl border border-slate-200 bg-white p-5 shadow-sm dark:border-slate-800 dark:bg-slate-900">
      <div className="mb-4">
        <h2 className="text-base font-semibold text-slate-900 dark:text-slate-50">{title}</h2>
        {description ? (
          <p className="mt-1 text-sm text-slate-600 dark:text-slate-300">{description}</p>
        ) : null}
      </div>
      {children}
    </section>
  );
}

function Input({
  value,
  onChange,
  placeholder,
  type = "text",
  min,
}: {
  value: string | number;
  onChange: (value: string) => void;
  placeholder?: string;
  type?: "text" | "number";
  min?: number;
}) {
  return (
    <input
      className="mt-2 w-full rounded-xl border border-slate-200 bg-white px-3 py-2 text-sm text-slate-900 shadow-sm outline-none transition focus:border-sky-500 focus:ring-2 focus:ring-sky-500/20 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-50"
      value={value}
      type={type}
      min={min}
      onChange={(e) => onChange(e.currentTarget.value)}
      placeholder={placeholder}
    />
  );
}

function Switch({
  checked,
  onChange,
  disabled,
}: {
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <span className="relative inline-flex h-6 w-11 items-center">
      <input
        type="checkbox"
        className="peer sr-only"
        checked={checked}
        disabled={disabled}
        onChange={(e) => onChange(e.currentTarget.checked)}
      />
      <span className="h-6 w-11 rounded-full border border-slate-200 bg-slate-200 transition peer-checked:border-sky-600 peer-checked:bg-sky-600 peer-disabled:cursor-not-allowed peer-disabled:opacity-50 dark:border-slate-700 dark:bg-slate-700" />
      <span className="pointer-events-none absolute left-0.5 top-0.5 h-5 w-5 rounded-full bg-white shadow-sm transition-transform peer-checked:translate-x-5" />
    </span>
  );
}

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
  const isBusy = loading || saving;

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
    <main className="min-h-screen px-4 py-10">
      <div className="mx-auto w-full max-w-5xl">
        <header className="mb-8 flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
          <div>
            <h1 className="text-2xl font-semibold tracking-tight text-slate-900 dark:text-slate-50">
              VoiceDictation
            </h1>
            <p className="mt-1 text-sm text-slate-600 dark:text-slate-300">
              Settings (API key is read from env only).
            </p>
          </div>

          <div className="flex flex-wrap items-center gap-2">
            <span className="inline-flex items-center gap-2 rounded-full border border-slate-200 bg-white px-3 py-1 text-sm text-slate-700 shadow-sm dark:border-slate-800 dark:bg-slate-900 dark:text-slate-200">
              <span className="h-2 w-2 rounded-full bg-emerald-500" />
              Status: <span className="font-medium">{status.state}</span>
            </span>
            {status.lastError ? (
              <span className="inline-flex items-center rounded-full border border-rose-200 bg-rose-50 px-3 py-1 text-sm text-rose-800 dark:border-rose-900/50 dark:bg-rose-950 dark:text-rose-200">
                {status.lastError}
              </span>
            ) : null}
          </div>
        </header>

        {error ? (
          <div
            role="alert"
            className="mb-6 rounded-2xl border border-rose-200 bg-rose-50 p-4 text-sm text-rose-800 dark:border-rose-900/50 dark:bg-rose-950 dark:text-rose-200"
          >
            {error}
          </div>
        ) : null}

        <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
          <div className="lg:col-span-2">
            <Card title="Azure" description="Endpoint / deployment / API version (API key comes from env).">
              <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
                <label className="sm:col-span-2">
                  <span className="text-sm font-medium text-slate-800 dark:text-slate-200">
                    Endpoint
                  </span>
                  <Input
                    value={config.azure.endpoint}
                    onChange={(value) => {
                      setConfig((prev) => ({
                        ...prev,
                        azure: { ...prev.azure, endpoint: value },
                      }));
                    }}
                    placeholder="https://<resource>.openai.azure.com"
                  />
                </label>

                <label className="sm:col-span-2">
                  <span className="text-sm font-medium text-slate-800 dark:text-slate-200">
                    Deployment
                  </span>
                  <Input
                    value={config.azure.deployment}
                    onChange={(value) => {
                      setConfig((prev) => ({
                        ...prev,
                        azure: { ...prev.azure, deployment: value },
                      }));
                    }}
                    placeholder="gpt-4o-mini-transcribe (deployment name)"
                  />
                </label>

                <label>
                  <span className="text-sm font-medium text-slate-800 dark:text-slate-200">
                    API version
                  </span>
                  <Input
                    value={config.azure.apiVersion}
                    onChange={(value) => {
                      setConfig((prev) => ({
                        ...prev,
                        azure: { ...prev.azure, apiVersion: value },
                      }));
                    }}
                    placeholder="2025-03-01-preview"
                  />
                </label>

                <div className="flex items-end">
                  <p className="text-sm text-slate-700 dark:text-slate-200">
                    API key: <code className="rounded bg-slate-100 px-1.5 py-0.5 dark:bg-slate-800">AZURE_OPENAI_API_KEY</code>{" "}
                    {apiKeyPresent === null ? (
                      <span className="text-slate-500 dark:text-slate-400">(checking...)</span>
                    ) : apiKeyPresent ? (
                      <span className="text-emerald-700 dark:text-emerald-300">(detected)</span>
                    ) : (
                      <span className="text-rose-700 dark:text-rose-300">(not detected)</span>
                    )}
                  </p>
                </div>
              </div>
            </Card>
          </div>

          <Card title="Startup" description="Launch VoiceDictation at login.">
            <label className="flex items-center justify-between gap-4">
              <span className="text-sm font-medium text-slate-800 dark:text-slate-200">
                Launch at login
              </span>
              <Switch
                checked={autostartEnabled ?? false}
                disabled={autostartEnabled === null || isBusy}
                onChange={(checked) => {
                  void setAutostart(checked);
                }}
              />
            </label>
          </Card>

          <Card title="Hotkey" description="Windows default hotkey (takes effect after restart).">
            <div className="space-y-4">
              <label className="block">
                <span className="text-sm font-medium text-slate-800 dark:text-slate-200">
                  Windows default hotkey
                </span>
                <Input
                  value={config.hotkey.windows}
                  onChange={(value) => {
                    setConfig((prev) => ({
                      ...prev,
                      hotkey: { ...prev.hotkey, windows: value },
                    }));
                  }}
                  placeholder="Win+Shift+D"
                />
              </label>

              <div className="rounded-xl border border-slate-200 bg-slate-50 p-3 text-sm text-slate-700 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-200">
                <div className="font-medium text-slate-800 dark:text-slate-200">macOS shortcut</div>
                <div className="mt-1">
                  Use <code className="rounded bg-white px-1.5 py-0.5 dark:bg-slate-900">Language (Globe/Fn)</code>{" "}
                  key:
                  <span className="ml-2 text-slate-600 dark:text-slate-300">
                    hold = push-to-talk, double-click = toggle.
                  </span>
                </div>
              </div>
            </div>
          </Card>

          <Card title="Thresholds" description="Tune hold and double-click timings.">
            <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
              <label>
                <span className="text-sm font-medium text-slate-800 dark:text-slate-200">
                  Hold (ms)
                </span>
                <Input
                  type="number"
                  min={0}
                  value={config.thresholds.holdMs}
                  onChange={(value) => {
                    setConfig((prev) => ({
                      ...prev,
                      thresholds: { ...prev.thresholds, holdMs: Number(value) },
                    }));
                  }}
                />
              </label>

              <label>
                <span className="text-sm font-medium text-slate-800 dark:text-slate-200">
                  Double-click (ms)
                </span>
                <Input
                  type="number"
                  min={0}
                  value={config.thresholds.doubleClickMs}
                  onChange={(value) => {
                    setConfig((prev) => ({
                      ...prev,
                      thresholds: { ...prev.thresholds, doubleClickMs: Number(value) },
                    }));
                  }}
                />
              </label>
            </div>
          </Card>

          <Card title="Recording" description="Safety limit for max recording duration.">
            <label>
              <span className="text-sm font-medium text-slate-800 dark:text-slate-200">
                Max seconds
              </span>
              <Input
                type="number"
                min={1}
                value={config.recording.maxSeconds}
                onChange={(value) => {
                  setConfig((prev) => ({
                    ...prev,
                    recording: { ...prev.recording, maxSeconds: Number(value) },
                  }));
                }}
              />
            </label>
          </Card>

          <Card title="Insert" description="Clipboard behavior after pasting transcription.">
            <label className="flex items-center justify-between gap-4">
              <span className="text-sm font-medium text-slate-800 dark:text-slate-200">
                Restore clipboard after paste
              </span>
              <Switch
                checked={config.insert.restoreClipboard}
                onChange={(checked) => {
                  setConfig((prev) => ({
                    ...prev,
                    insert: { ...prev.insert, restoreClipboard: checked },
                  }));
                }}
              />
            </label>
          </Card>
        </div>

        <div className="sticky bottom-0 mt-8 border-t border-slate-200/80 bg-slate-50/80 py-4 backdrop-blur dark:border-slate-800/80 dark:bg-slate-950/70">
          <div className="mx-auto flex w-full max-w-5xl flex-wrap gap-2 px-0">
            <Button
              onClick={() => void toggleRecording()}
              disabled={isBusy}
              variant={status.state === "Recording" ? "danger" : "primary"}
            >
              {status.state === "Recording" ? "Stop" : "Start"}
            </Button>
            <Button onClick={() => void reload()} disabled={isBusy}>
              Reset settings
            </Button>
            <Button onClick={() => void save()} disabled={!canSave} variant="primary">
              Save
            </Button>
            <Button onClick={() => void testTranscription()} disabled={isBusy}>
              Test connection (1.2s)
            </Button>
          </div>
        </div>

        {testResult ? (
          <div className="mt-6">
            <Card title="Transcript">
              <pre className="whitespace-pre-wrap rounded-xl border border-slate-200 bg-slate-50 p-3 text-sm text-slate-900 dark:border-slate-800 dark:bg-slate-950 dark:text-slate-50">
                {testResult}
              </pre>
            </Card>
          </div>
        ) : null}
      </div>
    </main>
  );
}

export default App;
