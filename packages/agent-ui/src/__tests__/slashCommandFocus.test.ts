import { describe, expect, it } from "vitest";
import { appendSlashCommandFocus } from "../lib/slashCommandFocus.js";

describe("appendSlashCommandFocus", () => {
  it("leaves base alone when tail empty", () => {
    expect(appendSlashCommandFocus("Do X.", "")).toBe("Do X.");
    expect(appendSlashCommandFocus("Do X.", "   ")).toBe("Do X.");
  });

  it("appends focus paragraph", () => {
    expect(appendSlashCommandFocus("Do X.", "files only")).toBe(
      "Do X.\n\nFocus from the user: files only",
    );
  });
});
