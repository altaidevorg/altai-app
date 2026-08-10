import { describe, expect, it } from "vitest";
import {
  canCreateAutomationDraft,
  canCreateTaskDraft,
  resolveRunModelIdFromCandidates,
} from "../lib/opsCreateChrome.js";

describe("canCreateAutomationDraft", () => {
  it("requires owner, message, not creating, no schedule error", () => {
    expect(
      canCreateAutomationDraft({
        ownerChatId: "c",
        message: " hi ",
        creating: false,
        scheduleError: null,
      }),
    ).toBe(true);
    expect(
      canCreateAutomationDraft({
        ownerChatId: null,
        message: "hi",
        creating: false,
        scheduleError: null,
      }),
    ).toBe(false);
    expect(
      canCreateAutomationDraft({
        ownerChatId: "c",
        message: "hi",
        creating: true,
        scheduleError: null,
      }),
    ).toBe(false);
  });
});

describe("canCreateTaskDraft", () => {
  it("requires non-empty prompt when not creating", () => {
    expect(canCreateTaskDraft({ prompt: "x", creating: false })).toBe(true);
    expect(canCreateTaskDraft({ prompt: "  ", creating: false })).toBe(false);
  });
});

describe("resolveRunModelIdFromCandidates", () => {
  it("returns requested when auto off", () => {
    expect(
      resolveRunModelIdFromCandidates({
        requestedModelId: "a",
        useAuto: false,
        listResolvable: () => [{ id: "b" }],
        pick: (ms) => ms[0] ?? null,
      }),
    ).toBe("a");
  });
  it("picks auto candidate", () => {
    expect(
      resolveRunModelIdFromCandidates({
        requestedModelId: "a",
        useAuto: true,
        listResolvable: () => [{ id: "b" }, { id: "c" }],
        pick: (ms) => ms[1] ?? null,
      }),
    ).toBe("c");
  });
  it("falls back when pick null", () => {
    expect(
      resolveRunModelIdFromCandidates({
        requestedModelId: "a",
        useAuto: true,
        listResolvable: () => [],
        pick: () => null,
      }),
    ).toBe("a");
  });
});
