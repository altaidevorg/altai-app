import { describe, expect, it } from "vitest";
import {
  COMPOSER_QUEUE_CONTROL_TITLE,
  composerDesktopPlaceholder,
  composerFollowupBarHint,
  composerSteerControlTitle,
} from "../lib/composerDesktopChrome.js";

describe("composerDesktopChrome", () => {
  it("builds placeholder for busy vs idle", () => {
    expect(composerDesktopPlaceholder(true)).toContain("follow-up");
    expect(composerDesktopPlaceholder(false)).toContain("@ files");
  });

  it("builds follow-up hint and steer title", () => {
    expect(
      composerFollowupBarHint({ isCancelling: true, canSteer: false }),
    ).toContain("Cancellation requested");
    expect(
      composerFollowupBarHint({ isCancelling: false, canSteer: true }),
    ).toContain("steers this run");
    expect(
      composerFollowupBarHint({ isCancelling: false, canSteer: false }),
    ).toContain("after the active run ends");
    expect(composerSteerControlTitle(true)).toContain("images or PDFs");
    expect(composerSteerControlTitle(false)).toContain("safe boundary");
    expect(COMPOSER_QUEUE_CONTROL_TITLE).toContain("terminates");
  });
});
