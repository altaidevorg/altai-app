import { describe, expect, it } from "vitest";
import {
  getComposerActionAvailability,
  resolveComposerEnterAction,
} from "../lib/composerEnterAction.js";

function availability(
  overrides: Partial<Parameters<typeof getComposerActionAvailability>[0]> = {},
) {
  return getComposerActionAvailability({
    status: "idle",
    hasDraft: true,
    hasNativeAttachment: false,
    runId: null,
    submitting: false,
    ...overrides,
  });
}

describe("getComposerActionAvailability", () => {
  it("maps Enter to send while idle and Queue next during an active run", () => {
    expect(
      resolveComposerEnterAction({
        availability: availability(),
        shiftKey: false,
        modifierKey: false,
      }),
    ).toBe("send");
    expect(
      resolveComposerEnterAction({
        availability: availability({ status: "streaming", runId: "run-1" }),
        shiftKey: false,
        modifierKey: false,
      }),
    ).toBe("queue");
  });

  it("maps Cmd/Ctrl+Enter to Steer only for a steerable active run", () => {
    const running = availability({ status: "thinking", runId: "run-1" });
    expect(
      resolveComposerEnterAction({
        availability: running,
        shiftKey: false,
        modifierKey: true,
      }),
    ).toBe("steer");
  });

  it("keeps Queue next available while cancellation is acknowledged", () => {
    const cancelling = availability({ status: "cancelling", runId: "run-1" });
    expect(cancelling.isCancelling).toBe(true);
    expect(cancelling.canSteer).toBe(false);
    expect(cancelling.canQueue).toBe(true);
  });

  it("does not allow steering with native attachments", () => {
    const withAttachment = availability({
      status: "streaming",
      runId: "run-1",
      hasNativeAttachment: true,
    });
    expect(withAttachment.canSteer).toBe(false);
    expect(withAttachment.canQueue).toBe(true);
  });

  it("treats awaiting-approval as busy for queue/send", () => {
    const awaiting = availability({ status: "awaiting-approval", runId: "r1" });
    expect(awaiting.isBusy).toBe(true);
    expect(awaiting.canSend).toBe(false);
    expect(awaiting.canQueue).toBe(true);
  });
});
