import { beforeEach, describe, expect, it, vi } from "vitest";
import { HostPortUnsupportedError } from "@altai/agent-ui";
import type { WorkAttempt } from "@altai/host-contract";

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
    workList: vi.fn(async () => []),
    workChildren: vi.fn(async () => []),
    workGet: vi.fn(async () => null),
    workCreate: vi.fn(),
    workTransition: vi.fn(),
    workStart: vi.fn(),
    workAttempts: vi.fn(async () => []),
    workReadyForReview: vi.fn(),
    workReview: vi.fn(),
    workInboxList: vi.fn(async () => []),
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
          entry.id === "work.items" && entry.availability === "available",
      ),
    ).toBe(true);
    expect(
      capabilities.capabilities.some(
        (entry) =>
          entry.id === "work.inbox" && entry.availability === "available",
      ),
    ).toBe(true);
    expect(
      capabilities.capabilities.some(
        (entry) =>
          entry.id === "work.attempts" &&
          entry.availability === "available",
      ),
    ).toBe(true);
    expect(
      capabilities.capabilities.some(
        (entry) =>
          entry.id === "work.attemptRuns" &&
          entry.availability === "available",
      ),
    ).toBe(false);
    expect(
      capabilities.capabilities.some(
        (entry) =>
          entry.id === "review.editProposal" &&
          entry.availability === "available",
      ),
    ).toBe(true);
  });

  it("maps canonical Work methods through the durable native host", async () => {
    const item = {
      id: "work-1",
      projectId: "project",
      title: "Ship Work OS",
      description: "",
      acceptanceCriteria: "All surfaces share an id",
      kind: "task" as const,
      state: "ready" as const,
      revision: 2,
      createdAtMs: 1,
      updatedAtMs: 2,
    };
    vi.mocked(native.workList).mockResolvedValue([item]);
    vi.mocked(native.workChildren).mockResolvedValue([item]);
    vi.mocked(native.workGet).mockResolvedValue(item);
    vi.mocked(native.workReview).mockResolvedValue({
      ...item,
      state: "done",
      revision: 3,
    });

    const ports = createTauriHostPorts();
    await expect(ports.work.listWork("my_active")).resolves.toEqual([item]);
    await expect(ports.work.listWorkChildren(item.id)).resolves.toEqual([item]);
    await expect(ports.work.getWork(item.id)).resolves.toEqual(item);
    await expect(
      ports.work.reviewWork({
        workId: item.id,
        expectedRevision: item.revision,
        accept: true,
      }),
    ).resolves.toMatchObject({ state: "done", revision: 3 });
    expect(native.workList).toHaveBeenCalledWith("my_active");
    expect(native.workChildren).toHaveBeenCalledWith(item.id);
    expect(native.workReview).toHaveBeenCalledWith({
      workId: item.id,
      expectedRevision: item.revision,
      accept: true,
    });
    await expect(ports.inbox.listWorkInbox()).resolves.toEqual([]);
    expect(native.workInboxList).toHaveBeenCalledOnce();
  });

  it("lists Work attempts through the durable native host", async () => {
    const attempt = {
      id: "attempt-1",
      workId: "work-1",
      number: 1,
      role: "executor",
      phase: "succeeded" as const,
      chatId: "chat-1",
      runId: "run-1",
      createdAtMs: 1,
      updatedAtMs: 2,
    } satisfies WorkAttempt;
    vi.mocked(native.workAttempts).mockResolvedValue([attempt]);

    const ports = createTauriHostPorts();
    await expect(ports.work.listWorkAttempts("work-1")).resolves.toEqual([
      attempt,
    ]);
    expect(native.workAttempts).toHaveBeenCalledWith("work-1");
  });

  it("keeps startWorkRun unsupported because Desktop owns session orchestration", async () => {
    const ports = createTauriHostPorts();
    await expect(
      ports.work.startWorkRun({ workId: "work-1", expectedRevision: 1 }),
    ).rejects.toBeInstanceOf(HostPortUnsupportedError);
  });

  it("throws for deferred startRun until store DI lands", async () => {
    const ports = createTauriHostPorts();
    await expect(
      ports.runtime.startRun({ prompt: "hi" }),
    ).rejects.toBeInstanceOf(HostPortUnsupportedError);
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
