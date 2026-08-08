import { describe, expect, it } from "vitest";
import {
  chatLineKind,
  shouldShowChatEmptyHome,
} from "../lib/chatLineKind.js";

describe("chatLineKind", () => {
  it("classifies user, agent, and meta lines", () => {
    expect(chatLineKind("You: hello")).toBe("user");
    expect(chatLineKind("Here is a reply")).toBe("agent");
    expect(chatLineKind("Host ready")).toBe("meta");
    expect(chatLineKind("Session abc focused")).toBe("meta");
  });
});

describe("shouldShowChatEmptyHome", () => {
  it("is true only for empty transcripts", () => {
    expect(shouldShowChatEmptyHome([])).toBe(true);
    expect(shouldShowChatEmptyHome(["You: hi"])).toBe(false);
  });
});
