import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

vi.mock("../lib/native", () => ({
  native: {
    agentSteer: vi.fn(),
    agentCancel: vi.fn(),
    agentApprove: vi.fn(),
    checkpointList: vi.fn(async () => []),
    checkpointRestore: vi.fn(),
    workspaceCurrentDir: vi.fn(async () => "/tmp/ws"),
    writeFile: vi.fn(async () => undefined),
    createDir: vi.fn(async () => undefined),
    delete: vi.fn(async () => undefined),
    agentListSkills: vi.fn(async () => []),
    agentInstallSkill: vi.fn(async () => []),
  },
}));

import { native } from "../lib/native";
import { createTauriHostPorts } from "./createTauriHostPorts";

describe("createTauriHostPorts", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("initializes desktop capabilities", async () => {
    const ports = createTauriHostPorts({ hostVersion: "0.0.0-test" });
    const capabilities = await ports.runtime.initialize({
      protocolMin: 1,
      protocolMax: 1,
      clientName: "test",
      clientVersion: "0.0.0",
    });
    expect(capabilities.hostName).toBe("altai-desktop");
    expect(capabilities.hostVersion).toBe("0.0.0-test");
    expect(capabilities.protocolVersion).toBe(1);
    expect(
      capabilities.capabilities.some(
        (entry) =>
          entry.id === "runtime.startRun" && entry.availability === "available",
      ),
    ).toBe(true);
    expect(
      capabilities.capabilities.some(
        (entry) =>
          entry.id === "review.editProposal" &&
          entry.availability === "available",
      ),
    ).toBe(true);
  });

  it("throws for deferred startRun until store DI lands", async () => {
    const ports = createTauriHostPorts();
    await expect(
      ports.runtime.startRun({ prompt: "hi" }),
    ).rejects.toThrow(/startRun/);
  });

  it("maps getWorkspace through native", async () => {
    const ports = createTauriHostPorts();
    await expect(ports.workspace.getWorkspace()).resolves.toEqual({
      roots: ["/tmp/ws"],
      trusted: true,
      currentDir: "/tmp/ws",
    });
  });

  it("applies edit proposals through planEditFs / native write", async () => {
    const ports = createTauriHostPorts();
    await ports.review.applyEditProposal("p1", {
      path: "src/a.ts",
      kind: "edit_file",
      proposedContent: "hello",
      originalContent: "",
    });
    expect(native.writeFile).toHaveBeenCalledWith(
      "src/a.ts",
      "hello",
      expect.objectContaining({ source: "ai-plan-review" }),
    );
    await expect(ports.review.denyEditProposal("p1")).resolves.toBeUndefined();
  });

  it("lists and installs skills through native skill APIs", async () => {
    vi.mocked(native.agentListSkills).mockResolvedValue([
      { name: "demo", description: "Demo skill" },
    ]);
    vi.mocked(native.agentInstallSkill).mockResolvedValue(["demo"]);
    const ports = createTauriHostPorts();
    const caps = await ports.runtime.initialize({
      protocolMin: 1,
      protocolMax: 1,
      clientName: "test",
      clientVersion: "0.0.0",
    });
    expect(
      caps.capabilities.some(
        (entry) =>
          entry.id === "skills.install" && entry.availability === "available",
      ),
    ).toBe(true);
    await expect(ports.mcpSkills.listSkills()).resolves.toEqual([
      { name: "demo", description: "Demo skill", enabled: true },
    ]);
    await expect(
      ports.mcpSkills.installSkill("owner/repo#demo"),
    ).resolves.toEqual({
      name: "demo",
      description: "Demo skill",
      enabled: true,
    });
    expect(native.agentInstallSkill).toHaveBeenCalledWith(
      "owner/repo",
      "/tmp/ws",
      "demo",
    );
  });
});
