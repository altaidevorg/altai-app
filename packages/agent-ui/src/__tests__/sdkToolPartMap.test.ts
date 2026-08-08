import { describe, expect, it } from "vitest";
import {
  isSdkToolPart,
  mapSdkToolApprovalPart,
  mapSdkToolCardPart,
  sdkToolName,
} from "../lib/sdkToolPartMap.js";

describe("sdkToolPartMap", () => {
  it("resolves names from static and dynamic tools", () => {
    expect(sdkToolName({ type: "tool-read_file" })).toBe("read_file");
    expect(
      sdkToolName({ type: "dynamic-tool", toolName: "exec" }),
    ).toBe("exec");
    expect(isSdkToolPart({ type: "text" })).toBe(false);
    expect(isSdkToolPart({ type: "tool-x" })).toBe(true);
  });

  it("maps approval and card views", () => {
    expect(
      mapSdkToolApprovalPart({
        type: "tool-exec",
        state: "output-available",
        approval: { id: "a1" },
      }),
    ).toBeNull();

    expect(
      mapSdkToolApprovalPart({
        type: "tool-exec",
        state: "approval-requested",
        approval: { id: "a1" },
        input: { cmd: "ls" },
      }),
    ).toEqual({
      toolName: "exec",
      approvalId: "a1",
      input: { cmd: "ls" },
    });

    const card = mapSdkToolCardPart({
      type: "tool-list_directory",
      state: "output-available",
      input: { path: "." },
      output: { entries: [] },
    });
    expect(card.defaultOpen).toBe(true);
    expect(card.toolName).toBe("list_directory");
  });
});
