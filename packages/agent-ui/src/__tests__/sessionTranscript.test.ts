import { describe, expect, it } from "vitest";
import {
  formatSessionMessageLine,
  transcriptLinesFromMessages,
} from "../lib/sessionTranscript.js";

describe("formatSessionMessageLine", () => {
  it("labels user and assistant turns", () => {
    expect(
      formatSessionMessageLine({ role: "user", content: "  Hello  world  " }),
    ).toBe("You: Hello world");
    expect(
      formatSessionMessageLine({
        role: "assistant",
        content: "Done.",
      }),
    ).toBe("ALTAI: Done.");
  });

  it("clips long content", () => {
    const line = formatSessionMessageLine(
      { role: "user", content: "x".repeat(50) },
      10,
    );
    expect(line).toBe(`You: ${"x".repeat(10)}…`);
  });
});

describe("transcriptLinesFromMessages", () => {
  it("keeps user/assistant and drops tool/system", () => {
    const lines = transcriptLinesFromMessages([
      { role: "system", content: "sys" },
      { role: "user", content: "hi" },
      { role: "tool", content: "tool" },
      { role: "assistant", content: "hello" },
    ]);
    expect(lines).toEqual(["You: hi", "ALTAI: hello"]);
  });

  it("retains the newest messages when over limit", () => {
    const messages = Array.from({ length: 5 }, (_, index) => ({
      role: "user" as const,
      content: `m${index}`,
    }));
    expect(transcriptLinesFromMessages(messages, { maxLines: 2 })).toEqual([
      "You: m3",
      "You: m4",
    ]);
  });
});
