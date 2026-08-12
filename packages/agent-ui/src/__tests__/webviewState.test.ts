import { describe, expect, it } from "vitest";
import {
  activeChatFocusPatch,
  activeChatIdForRoot,
  mergePersistedWebviewState,
  normalizeComposerDraft,
  parsePersistedWebviewState,
} from "../lib/webviewState.js";

describe("webviewState", () => {
  it("parses valid surface and draft", () => {
    const s = parsePersistedWebviewState({
      surface: "chat",
      composerDraft: "hello",
    });
    expect(s.surface).toBe("chat");
    expect(s.composerDraft).toBe("hello");
  });
  it("drops invalid surface", () => {
    expect(parsePersistedWebviewState({ surface: "x" }).surface).toBeUndefined();
  });
  it("normalizes empty draft to undefined", () => {
    expect(normalizeComposerDraft("")).toBeUndefined();
  });
  it("merges patches without inventing privileged fields", () => {
    const next = mergePersistedWebviewState(
      { surface: "chat" },
      { operationsView: "work", composerDraft: "" },
    );
    expect(next.surface).toBe("chat");
    expect(next.operationsView).toBe("work");
    expect(next.composerDraft).toBeUndefined();
  });

  it("keeps active chat focus per workspace root", () => {
    const parsed = parsePersistedWebviewState({
      preferredRootUri: "file:///a",
      activeChatId: "chat-a",
    });
    expect(parsed.activeChatIdByRoot).toEqual({ "file:///a": "chat-a" });

    const merged = mergePersistedWebviewState(
      parsed,
      activeChatFocusPatch("file:///b", "chat-b"),
    );
    expect(activeChatIdForRoot(merged, "file:///a")).toBe("chat-a");
    expect(activeChatIdForRoot(merged, "file:///b")).toBe("chat-b");
    expect(merged.activeChatId).toBe("chat-b");
  });
});
