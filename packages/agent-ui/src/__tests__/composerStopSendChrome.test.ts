import { describe, expect, it } from "vitest";
import {
  COMPOSER_SEND_ARIA_LABEL,
  COMPOSER_SEND_TOOLTIP,
  composerStopAriaLabel,
  composerStopControlLabel,
} from "../lib/composerStopSendChrome.js";

describe("composerStopSendChrome", () => {
  it("labels stop and send controls", () => {
    expect(composerStopControlLabel(true)).toBe("Stopping");
    expect(composerStopControlLabel(false)).toBe("Stop");
    expect(composerStopAriaLabel(true)).toBe("Cancelling");
    expect(composerStopAriaLabel(false)).toBe("Stop");
    expect(COMPOSER_SEND_TOOLTIP).toContain("Enter");
    expect(COMPOSER_SEND_ARIA_LABEL).toBe("Send");
  });
});
