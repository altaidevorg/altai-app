import { describe, expect, it } from "vitest";
import {
  composerFollowupVisibility,
  resolveComposerSubmitMode,
} from "../lib/composerFollowup.js";

describe("composerFollowupVisibility", () => {
  it("hides the bar without an active run", () => {
    expect(
      composerFollowupVisibility({
        hasActiveRun: false,
        canStartRun: true,
        canSteer: true,
        canQueue: true,
        hasPrompt: true,
      }).showBar,
    ).toBe(false);
  });

  it("shows steer/queue only with matching capabilities", () => {
    const steersonly = composerFollowupVisibility({
      hasActiveRun: true,
      canStartRun: true,
      canSteer: true,
      canQueue: false,
      hasPrompt: true,
    });
    expect(steersonly.showSteer).toBe(true);
    expect(steersonly.showQueue).toBe(false);
    expect(steersonly.canSteerAction).toBe(true);

    const noText = composerFollowupVisibility({
      hasActiveRun: true,
      canStartRun: true,
      canSteer: true,
      canQueue: true,
      hasPrompt: false,
    });
    expect(noText.canSteerAction).toBe(false);
    expect(noText.canQueueAction).toBe(false);
  });
});

describe("resolveComposerSubmitMode", () => {
  it("starts when idle", () => {
    expect(
      resolveComposerSubmitMode({
        hasActiveRun: false,
        canStartRun: true,
        canSteer: true,
        canQueue: true,
        hasPrompt: true,
      }),
    ).toBe("start");
  });

  it("prefers queue on enter during an active run", () => {
    expect(
      resolveComposerSubmitMode({
        hasActiveRun: true,
        canStartRun: true,
        canSteer: true,
        canQueue: true,
        hasPrompt: true,
      }),
    ).toBe("queue");
  });

  it("steers when meta is held", () => {
    expect(
      resolveComposerSubmitMode({
        hasActiveRun: true,
        canStartRun: true,
        canSteer: true,
        canQueue: true,
        hasPrompt: true,
        preferSteer: true,
      }),
    ).toBe("steer");
  });
});
