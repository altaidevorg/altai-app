import { describe, expect, it } from "vitest";
import {
  remainingTextAfterAcceptedDispatch,
  resolveComposerEnterAction,
} from "../lib/composerEnterAction.js";

describe("resolveComposerEnterAction", () => {
  const idle = {
    isRunning: false,
    isBusy: false,
    canSend: true,
    canSteer: false,
    canQueue: false,
  };

  it("sends on enter when idle and sendable", () => {
    expect(
      resolveComposerEnterAction({
        availability: idle,
        shiftKey: false,
        modifierKey: false,
      }),
    ).toBe("send");
  });

  it("ignores shift+enter", () => {
    expect(
      resolveComposerEnterAction({
        availability: idle,
        shiftKey: true,
        modifierKey: false,
      }),
    ).toBeNull();
  });

  it("steers with modifier while running", () => {
    expect(
      resolveComposerEnterAction({
        availability: {
          isRunning: true,
          isBusy: true,
          canSend: false,
          canSteer: true,
          canQueue: true,
        },
        shiftKey: false,
        modifierKey: true,
      }),
    ).toBe("steer");
  });

  it("queues while busy without modifier", () => {
    expect(
      resolveComposerEnterAction({
        availability: {
          isRunning: true,
          isBusy: true,
          canSend: false,
          canSteer: true,
          canQueue: true,
        },
        shiftKey: false,
        modifierKey: false,
      }),
    ).toBe("queue");
  });
});

describe("remainingTextAfterAcceptedDispatch", () => {
  it("clears when draft was unchanged", () => {
    expect(remainingTextAfterAcceptedDispatch("hello", "hello", true)).toBe("");
  });

  it("keeps trailing typed text", () => {
    expect(
      remainingTextAfterAcceptedDispatch("hello world", "hello", false),
    ).toBe("world");
  });
});
