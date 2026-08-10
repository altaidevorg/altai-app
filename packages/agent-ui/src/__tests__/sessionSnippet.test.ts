import { describe, expect, it } from "vitest";
import {
  cleanTranscriptSnippetText,
  extractSessionSnippet,
  hasConversationContent,
} from "../lib/sessionSnippet.js";

describe("cleanTranscriptSnippetText", () => {
  it("strips context tags and collapses space", () => {
    expect(
      cleanTranscriptSnippetText(
        "Hello  <env>x</env>  world\n\nnext",
      ),
    ).toBe("Hello world next");
  });
});

describe("extractSessionSnippet", () => {
  it("takes latest cleaned text and truncates", () => {
    const long = "a".repeat(100);
    expect(
      extractSessionSnippet([
        { parts: [{ type: "text", text: "old" }] },
        {
          parts: [
            {
              type: "text",
              text: `<file>x</file>${long}`,
            },
          ],
        },
      ]),
    ).toBe(`${"a".repeat(90)}…`);
  });
  it("returns empty when only empty text", () => {
    expect(
      extractSessionSnippet([{ parts: [{ type: "text", text: "  " }] }]),
    ).toBe("");
  });
});

describe("hasConversationContent", () => {
  it("detects text and user non-text parts", () => {
    expect(
      hasConversationContent([{ parts: [{ type: "text", text: "hi" }] }]),
    ).toBe(true);
    expect(
      hasConversationContent([
        { role: "user", parts: [{ type: "file" }] },
      ]),
    ).toBe(true);
    expect(
      hasConversationContent([
        { role: "assistant", parts: [{ type: "tool" }] },
      ]),
    ).toBe(false);
  });
});
