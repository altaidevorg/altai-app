import { describe, expect, it } from "vitest";
import {
  ALTAI_RECOVERY_COMMANDS,
  isAltaiRecoveryCommand,
  listRecoveryActions,
} from "../../src/shared/hostRecoveryCommands.js";

describe("host recovery actions", () => {
  it("allowlists open logs, diagnostics, restart, and version", () => {
    expect(listRecoveryActions().map((a) => a.command)).toEqual([
      "altai.openLogs",
      "altai.runDiagnostics",
      "altai.copyDiagnosticsReport",
      "altai.restartAgentHost",
      "altai.showVersionCompatibility",
      "altai.openWalkthrough",
    ]);
    expect(isAltaiRecoveryCommand("altai.openLogs")).toBe(true);
    expect(isAltaiRecoveryCommand("altai.connectProvider")).toBe(true);
    expect(isAltaiRecoveryCommand("altai.clearProviderCredential")).toBe(true);
    expect(isAltaiRecoveryCommand("altai.openWalkthrough")).toBe(true);
    expect(isAltaiRecoveryCommand("workbench.action.quit")).toBe(false);
  });

  it("includes slash-facing credential commands in the allowlist", () => {
    expect(ALTAI_RECOVERY_COMMANDS).toContain("altai.connectProvider");
    expect(ALTAI_RECOVERY_COMMANDS).toContain("altai.clearProviderCredential");
  });

  it("prepends manage workspace trust when host is untrusted", () => {
    expect(
      listRecoveryActions({ diagnosticCode: "host.untrusted" }).map(
        (a) => a.command,
      ),
    ).toEqual([
      "workbench.action.manageWorkspaceTrust",
      "altai.openLogs",
      "altai.runDiagnostics",
      "altai.copyDiagnosticsReport",
      "altai.restartAgentHost",
      "altai.showVersionCompatibility",
      "altai.openWalkthrough",
    ]);
  });

  it("surfaces host path settings when host binary is missing", () => {
    expect(
      listRecoveryActions({ diagnosticCode: "host.missing" })[0]?.command,
    ).toBe("altai.openExtensionSettings");
  });

  it("surfaces open folder when no workspace is open", () => {
    expect(
      listRecoveryActions({ diagnosticCode: "host.no_workspace" })[0]?.command,
    ).toBe("workbench.action.files.openFolder");
    expect(isAltaiRecoveryCommand("workbench.action.files.openFolder")).toBe(
      true,
    );
  });
});
