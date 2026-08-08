import { describe, expect, it, vi } from "vitest";
import { getComposerActionAvailability } from "../lib/composerEnterAction.js";
import { executeComposerSubmit } from "../lib/composerSubmitExecute.js";

const idle = getComposerActionAvailability({
  status: "idle",
  hasDraft: true,
  hasNativeAttachment: false,
  runId: null,
  submitting: false,
});

const running = getComposerActionAvailability({
  status: "streaming",
  hasDraft: true,
  hasNativeAttachment: false,
  runId: "run-1",
  submitting: false,
});

describe("executeComposerSubmit", () => {
  it("returns noop when send is unavailable", async () => {
    const send = vi.fn();
    const result = await executeComposerSubmit({
      action: "send",
      availability: getComposerActionAvailability({
        status: "idle",
        hasDraft: false,
        hasNativeAttachment: false,
        runId: null,
        submitting: false,
      }),
      draft: { value: "", files: [], snippets: [], commands: [] },
      catalog: [],
      sessionId: "s",
      runId: null,
      host: { send, steer: vi.fn() },
    });
    expect(result.kind).toBe("noop");
    expect(send).not.toHaveBeenCalled();
  });

  it("sends via host and reports accepted", async () => {
    const send = vi.fn().mockResolvedValue(true);
    const result = await executeComposerSubmit({
      action: "send",
      availability: idle,
      draft: { value: "hello", files: [], snippets: [], commands: [] },
      catalog: [],
      sessionId: "s1",
      runId: null,
      host: { send, steer: vi.fn() },
    });
    expect(result.kind).toBe("accepted");
    expect(send).toHaveBeenCalledWith(
      expect.objectContaining({
        sessionId: "s1",
        composed: expect.stringContaining("hello"),
        queue: false,
      }),
    );
  });

  it("steers via host when run is active", async () => {
    const steer = vi.fn().mockResolvedValue(true);
    const result = await executeComposerSubmit({
      action: "steer",
      availability: running,
      draft: { value: "nudge", files: [], snippets: [], commands: [] },
      catalog: [],
      sessionId: "s1",
      runId: "run-1",
      host: { send: vi.fn(), steer },
    });
    expect(result.kind).toBe("accepted");
    expect(steer).toHaveBeenCalledWith(
      expect.objectContaining({
        sessionId: "s1",
        runId: "run-1",
        composed: expect.stringContaining("nudge"),
      }),
    );
  });

  it("handles local slash outcomes and toasts", async () => {
    const onToast = vi.fn();
    const result = await executeComposerSubmit({
      action: "send",
      availability: idle,
      draft: { value: "/help", files: [], snippets: [], commands: [] },
      catalog: [],
      sessionId: "s",
      runId: null,
      resolveSlash: () => ({
        kind: "handled",
        toast: "slash done",
      }),
      host: { send: vi.fn(), steer: vi.fn(), onToast },
    });
    expect(result).toEqual({ kind: "handled", toast: "slash done" });
    expect(onToast).toHaveBeenCalledWith("slash done");
  });

  it("reports error when host throws", async () => {
    const onError = vi.fn();
    const err = new Error("boom");
    const result = await executeComposerSubmit({
      action: "send",
      availability: idle,
      draft: { value: "hi", files: [], snippets: [], commands: [] },
      catalog: [],
      sessionId: "s",
      runId: null,
      host: {
        send: async () => {
          throw err;
        },
        steer: vi.fn(),
        onError,
      },
    });
    expect(result.kind).toBe("error");
    expect(onError).toHaveBeenCalledWith({ action: "send", error: err });
  });
});
