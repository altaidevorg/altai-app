/**
 * Desktop Tauri adapter for `@altai/host-contract` HostPorts.
 *
 * TASK-007 / A4 first slice: inject a real HostPorts aggregate into shared UI.
 * Wave 1.3: ReviewPort apply/deny mutates via shared planEditFs (not UI-direct writes).
 */

import { listenAppEvent, type UnlistenFn } from "@/lib/appEvent";
import {
  createCapabilities,
  type AgentEvent,
  type Capabilities,
  type EditProposalInput,
  type HostPorts,
  type InitializeInput,
  type PermissionMode,
  type SkillInfo,
} from "@altai/host-contract";
import { withUnsupportedDefaults } from "@altai/agent-ui";
import { native } from "../lib/native";
import {
  applyPlanEditMutation,
  type PlanEditFs,
} from "../lib/planEditFs";
import { parseSkillInstallSource } from "../lib/skillInstallSource";

const HOST_NAME = "altai-desktop";

const tauriPlanFs: PlanEditFs = {
  writeFile: (path, content, opts) =>
    native.writeFile(path, content, {
      source: opts?.source ?? "ai-plan-review",
    }),
  createDir: (path) => native.createDir(path),
  delete: (path) => native.delete(path),
};

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
            overrides: {
              "review.checkpoints": "available",
              "review.restoreCheckpoint": "available",
              "review.editProposal": "available",
              "workspace.info": "available",
              "skills.list": "available",
              "skills.install": "available",
            },
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
        async applyEditProposal(
          proposalId: string,
          input?: EditProposalInput,
        ) {
          const id = proposalId.trim();
          if (!id) {
            throw new Error("invalid_proposal_id");
          }
          if (!input?.path?.trim()) {
            throw new Error("edit_proposal_requires_input");
          }
          const kind = input.kind ?? "edit_file";
          await applyPlanEditMutation(
            tauriPlanFs,
            {
              kind,
              path: input.path,
              proposedContent: input.proposedContent ?? "",
              originalContent: input.originalContent ?? "",
              isNewFile: kind === "create_file",
            },
            "ai-plan-review",
          );
        },
        async denyEditProposal(proposalId: string) {
          const id = proposalId.trim();
          if (!id) {
            throw new Error("invalid_proposal_id");
          }
          // Desktop queue is UI-owned; deny is a host ACK so callers can
          // drop local rows after a successful response.
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
      {
        async listSkills(): Promise<SkillInfo[]> {
          let workspace: string | undefined;
          try {
            workspace = await native.workspaceCurrentDir();
          } catch {
            workspace = undefined;
          }
          const rows = await native.agentListSkills(workspace);
          return rows.map((row) => ({
            name: row.name,
            description: row.description,
            enabled: true,
          }));
        },
        async installSkill(source: string): Promise<SkillInfo> {
          const { repo, skill } = parseSkillInstallSource(source);
          if (!repo) {
            throw new Error("A repository URL or owner/repo is required.");
          }
          let workspace: string | undefined;
          try {
            workspace = await native.workspaceCurrentDir();
          } catch {
            workspace = undefined;
          }
          const names = await native.agentInstallSkill(repo, workspace, skill);
          if (names.length === 0) {
            throw new Error("No skills found in that repository.");
          }
          const installed = await native.agentListSkills(workspace);
          const first = names[0]!;
          const match = installed.find((row) => row.name === first);
          return {
            name: first,
            description: match?.description ?? null,
            enabled: true,
          };
        },
      },
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
