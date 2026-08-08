import { describe, expect, it } from "vitest";
import { hostStatusBarPresentation } from "../lib/hostStatusBar.js";

describe("hostStatusBarPresentation", () => {
  it("hides when ready", () => {
    expect(
      hostStatusBarPresentation({ status: "ready", message: "ok" }).show,
    ).toBe(false);
  });
  it("wires trust manage for untrusted diagnostic", () => {
    const p = hostStatusBarPresentation({
      status: "error",
      message: "blocked",
      diagnosticCode: "host.untrusted",
    });
    expect(p.command).toBe("workbench.action.manageWorkspaceTrust");
    expect(p.warning).toBe(true);
  });
});
