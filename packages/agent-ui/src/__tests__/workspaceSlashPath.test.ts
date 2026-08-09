import { describe, expect, it } from "vitest";
import {
  isValidSlashCommandName,
  isWorkspaceSlashCommandPath,
  workspaceSlashCommandStem,
} from "../lib/workspaceSlashPath.js";

describe("workspace slash path", () => {
  it("accepts .altai/commands paths", () => {
    expect(isWorkspaceSlashCommandPath(".altai/commands/release-notes.md")).toBe(true);
    expect(isWorkspaceSlashCommandPath("src/ignored.md")).toBe(false);
    expect(workspaceSlashCommandStem(".altai/commands/Init.md")).toBe("init");
    expect(workspaceSlashCommandStem("nope")).toBeNull();
  });

  it("validates command names", () => {
    expect(isValidSlashCommandName("release-notes")).toBe(true);
    expect(isValidSlashCommandName("-bad")).toBe(false);
    expect(isValidSlashCommandName("has spaces")).toBe(false);
  });
});
