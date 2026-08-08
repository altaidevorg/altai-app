import { describe, expect, it } from "vitest";
import {
  chatDisplayBubbleClassName,
  chatDisplayBubbleModifier,
  chatDisplayRoleLabel,
} from "../lib/chatDisplayChrome.js";

describe("chatDisplayChrome", () => {
  it("labels and modifies roles", () => {
    expect(chatDisplayRoleLabel("user")).toBe("You");
    expect(chatDisplayRoleLabel("assistant")).toBe("ALTAI");
    expect(chatDisplayRoleLabel("meta")).toBe("");
    expect(chatDisplayBubbleModifier("assistant")).toBe("assistant");
    expect(chatDisplayBubbleModifier("unknown")).toBe("meta");
    expect(chatDisplayBubbleClassName("user")).toBe(
      "altai-chat-bubble altai-chat-bubble--user",
    );
  });
});
