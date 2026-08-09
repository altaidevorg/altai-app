import { describe, expect, it } from "vitest";
import { recoveryHintForDiagnosticCode } from "../lib/hostRecovery.js";

describe("recoveryHintForDiagnosticCode", () => {
  it("maps known codes", () => {
    expect(recoveryHintForDiagnosticCode("host.untrusted")).toMatch(/Trust/i);
    expect(recoveryHintForDiagnosticCode("host.missing")).toMatch(/VSIX|agentHostPath/i);
  });
  it("returns undefined for unknown", () => {
    expect(recoveryHintForDiagnosticCode("other")).toBeUndefined();
    expect(recoveryHintForDiagnosticCode(undefined)).toBeUndefined();
  });
});
