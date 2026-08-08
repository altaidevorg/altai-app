import { describe, expect, it } from "vitest";
import {
  hostStatusPillLabel,
  shouldShowHostSubtitle,
} from "../lib/hostChromeLabels.js";

describe("hostChromeLabels", () => {
  it("labels lifecycle statuses", () => {
    expect(hostStatusPillLabel("ready")).toBe("Ready");
    expect(hostStatusPillLabel("error")).toBe("Error");
    expect(hostStatusPillLabel("starting")).toBe("Starting…");
  });

  it("hides generic ready subtitles", () => {
    expect(shouldShowHostSubtitle("ready", "ALTAI host ready")).toBe(false);
    expect(shouldShowHostSubtitle("ready", "Host path missing")).toBe(true);
    expect(shouldShowHostSubtitle("error", "Something")).toBe(true);
  });
});
