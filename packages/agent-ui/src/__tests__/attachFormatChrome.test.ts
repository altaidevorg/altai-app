import { describe, expect, it } from "vitest";
import {
  formatGitDiffSummary,
  formatTerminalAttachText,
} from "../lib/attachFormatChrome.js";

describe("formatGitDiffSummary", () => {
  it("returns null for empty files", () => {
    expect(formatGitDiffSummary({ files: [] })).toBeNull();
  });

  it("formats branch and status lines", () => {
    const text = formatGitDiffSummary({
      branch: "main",
      files: [
        { path: "a.ts", status: "M" },
        { path: "b.ts", status: "A" },
      ],
    });
    expect(text).toContain("Working tree changes on main");
    expect(text).toContain("- M  a.ts");
    expect(text).toContain("- A  b.ts");
  });
});

describe("formatTerminalAttachText", () => {
  it("prefers selection, then command, then cwd", () => {
    expect(
      formatTerminalAttachText({
        selectedText: "  sel  ",
        lastCommand: "npm test",
        cwd: "/ws",
      }),
    ).toBe("sel");
    expect(
      formatTerminalAttachText({ lastCommand: "npm test", cwd: "/ws" }),
    ).toBe("npm test");
    expect(formatTerminalAttachText({ cwd: "/ws" })).toBe(
      "Active terminal cwd: /ws",
    );
    expect(formatTerminalAttachText({})).toBeNull();
  });
});
