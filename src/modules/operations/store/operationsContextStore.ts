import { create } from "zustand";
import type { OperationsHealth } from "../lib/operationsHealth";

/**
 * The operations shell's explicit context (package 061). One store holds the
 * connection state machine and the workspace it describes, so every surface
 * answers "is the control plane serving this workspace, and which workspace
 * am I in?" from one place instead of re-deriving it.
 *
 * Connection states:
 * - `offline` — no workspace is open; there is nothing to serve.
 * - `connecting` — a workspace is open but no probe has answered yet.
 * - `healthy` — negotiation answered compatible through the shared dispatcher.
 * - `degraded` — the control plane cannot be confirmed for this workspace
 *   (command error, version mismatch, or missing capabilities).
 */

export type OperationsConnection =
  | "offline"
  | "connecting"
  | "degraded"
  | "healthy";

/**
 * The deployment's organization/project context. The embedded desktop host
 * serves no org/project projections over the protocol yet (it wires only
 * `activity_audit` and `event_replay`), so the explicit scope today is the
 * local workspace: one local organization, the open workspace as the
 * project. When the protocol grows org/project queries, this becomes
 * protocol-sourced instead of workspace-derived.
 */
export type OperationsScope = {
  kind: "workspace-local";
  organization: "local";
  project: string;
};

export type OperationsContext = {
  connection: OperationsConnection;
  workspacePath: string | null;
  workspaceName: string | null;
  /** The org/project context above; null exactly when no workspace is open. */
  scope: OperationsScope | null;
  /** Deployment mode from the last healthy negotiation, e.g. `embedded_host`. */
  deploymentMode: string | null;
  /** Protocol version from the last healthy negotiation, e.g. `1.0`. */
  protocolVersion: string | null;
  /** Why the connection is degraded, when it is. */
  detail: string | null;
  checkedAtMs: number | null;
};

type State = OperationsContext & {
  /** Announce the open workspace; null closes it and enters offline. */
  setWorkspace: (
    workspacePath: string | null,
    workspaceName: string | null,
  ) => void;
  /** Fold one probe result in. Results from a workspace that is no longer
   *  open are dropped, so a slow probe can never speak for another workspace. */
  applyHealth: (
    workspacePath: string,
    health: OperationsHealth,
    checkedAtMs: number,
  ) => void;
};

const OFFLINE: OperationsContext = {
  connection: "offline",
  workspacePath: null,
  workspaceName: null,
  scope: null,
  deploymentMode: null,
  protocolVersion: null,
  detail: null,
  checkedAtMs: null,
};

function connecting(path: string, name: string | null): OperationsContext {
  return {
    ...OFFLINE,
    connection: "connecting",
    workspacePath: path,
    workspaceName: name,
    scope: { kind: "workspace-local", organization: "local", project: name ?? path },
  };
}

export const useOperationsContextStore = create<State>((set) => ({
  ...OFFLINE,
  setWorkspace: (workspacePath, workspaceName) =>
    set((state) => {
      if (state.workspacePath === workspacePath) {
        // Same workspace: keep the classification, just refresh the label.
        if (state.workspaceName === workspaceName) return state;
        return { ...state, workspaceName };
      }
      // Opening or switching: the previous workspace's negotiation is void.
      return workspacePath === null ? OFFLINE : connecting(workspacePath, workspaceName);
    }),
  applyHealth: (workspacePath, health, checkedAtMs) =>
    set((state) => {
      if (state.workspacePath !== workspacePath) return state;
      if (health.connection === "healthy") {
        const { major, minor } = health.negotiation.server_version;
        return {
          ...state,
          connection: "healthy",
          deploymentMode: health.negotiation.deployment_mode,
          protocolVersion: `${major}.${minor}`,
          detail: null,
          checkedAtMs,
        };
      }
      if (health.connection === "degraded") {
        return {
          ...state,
          connection: "degraded",
          deploymentMode: null,
          protocolVersion: null,
          detail: health.detail,
          checkedAtMs,
        };
      }
      // An offline classification never reaches a non-null workspace path
      // (the probe only classifies null as offline), so anything else is a
      // no-op rather than marking an open workspace offline.
      return state;
    }),
}));
