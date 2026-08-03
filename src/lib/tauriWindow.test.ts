import { describe, expect, it } from "vitest";
import { hasTauriWindowMetadata } from "./tauriWindow";

describe("hasTauriWindowMetadata", () => {
  it("rejects a regular browser global", () => {
    expect(hasTauriWindowMetadata({})).toBe(false);
  });

  it("rejects partial Tauri initialization", () => {
    expect(hasTauriWindowMetadata({ isTauri: true })).toBe(false);
    expect(
      hasTauriWindowMetadata({
        isTauri: true,
        __TAURI_INTERNALS__: { metadata: {} },
      }),
    ).toBe(false);
  });

  it("accepts a fully initialized native WebView", () => {
    expect(
      hasTauriWindowMetadata({
        isTauri: true,
        __TAURI_INTERNALS__: {
          metadata: {
            currentWindow: { label: "main" },
            currentWebview: { label: "main" },
          },
        },
      }),
    ).toBe(true);
  });
});
