import { describe, expect, it } from "vitest";
import {
  canCopyDisplayMessage,
  hasDisplayMessageActions,
  lastAssistantMessageId,
  resolveDisplayMessageActions,
} from "../lib/chatDisplayActions.js";

describe("chatDisplayActions", () => {
  it("copies finished user/assistant text only", () => {
    expect(canCopyDisplayMessage({ role: "user", content: "hi" })).toBe(true);
    expect(
      canCopyDisplayMessage({
        role: "assistant",
        content: "…",
        streaming: true,
      }),
    ).toBe(false);
    expect(canCopyDisplayMessage({ role: "tool", content: "x" })).toBe(false);
  });

  it("finds the trailing assistant id", () => {
    expect(
      lastAssistantMessageId([
        { id: "u", role: "user" },
        { id: "a1", role: "assistant" },
        { id: "t", role: "tool" },
        { id: "a2", role: "assistant" },
      ]),
    ).toBe("a2");
    expect(lastAssistantMessageId([{ id: "u", role: "user" }])).toBeNull();
  });

  it("resolves action flags from capabilities", () => {
    const flags = resolveDisplayMessageActions({
      message: {
        id: "a1",
        role: "assistant",
        content: "hello",
      },
      lastAssistantId: "a1",
      canEditUserMessages: true,
      canRetry: true,
      canOpenFile: true,
      canOpenDiff: true,
      hasEditHandler: true,
      hasRetryHandler: true,
    });
    expect(flags).toEqual({
      showEdit: false,
      showRetry: true,
      showCopy: true,
      showOpenFile: false,
      showOpenDiff: false,
    });
    expect(hasDisplayMessageActions(flags)).toBe(true);

    const userFlags = resolveDisplayMessageActions({
      message: { id: "u1", role: "user", content: "q" },
      lastAssistantId: "a1",
      canEditUserMessages: true,
      canRetry: true,
      canOpenFile: false,
      canOpenDiff: false,
      hasEditHandler: true,
      hasRetryHandler: true,
    });
    expect(userFlags.showEdit).toBe(true);
    expect(userFlags.showRetry).toBe(false);
  });
});
