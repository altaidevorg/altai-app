import { describe, expect, it } from "vitest";
import type { AgentRecord } from "@altai/host-contract";
import {
  projectAgentsAdmin,
  toAgentAdminError,
  toAgentAdminRow,
  toManagerOptions,
} from "./agentsAdminProjection";

function agent(overrides: Partial<AgentRecord> = {}): AgentRecord {
  return {
    id: "agent_1",
    name: "Atlas",
    status: "active",
    reportsTo: null,
    createdAtMs: 1_000,
    updatedAtMs: 2_000,
    ...overrides,
  };
}

describe("toAgentAdminRow", () => {
  it("resolves the reporting line to a manager name", () => {
    const lead = agent({ id: "agent_lead", name: "Atlas" });
    const mid = agent({
      id: "agent_mid",
      name: "Brix",
      reportsTo: "agent_lead",
    });
    const { byId } = projectAgentsAdmin([lead, mid]);
    const row = toAgentAdminRow(mid, byId);
    expect(row.reportsToId).toBe("agent_lead");
    expect(row.reportsToName).toBe("Atlas");
  });

  it("leaves a dangling reporting line nameless, not crashy", () => {
    const row = toAgentAdminRow(
      agent({ reportsTo: "agent_gone" }),
      new Map(),
    );
    expect(row.reportsToId).toBe("agent_gone");
    expect(row.reportsToName).toBeNull();
  });

  it("derives legal actions from the lifecycle rules", () => {
    const rules: Array<[AgentRecord["status"], boolean, boolean, boolean]> = [
      ["active", true, false, true],
      ["paused", false, true, true],
      ["terminated", false, false, false],
    ];
    for (const [status, canPause, canResume, canTerminate] of rules) {
      const row = toAgentAdminRow(agent({ status }), new Map());
      expect(row.statusLabel, status).toBe(
        status.charAt(0).toUpperCase() + status.slice(1),
      );
      expect(row.canPause, status).toBe(canPause);
      expect(row.canResume, status).toBe(canResume);
      expect(row.canTerminate, status).toBe(canTerminate);
    }
  });
});

describe("projectAgentsAdmin", () => {
  it("orders rows by name and builds the id map", () => {
    const zed = agent({ id: "agent_z", name: "Zed" });
    const atlas = agent({ id: "agent_a", name: "Atlas" });
    const { rows, byId } = projectAgentsAdmin([zed, atlas]);
    expect(rows.map((row) => row.name)).toEqual(["Atlas", "Zed"]);
    expect(byId.get("agent_z")).toBe(zed);
  });
});

describe("toManagerOptions", () => {
  it("offers every agent except the one moving", () => {
    const lead = agent({ id: "agent_lead", name: "Atlas" });
    const mid = agent({ id: "agent_mid", name: "Brix" });
    const junior = agent({ id: "agent_j", name: "Cade" });
    const options = toManagerOptions([lead, mid, junior], "agent_mid");
    expect(options).toEqual([
      { id: "agent_lead", name: "Atlas" },
      { id: "agent_j", name: "Cade" },
    ]);
  });

  it("keeps everyone available for the create form", () => {
    const lead = agent();
    expect(toManagerOptions([lead], null)).toEqual([
      { id: "agent_1", name: "Atlas" },
    ]);
  });
});

describe("toAgentAdminError", () => {
  it("translates the store's work-shaped vocabulary", () => {
    expect(
      toAgentAdminError(
        "invalid work transition: reporting line would form an org-chart cycle",
      ),
    ).toBe("Reporting line would form an org-chart cycle.");
    expect(
      toAgentAdminError("invalid work transition: a terminated agent is final"),
    ).toBe("A terminated agent is final.");
    expect(toAgentAdminError("work item not found: agent_x")).toBe(
      "Agent not found.",
    );
  });

  it("capitalizes and closes unknown messages", () => {
    expect(toAgentAdminError("workspace is not registered")).toBe(
      "Workspace is not registered.",
    );
  });
});
