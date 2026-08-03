import { afterEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

import {
  openSettingsWindow,
  registerOpenSettings,
} from "./openSettingsWindow";

type TauriTestGlobal = typeof globalThis & {
  isTauri?: boolean;
  __TAURI_INTERNALS__?: unknown;
};

const testGlobal = globalThis as TauriTestGlobal;

afterEach(() => {
  delete testGlobal.isTauri;
  delete testGlobal.__TAURI_INTERNALS__;
  invokeMock.mockReset();
});

describe("openSettingsWindow", () => {
  it("uses the registered in-app settings surface in a browser", async () => {
    const open = vi.fn();
    const unregister = registerOpenSettings(open);

    await openSettingsWindow("models");

    expect(open).toHaveBeenCalledWith("models");
    expect(invokeMock).not.toHaveBeenCalled();
    unregister();
  });

  it("invokes the native settings window when Tauri metadata exists", async () => {
    testGlobal.isTauri = true;
    testGlobal.__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: "main" },
        currentWebview: { label: "main" },
      },
    };
    invokeMock.mockResolvedValue(undefined);
    const open = vi.fn();
    const unregister = registerOpenSettings(open);

    await openSettingsWindow("agents");

    expect(invokeMock).toHaveBeenCalledWith("open_settings_window", {
      tab: "agents",
    });
    expect(open).not.toHaveBeenCalled();
    unregister();
  });

  it("falls back when invoking Tauri throws synchronously", async () => {
    testGlobal.isTauri = true;
    testGlobal.__TAURI_INTERNALS__ = {
      metadata: {
        currentWindow: { label: "main" },
        currentWebview: { label: "main" },
      },
    };
    invokeMock.mockImplementation(() => {
      throw new TypeError("native bridge unavailable");
    });
    const open = vi.fn();
    const unregister = registerOpenSettings(open);

    await openSettingsWindow("general");

    expect(open).toHaveBeenCalledWith("general");
    unregister();
  });
});
