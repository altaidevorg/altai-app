import { describe, expect, it, vi } from "vitest";
import {
  OPERATIONS_CLIENT_NAME,
  OPERATIONS_PROTOCOL_CLIENT_VERSION,
  operationsNegotiateParams,
  probeOperationsHealth,
  type OperationsNegotiation,
} from "./operationsHealth";

function negotiation(
  overrides: Partial<OperationsNegotiation> = {},
): OperationsNegotiation {
  return {
    server_version: { major: 1, minor: 0 },
    deployment_mode: "embedded_host",
    compatible: true,
    missing_capabilities: [],
    ...overrides,
  };
}

describe("operations health probe", () => {
  it("classifies a compatible negotiation as healthy", async () => {
    const health = await probeOperationsHealth({
      workspacePath: "/workspace-a",
      negotiate: async () => negotiation(),
    });
    expect(health).toEqual({
      connection: "healthy",
      negotiation: negotiation(),
    });
  });

  it("negotiates with the desktop client identity and no required capabilities", async () => {
    const negotiate = vi.fn(async () => negotiation());
    await probeOperationsHealth({
      workspacePath: "/workspace-a",
      negotiate,
    });
    expect(negotiate).toHaveBeenCalledWith(
      "/workspace-a",
      operationsNegotiateParams(),
    );
    expect(operationsNegotiateParams()).toEqual({
      client_version: OPERATIONS_PROTOCOL_CLIENT_VERSION,
      client_name: OPERATIONS_CLIENT_NAME,
      required_capabilities: [],
    });
  });

  it("classifies an incompatible negotiation as degraded with a reason", async () => {
    const health = await probeOperationsHealth({
      workspacePath: "/workspace-a",
      negotiate: async () =>
        negotiation({
          compatible: false,
          missing_capabilities: ["activity_audit"],
        }),
    });
    expect(health.connection).toBe("degraded");
    expect(health.connection === "degraded" && health.detail).toContain(
      "activity_audit",
    );
  });

  it("reports a version mismatch when no capabilities are missing", async () => {
    const health = await probeOperationsHealth({
      workspacePath: "/workspace-a",
      negotiate: async () =>
        negotiation({
          compatible: false,
          server_version: { major: 2, minor: 0 },
        }),
    });
    expect(health.connection === "degraded" && health.detail).toContain(
      "protocol version",
    );
  });

  it("classifies a command error as degraded and keeps the reason", async () => {
    const health = await probeOperationsHealth({
      workspacePath: "/workspace-a",
      negotiate: async () => {
        throw new Error("work.db schema is newer than this build");
      },
    });
    expect(health).toMatchObject({
      connection: "degraded",
      negotiation: null,
    });
    expect(health.connection === "degraded" && health.detail).toContain(
      "newer than this build",
    );
  });

  it("classifies a missing workspace as offline without negotiating", async () => {
    const negotiate = vi.fn(async () => negotiation());
    const health = await probeOperationsHealth({
      workspacePath: null,
      negotiate,
    });
    expect(health).toEqual({ connection: "offline", detail: null });
    expect(negotiate).not.toHaveBeenCalled();
  });
});
