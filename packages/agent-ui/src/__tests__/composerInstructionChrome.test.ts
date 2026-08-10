import { describe, expect, it } from "vitest";
import {
  prependComposerInstruction,
  SEMBLE_SCOUT_SEARCH_INSTRUCTION,
} from "../lib/composerInstructionChrome.js";

describe("prependComposerInstruction", () => {
  it("keeps empty draft as prefix + blanks", () => {
    expect(prependComposerInstruction("  ", "Lead")).toBe("Lead\n\n");
  });

  it("prepends before existing body", () => {
    expect(prependComposerInstruction("hello", SEMBLE_SCOUT_SEARCH_INSTRUCTION)).toBe(
      `${SEMBLE_SCOUT_SEARCH_INSTRUCTION}\n\nhello`,
    );
  });
});
