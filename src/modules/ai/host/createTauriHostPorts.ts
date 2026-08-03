/**
 * Desktop Tauri adapter for `@altai/host-contract` HostPorts.
 *
 * TASK-007 / A4 first slice: inject a real HostPorts aggregate into shared UI.
 * Chat stores still call `native.ts` directly until store DI lands; methods that
 * are not yet mapped throw `HostPortUnsupportedError`.
 */

import { listenAppEvent, type UnlistenFn } from "@/lib/appEvent";
import {
  createCapabilities,
  type AgentEvent,
  type Capabilities,
  type HostPorts,
  type InitializeInput,
  type PermissionMode,
} from "@altai/host-contract";
import { withUnsupportedDefaults } from "@altai/agent-ui";
import { native } from "../lib/native";

const HOST_NAME = "altai-desktop";

export type TauriHostPortsOptions = {
  hostVersion?: string;
};

/**
 * Build the Desktop HostPorts aggregate used by `@altai/agent-ui`.
 */
export function createTauriHostPorts(
  options: TauriHostPortsOptions = {},
): HostPorts {
  const hostVersion = options.hostVersion ?? "0.6.6";

  return {
    runtime: withUnsupportedDefaults(
      "runtime",
      [
        "initialize",
        "startRun",
        "steerRun",
        "cancelRun",
        "retryRun",
        "respondToApproval",
        "respondToClarification",
        "compactContext",
        "replayRun",
        "shutdown",
      ],
      {
        async initialize(_input: InitializeInput): Promise<Capabilities> {
          return createCapabilities({
            protocolVersion: 1,
            hostName: HOST_NAME,
            hostVersion,
          });
        },
        async steerRun(input) {
          await native.agentSteer(input.chatId, input.runId, input.prompt);
        },
        async cancelRun(input) {
          await native.agentCancel(input.chatId, input.runId);
        },
        async respondToApproval(input) {
          await native.agentApprove(
            input.approvalId,
            input.decision === "approve",
          );
        },
        async shutdown() {
          // Desktop keeps the long-lived AgentService for the app lifetime.
        },
      },
    ),
    sessions: withUnsupportedDefaults(
      "sessions",
      [
        "listSessions",
        "getSession",
        "createSession",
        "renameSession",
        "archiveSession",
        "deleteSession",
        "truncateSession",
        "listMessages",
      ],
      {},
    ),
    workspace: withUnsupportedDefaults(
      "workspace",
      [
        "getWorkspace",
        "getActiveFile",
        "getSelection",
        "searchFiles",
        "readFile",
        "openFile",
        "openDiff",
        "getGitDiff",
        "getTerminalContext",
      ],
      {
        async getWorkspace() {
          const root = await native.workspaceCurrentDir();
          return {
            roots: [root],
            trusted: true,
            currentDir: root,
          };
        },
      },
    ),
    settings: withUnsupportedDefaults(
      "settings",
      [
        "getSettings",
        "updateSettings",
        "getProviderStatus",
        "beginProviderConnection",
        "clearProviderCredential",
        "listModels",
        "setPermissionMode",
      ],
      {
        async setPermissionMode(mode: PermissionMode): Promise<PermissionMode> {
          return mode;
        },
      },
    ),
    review: withUnsupportedDefaults(
      "review",
      [
        "listCheckpoints",
        "restoreCheckpoint",
        "applyEditProposal",
        "denyEditProposal",
      ],
      {
        async listCheckpoints(chatId: string) {
          const rows = await native.checkpointList();
          return rows.map((row) => ({
            id: row.id,
            chatId,
            createdAt: new Date(row.createdMs).toISOString(),
            label: row.label || row.path,
          }));
        },
        async restoreCheckpoint(checkpointId: string) {
          await native.checkpointRestore(checkpointId);
        },
      },
    ),
    work: withUnsupportedDefaults(
      "work",
      [
        "listTaskRuns",
        "createTaskRun",
        "cancelTaskRun",
        "retryTaskRun",
        "removeTaskRun",
        "listAutomations",
        "createAutomation",
        "updateAutomation",
        "triggerAutomation",
        "pauseAutomation",
        "deleteAutomation",
      ],
      {},
    ),
    inbox: withUnsupportedDefaults(
      "inbox",
      [
        "listNotifications",
        "markNotificationSeen",
        "resolveNotification",
        "dismissNotification",
      ],
      {},
    ),
    mcpSkills: withUnsupportedDefaults(
      "mcpSkills",
      [
        "listMcpServers",
        "configureMcpServer",
        "setMcpServerEnabled",
        "restartMcpServer",
        "listSkills",
        "installSkill",
        "setSkillEnabled",
      ],
      {},
    ),
    events: {
      subscribe(listener: (event: AgentEvent) => void): () => void {
        let active = true;
        let unlisten: UnlistenFn | undefined;
        void listenAppEvent<AgentEvent>("agent://event", (event) => {
          if (active) {
            listener(event.payload);
          }
        }).then((fn) => {
          unlisten = fn;
          if (!active) {
            fn();
          }
        });
        return () => {
          active = false;
          unlisten?.();
        };
      },
    },
  };
}
