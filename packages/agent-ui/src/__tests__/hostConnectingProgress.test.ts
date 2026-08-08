import { describe, expect, it } from "vitest";
import { hostConnectingProgressPresentation } from "../lib/hostConnectingProgress.js";

describe("hostConnectingProgressPresentation", () => {
  it("hides unless connecting", () => {
    expect(hostConnectingProgressPresentation({ status: "ready" }).show).toBe(
      false,
    );
  });
  it("shows title with optional detail", () => {
    expect(
      hostConnectingProgressPresentation({ status: "connecting" }).title,
    ).toContain("starting");
    expect(
      hostConnectingProgressPresentation({
        status: "connecting",
        message: " handshake ",
      }).title,
    ).toBe("ALTAI agent host: handshake");
  });
});
