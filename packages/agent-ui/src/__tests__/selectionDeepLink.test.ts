import { describe, expect, it } from "vitest";
import {
  buildOpenChatWithSelectionPayload,
  parseOpenChatWithSelectionPayload,
} from "../lib/selectionDeepLink.js";

describe("selectionDeepLink", () => {
  it("parses and counts lines", () => {
    const p = parseOpenChatWithSelectionPayload({
      key: 1,
      uri: "file:///a",
      path: "/a",
      text: "a\nb\n",
    });
    expect(p?.lines).toBe(2);
  });
  it("rejects empty text", () => {
    expect(
      buildOpenChatWithSelectionPayload({
        uri: "u",
        path: "p",
        text: "  ",
      }),
    ).toBeNull();
  });
});
