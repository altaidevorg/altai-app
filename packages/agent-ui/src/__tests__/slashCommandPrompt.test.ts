import { describe, expect, it } from "vitest";
import {
  INIT_WORKSPACE_PROMPT,
  promptForSlashCommand,
  SLASH_COMMAND_PROMPTS,
} from "../lib/slashCommandPrompt.js";

describe("promptForSlashCommand", () => {
  it("uses templates + focus", () => {
    expect(SLASH_COMMAND_PROMPTS.init).toBe(INIT_WORKSPACE_PROMPT);
    expect(promptForSlashCommand("index", "")).toContain("codebase map");
    expect(promptForSlashCommand("fix", " login bug")).toContain(
      "Focus from the user: login bug",
    );
    expect(promptForSlashCommand("unknown", "")).toMatch(/carefully/);
  });
});
