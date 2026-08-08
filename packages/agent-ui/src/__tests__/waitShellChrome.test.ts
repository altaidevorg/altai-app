import { describe, expect, it } from "vitest";
import { formatDiagnosticClipboardText } from "../lib/waitShellChrome.js";

describe("formatDiagnosticClipboardText", () => {
  it("builds a multi-line diagnostic report", () => {
    expect(
      formatDiagnosticClipboardText({
        diagnosticCode: "host.untrusted",
        message: "Workspace is not trusted",
        recoveryHint: "Trust this workspace",
      }),
    ).toBe(
      [
        "ALTAI diagnostic: host.untrusted",
        "Message: Workspace is not trusted",
        "Recovery: Trust this workspace",
      ].join("\n"),
    );
  });

  it("returns null when empty", () => {
    expect(formatDiagnosticClipboardText({})).toBeNull();
  });
});
