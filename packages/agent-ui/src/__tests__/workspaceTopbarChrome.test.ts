import { describe, expect, it } from "vitest";
import {
  canMountWorkspaceTopbar,
  workspaceTopbarInboxOpen,
  workspaceTopbarWorkOpen,
} from "../lib/workspaceTopbarChrome.js";

describe("canMountWorkspaceTopbar", () => {
  it("mounts only when the run inspector is available", () => {
    expect(
      canMountWorkspaceTopbar({
        taskRuns: false,
        automations: false,
        inbox: false,
      }),
    ).toBe(false);
    expect(
      canMountWorkspaceTopbar({
        taskRuns: true,
        automations: false,
        inbox: false,
      }),
    ).toBe(false);
    expect(
      canMountWorkspaceTopbar({
        taskRuns: false,
        automations: false,
        inbox: true,
      }),
    ).toBe(false);
    expect(
      canMountWorkspaceTopbar({
        taskRuns: false,
        automations: false,
        inbox: false,
        inspector: true,
      }),
    ).toBe(true);
  });
});

describe("workspace topbar pressed state", () => {
  it("marks work or inbox open only on the Operations surface", () => {
    expect(workspaceTopbarWorkOpen("chat", "work")).toBe(false);
    expect(workspaceTopbarWorkOpen("operations", "work")).toBe(true);
    expect(workspaceTopbarInboxOpen("operations", "inbox")).toBe(true);
    expect(workspaceTopbarInboxOpen("operations", "overview")).toBe(false);
  });
});
