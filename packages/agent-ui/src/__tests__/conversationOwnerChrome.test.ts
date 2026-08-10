import { describe, expect, it } from "vitest";
import { nextConversationOwnerChatId } from "../lib/conversationOwnerChrome.js";

describe("nextConversationOwnerChatId", () => {
  it("adopts active when owner empty", () => {
    expect(nextConversationOwnerChatId("a", "", ["a", "b"])).toBe("a");
  });
  it("adopts active when owner not in sessions", () => {
    expect(nextConversationOwnerChatId("a", "gone", ["a"])).toBe("a");
  });
  it("returns null when owner still valid", () => {
    expect(nextConversationOwnerChatId("a", "b", ["a", "b"])).toBeNull();
  });
  it("returns null without active chat", () => {
    expect(nextConversationOwnerChatId(null, "", ["a"])).toBeNull();
  });
});
