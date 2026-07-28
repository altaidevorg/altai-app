import { beforeEach, describe, expect, it, vi } from "vitest";
import { invoke } from "@tauri-apps/api/core";
import { useChatStore } from "../store/chatStore";
import { native } from "./native";
import {
  findSlashCommands,
  refreshWorkspaceSlashCommands,
  tryRunSlashCommand,
} from "./slashCommands";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async () => ({ chatId: "chat-1" })),
}));

vi.mock("@/modules/workspace/folder", () => ({
  currentWorkspaceFolder: () => "/workspace",
}));

describe("manual compaction slash command", () => {
  beforeEach(() => {
    vi.mocked(invoke).mockClear();
    useChatStore.setState({ activeSessionId: "chat-1" });
    void refreshWorkspaceSlashCommands(null);
  });

  it("reaches exactly one backend command without producing a model prompt", async () => {
    const outcome = tryRunSlashCommand("/compact keep API decisions");

    expect(outcome).toMatchObject({ kind: "handled" });
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith("agent_compact", {
      workspacePath: "/workspace",
      chatId: "chat-1",
      focusInstructions: "keep API decisions",
    });
    await Promise.resolve();
  });

  it("indexes aliases without duplicating commands in the picker", () => {
    expect(findSlashCommands("condense")).toEqual([
      expect.objectContaining({ name: "compact", aliases: expect.arrayContaining(["smol", "condense"]) }),
    ]);
  });

  it("exposes a broad built-in command index for slash discovery", () => {
    const commands = findSlashCommands();
    expect(commands.length).toBeGreaterThanOrEqual(30);
    expect(commands).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ name: "new" }),
        expect.objectContaining({ name: "test" }),
        expect.objectContaining({ name: "security" }),
        expect.objectContaining({ name: "automations" }),
      ]),
    );
    const namesAndAliases = commands.flatMap((command) => [
      command.name,
      ...(command.aliases ?? []),
    ]);
    expect(new Set(namesAndAliases).size).toBe(namesAndAliases.length);
  });

  it("runs a compact alias through the same backend command", async () => {
    const outcome = tryRunSlashCommand("/smol keep API decisions");

    expect(outcome).toMatchObject({ kind: "handled" });
    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith("agent_compact", {
      workspacePath: "/workspace",
      chatId: "chat-1",
      focusInstructions: "keep API decisions",
    });
    await Promise.resolve();
  });

  it("indexes ALTAI workspace workflows without allowing built-in overrides", async () => {
    const list = vi.spyOn(native, "listWorkspaceFiles").mockResolvedValue({
      files: [
        ".altai/commands/release-notes.md",
        ".altai/commands/init.md",
        "src/ignored.md",
      ],
      truncated: false,
    });
    const read = vi.spyOn(native, "readFile").mockImplementation(async (path) => ({
      kind: "text",
      size: 42,
      content: path.endsWith("release-notes.md")
        ? "---\ndescription: Draft release notes\naliases: [release, notes]\n---\n\nRead the changes and draft release notes."
        : "# Should not override init\n",
    }));

    await refreshWorkspaceSlashCommands("/workspace");

    expect(findSlashCommands("release")).toEqual([
      expect.objectContaining({
        name: "release-notes",
        source: "workspace",
        workflowPath: ".altai/commands/release-notes.md",
      }),
    ]);
    expect(findSlashCommands("init")).toEqual(
      expect.arrayContaining([expect.objectContaining({ name: "init", source: "builtin" })]),
    );
    expect(list).toHaveBeenCalledWith("/workspace", expect.objectContaining({ showHidden: true }));
    expect(read).toHaveBeenCalledTimes(2);
  });
});
