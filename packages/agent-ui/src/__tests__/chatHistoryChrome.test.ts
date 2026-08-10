import { describe, expect, it } from "vitest";
import {
  CHAT_HISTORY_CONTROL_LABEL,
  CHAT_HISTORY_SEARCH_PLACEHOLDER,
  SESSION_UNTITLED_TITLE,
  chatHistoryEmptyMessage,
} from "../lib/chatHistoryChrome.js";

describe("chatHistoryChrome", () => {
  it("exposes control copy and empty messages", () => {
    expect(CHAT_HISTORY_CONTROL_LABEL).toBe("Chat history");
    expect(CHAT_HISTORY_SEARCH_PLACEHOLDER).toContain("Search");
    expect(SESSION_UNTITLED_TITLE).toBe("New chat");
    expect(chatHistoryEmptyMessage(true)).toBe("No chats match.");
    expect(chatHistoryEmptyMessage(false)).toBe("No chats yet.");
  });
});
