import { describe, expect, it } from "vitest";
import {
  canEnableComposerSend,
  canEnableComposerStop,
  composerSubmitChromeMode,
} from "../lib/composerSubmitChrome.js";

describe("composerSubmitChromeMode", () => {
  it("uses Stop while busy or when a run is active", () => {
    expect(
      composerSubmitChromeMode({ busy: true, hasActiveRun: false }),
    ).toBe("stop");
    expect(
      composerSubmitChromeMode({ busy: false, hasActiveRun: true }),
    ).toBe("stop");
    expect(
      composerSubmitChromeMode({ busy: false, hasActiveRun: false }),
    ).toBe("send");
  });
});

describe("canEnableComposerSend", () => {
  it("requires a prompt and an available run action", () => {
    expect(
      canEnableComposerSend({
        busy: false,
        hasPrompt: true,
        canStartRun: true,
        hasActiveRun: false,
        canSteer: false,
        canQueue: false,
      }),
    ).toBe(true);
    expect(
      canEnableComposerSend({
        busy: false,
        hasPrompt: false,
        canStartRun: true,
        hasActiveRun: false,
        canSteer: false,
        canQueue: false,
      }),
    ).toBe(false);
    expect(
      canEnableComposerSend({
        busy: false,
        hasPrompt: true,
        canStartRun: false,
        hasActiveRun: true,
        canSteer: true,
        canQueue: false,
      }),
    ).toBe(true);
  });
});

describe("canEnableComposerStop", () => {
  it("disables while cancellation is already in flight", () => {
    expect(
      canEnableComposerStop({
        hasActiveRun: true,
        busy: true,
        isCancelling: true,
      }),
    ).toBe(false);
    expect(
      canEnableComposerStop({ hasActiveRun: true, busy: false }),
    ).toBe(true);
  });
});
