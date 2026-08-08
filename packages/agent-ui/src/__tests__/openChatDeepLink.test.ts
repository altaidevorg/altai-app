import { describe, expect, it } from "vitest";
import {
  buildOpenChatFocus,
  chatFocusStatusLine,
} from "../lib/openChatDeepLink.js";

describe("buildOpenChatFocus", () => {
  it("trims chatId and label and always sets a key", () => {
    const focus = buildOpenChatFocus(
      { chatId: "  chat-1  ", label: "  Broken run  " },
      42,
    );
    expect(focus).toEqual({
      key: 42,
      chatId: "chat-1",
      label: "Broken run",
    });
  });

  it("omits empty fields", () => {
    expect(buildOpenChatFocus({ chatId: " ", label: "" }, 1)).toEqual({
      key: 1,
    });
  });
});

describe("chatFocusStatusLine", () => {
  it("prefers label + chatId copy", () => {
    expect(
      chatFocusStatusLine({
        key: 1,
        chatId: "c1",
        label: "Nightly",
      }),
    ).toBe("Opened from Operations · Nightly · chat c1");
  });

  it("falls back when only the surface switches", () => {
    expect(chatFocusStatusLine({ key: 1 })).toBe(
      "Opened Chat from Operations",
    );
  });
});
