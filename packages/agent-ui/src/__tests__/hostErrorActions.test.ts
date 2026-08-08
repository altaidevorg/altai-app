import { describe, expect, it } from "vitest";
import {
  hostErrorActionCommands,
  hostRecoveredActionCommands,
  shouldPromptHostErrorActions,
} from "../lib/hostErrorActions.js";

describe("hostErrorActions", () => {
  it("prompts only on transition into error", () => {
    expect(shouldPromptHostErrorActions("ready", "error")).toBe(true);
    expect(shouldPromptHostErrorActions("error", "error")).toBe(false);
  });
  it("picks trust commands for untrusted diagnostics", () => {
    expect(hostErrorActionCommands({ diagnosticCode: "host.untrusted" })).toEqual(
      [
        "workbench.action.manageWorkspaceTrust",
        "altai.runDiagnostics",
      ],
    );
  });
  it("offers open panel after recovery", () => {
    expect(hostRecoveredActionCommands()).toEqual(["altai.openSidePanel"]);
  });
});
