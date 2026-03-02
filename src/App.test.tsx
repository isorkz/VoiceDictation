import "@testing-library/jest-dom/vitest";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { invoke } from "@tauri-apps/api/core";
import { describe, expect, it, vi } from "vitest";
import App from "./App";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

const baseConfig = {
  azure: { endpoint: "", deployment: "", apiVersion: "2025-03-01-preview", apiKey: "" },
  hotkey: { windows: "Ctrl" },
  thresholds: { holdMs: 180, doubleClickMs: 300 },
  recording: { maxSeconds: 120 },
  insert: { restoreClipboard: false, postfix: "none" },
};

function mockInvoke() {
  vi.mocked(invoke).mockImplementation((cmd: string) => {
    switch (cmd) {
      case "get_config":
        return Promise.resolve(baseConfig);
      case "get_status":
        return Promise.resolve({ state: "Idle", lastError: null });
      case "get_autostart_enabled":
        return Promise.resolve(false);
      case "set_autostart_enabled":
        return Promise.resolve(null);
      case "set_config":
        return Promise.resolve(null);
      case "reset_config":
        return Promise.resolve(baseConfig);
      default:
        return Promise.reject(new Error(`Unexpected invoke: ${cmd}`));
    }
  });
}

describe("App", () => {
  it("edits inputs without crashing", async () => {
    mockInvoke();
    render(<App />);

    const autostart = screen.getByLabelText(/Launch at login/i);
    await waitFor(() => expect(autostart).not.toBeDisabled());

    const endpoint = screen.getByLabelText(/Endpoint/i);
    fireEvent.change(endpoint, { target: { value: "https://example.openai.azure.com" } });
    expect(endpoint).toHaveValue("https://example.openai.azure.com");

    const deployment = screen.getByLabelText(/Deployment/i);
    fireEvent.change(deployment, { target: { value: "gpt-4o-mini-transcribe" } });
    expect(deployment).toHaveValue("gpt-4o-mini-transcribe");

    const apiKey = screen.getByLabelText(/^API key$/i);
    fireEvent.change(apiKey, { target: { value: "test-key" } });
    expect(apiKey).toHaveValue("test-key");

    const holdMs = screen.getByLabelText(/Hold \(ms\)/i);
    fireEvent.change(holdMs, { target: { value: "200" } });
    expect(holdMs).toHaveValue(200);

    const restore = screen.getByLabelText(/Restore clipboard after paste/i);
    await waitFor(() => expect(restore).not.toBeChecked());
    fireEvent.click(restore);
    expect(restore).toBeChecked();

    fireEvent.click(autostart);

    await waitFor(() =>
      expect(vi.mocked(invoke)).toHaveBeenCalledWith("set_autostart_enabled", { enabled: true }),
    );

    const reset = screen.getByRole("button", { name: /Reset settings/i });
    fireEvent.click(reset);

    await waitFor(() => expect(vi.mocked(invoke)).toHaveBeenCalledWith("reset_config"));
  });
});
