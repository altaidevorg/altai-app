import { describe, expect, it } from "vitest";
import {
  formatTranscriptForCopy,
  roleLabelForCopy,
} from "../lib/transcriptCopyChrome.js";

describe("roleLabelForCopy", () => {
  it("maps known roles", () => {
    expect(roleLabelForCopy("user")).toBe("You");
    expect(roleLabelForCopy("assistant")).toBe("ALTAI");
    expect(roleLabelForCopy("tool")).toBe("Tool");
  });
});

describe("formatTranscriptForCopy", () => {
  it("joins non-empty rows", () => {
    expect(
      formatTranscriptForCopy([
        { role: "user", content: "hi" },
        { role: "assistant", content: "  " },
        { role: "assistant", content: "hello" },
      ]),
    ).toBe("You:\nhi\n\nALTAI:\nhello");
  });
});
