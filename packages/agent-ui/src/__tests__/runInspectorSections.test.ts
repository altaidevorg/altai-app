import { describe, expect, it } from "vitest";
import {
  activityFromMessages,
  buildRunInspectorSections,
  changesFromMessages,
  hasRunInspectorContent,
  mapApprovalsToInspectorItems,
} from "../lib/runInspectorSections.js";

describe("runInspectorSections", () => {
  it("maps approvals, changes, activity", () => {
    expect(
      mapApprovalsToInspectorItems([
        { approvalId: "a1", toolName: "edit", input: { x: 1 } },
      ]),
    ).toEqual([{ id: "a1", action: "edit", payload: { x: 1 } }]);

    const messages = [
      {
        id: "t1",
        role: "tool",
        content: "patched",
        toolName: "apply_patch",
        filePath: "a.ts",
        diffOriginalText: "old",
        diffModifiedText: "new",
      },
    ];
    expect(changesFromMessages(messages)).toEqual([
      {
        id: "t1",
        path: "a.ts",
        originalContent: "old",
        proposedContent: "new",
        isNewFile: false,
      },
    ]);
    expect(activityFromMessages(messages)[0]?.label).toBe("apply_patch");

    const model = buildRunInspectorSections({
      approvals: [{ approvalId: "a1", toolName: "edit" }],
      messages,
    });
    expect(hasRunInspectorContent(model)).toBe(true);
  });
});
