import { describe, expect, it } from "vitest";
import {
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
});
