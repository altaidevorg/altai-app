import { describe, expect, it } from "vitest";
import {
  isPlanPermissionMode,
  latestTodoItemsFromDisplayMessages,
  permissionModeAfterExitPlan,
} from "../lib/chatPlanChrome.js";

describe("isPlanPermissionMode", () => {
  it("detects plan vs other modes", () => {
    expect(isPlanPermissionMode("plan")).toBe(true);
    expect(isPlanPermissionMode("auto-edit")).toBe(false);
    expect(isPlanPermissionMode(null)).toBe(false);
  });
});

describe("latestTodoItemsFromDisplayMessages", () => {
  it("returns the newest non-empty todos list", () => {
    const messages = [
      {
        id: "t1",
        role: "tool" as const,
        todos: [{ title: "old", status: "completed" as const }],
      },
      { id: "a", role: "assistant" as const },
      {
        id: "t2",
        role: "tool" as const,
        todos: [
          { title: "new a", status: "completed" as const },
          { title: "new b", status: "pending" as const },
        ],
      },
    ];
    expect(latestTodoItemsFromDisplayMessages(messages)).toEqual([
      { title: "new a", status: "completed" },
      { title: "new b", status: "pending" },
    ]);
  });
});

describe("permissionModeAfterExitPlan", () => {
  it("exits into auto-edit", () => {
    expect(permissionModeAfterExitPlan()).toBe("auto-edit");
  });
});
