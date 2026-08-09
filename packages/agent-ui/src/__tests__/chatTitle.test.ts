import { describe, expect, it } from "vitest";
import {
  deriveChatTitleFromMessages,
  stripChatTitleNoise,
} from "../lib/chatTitle.js";

describe("deriveChatTitleFromMessages", () => {
  it("strips noise and uses first user line", () => {
    expect(stripChatTitleNoise("<env>x</env>\nHello world")).toBe("Hello world");
    expect(
      deriveChatTitleFromMessages([
        {
          role: "user",
          parts: [
            {
              type: "text",
              text: "<system-reminder>s</system-reminder>\n# Title line\nmore",
            },
          ],
        },
      ]),
    ).toBe("Title line");
  });

  it("returns empty title when no user text", () => {
    expect(deriveChatTitleFromMessages([])).toBe("New chat");
  });
});
