import { beforeEach, describe, expect, it } from "vitest";
import { useOperationsContextStore } from "./operationsContextStore";
import type { OperationsHealth } from "../lib/operationsHealth";

function reset() {
  useOperationsContextStore.setState({
    connection: "offline",
    workspacePath: null,
    workspaceName: null,
    deploymentMode: null,
    protocolVersion: null,
    detail: null,
    checkedAtMs: null,
  });
}

const healthy: OperationsHealth = {
  connection: "healthy",
  negotiation: {
    server_version: { major: 1, minor: 0 },
    deployment_mode: "embedded_host",
    compatible: true,
    missing_capabilities: [],
  },
};

beforeEach(reset);

describe("operations context store", () => {
  it("starts offline with no workspace and no protocol context", () => {
    const state = useOperationsContextStore.getState();
    expect(state.connection).toBe("offline");
    expect(state.workspacePath).toBeNull();
    expect(state.deploymentMode).toBeNull();
    expect(state.protocolVersion).toBeNull();
    expect(state.checkedAtMs).toBeNull();
  });

  it("opening a workspace enters connecting and clears stale protocol context", () => {
    useOperationsContextStore.getState().setWorkspace("/workspace-a", "alpha");
    useOperationsContextStore.getState().applyHealth("/workspace-a", healthy, 111);
    expect(useOperationsContextStore.getState().connection).toBe("healthy");

    useOperationsContextStore.getState().setWorkspace("/workspace-b", "beta");
    const state = useOperationsContextStore.getState();
    expect(state.connection).toBe("connecting");
    expect(state.workspacePath).toBe("/workspace-b");
    expect(state.workspaceName).toBe("beta");
    // A previous workspace's negotiation must never leak into the new one.
    expect(state.deploymentMode).toBeNull();
    expect(state.protocolVersion).toBeNull();
    expect(state.checkedAtMs).toBeNull();
  });

  it("closing the workspace enters offline and clears protocol context", () => {
    useOperationsContextStore.getState().setWorkspace("/workspace-a", "alpha");
    useOperationsContextStore.getState().applyHealth("/workspace-a", healthy, 111);

    useOperationsContextStore.getState().setWorkspace(null, null);
    const state = useOperationsContextStore.getState();
    expect(state.connection).toBe("offline");
    expect(state.workspacePath).toBeNull();
    expect(state.workspaceName).toBeNull();
    expect(state.deploymentMode).toBeNull();
    expect(state.protocolVersion).toBeNull();
    expect(state.checkedAtMs).toBeNull();
  });

  it("ignores a probe result from a workspace that is no longer open", () => {
    useOperationsContextStore.getState().setWorkspace("/workspace-b", "beta");
    useOperationsContextStore.getState().applyHealth("/workspace-a", healthy, 111);

    const state = useOperationsContextStore.getState();
    expect(state.connection).toBe("connecting");
    expect(state.checkedAtMs).toBeNull();
  });

  it("records a healthy negotiation with deployment mode and protocol version", () => {
    useOperationsContextStore.getState().setWorkspace("/workspace-a", "alpha");
    useOperationsContextStore.getState().applyHealth("/workspace-a", healthy, 1234);

    const state = useOperationsContextStore.getState();
    expect(state.connection).toBe("healthy");
    expect(state.deploymentMode).toBe("embedded_host");
    expect(state.protocolVersion).toBe("1.0");
    expect(state.checkedAtMs).toBe(1234);
    expect(state.detail).toBeNull();
  });

  it("records a degraded probe with its reason and no protocol version", () => {
    useOperationsContextStore.getState().setWorkspace("/workspace-a", "alpha");
    useOperationsContextStore.getState().applyHealth(
      "/workspace-a",
      {
        connection: "degraded",
        negotiation: null,
        detail: "work.db schema is newer than this build",
      },
      1234,
    );

    const state = useOperationsContextStore.getState();
    expect(state.connection).toBe("degraded");
    expect(state.detail).toBe("work.db schema is newer than this build");
    expect(state.deploymentMode).toBeNull();
    expect(state.protocolVersion).toBeNull();
  });
});
