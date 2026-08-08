import { describe, expect, it } from "vitest";
import {
  canMountWorkspaceTopbar,
  workspaceTopbarInboxOpen,
  workspaceTopbarWorkOpen,
} from "../lib/workspaceTopbarChrome.js";

describe("canMountWorkspaceTopbar", () => {
  it("requires at least one Operations domain capability", () => {
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
    ).toBe(true);
    expect(
      canMountWorkspaceTopbar({
        taskRuns: false,
        automations: false,
        inbox: true,
      }),
    ).toBe(true);
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
