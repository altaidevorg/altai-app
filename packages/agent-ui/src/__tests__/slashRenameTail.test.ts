import { describe, expect, it } from "vitest";
import { hasSlashCommandTail } from "../lib/slashRenameTail.js";

describe("hasSlashCommandTail", () => {
  it("requires non-empty trimmed tail", () => {
    expect(hasSlashCommandTail("title")).toBe(true);
    expect(hasSlashCommandTail("  ")).toBe(false);
    expect(hasSlashCommandTail("")).toBe(false);
  });
});
