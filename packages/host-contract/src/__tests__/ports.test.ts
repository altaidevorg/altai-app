import { describe, expect, it } from "vitest";
import type {
  AgentRuntimePort,
  EventPort,
  HostPorts,
  InboxPort,
  McpSkillsPort,
  ReviewPort,
  SessionPort,
  SettingsPort,
  WorkPort,
  WorkspacePort,
} from "../ports.js";
import { createCapabilities } from "../capabilities.js";
import type {
  AgentEvent,
  AltaiSettings,
  InitializeInput,
  RunRef,
} from "../types.js";

/**
 * Compile-time structural checks: if a required port method is removed,
 * this file fails typecheck. Runtime assertions keep the suite honest.
 */

type AssertEqual<A, B> = [A] extends [B] ? ([B] extends [A] ? true : false) : false;

type RequiredRuntimeMethods = keyof AgentRuntimePort;
type RequiredWorkspaceMethods = keyof WorkspacePort;

const _runtimeMethodsOk: AssertEqual<
  RequiredRuntimeMethods,
  | "initialize"
  | "startRun"
  | "steerRun"
  | "cancelRun"
  | "retryRun"
  | "respondToApproval"
  | "respondToClarification"
  | "compactContext"
  | "replayRun"
  | "shutdown"
> = true;

const _workspaceMethodsOk: AssertEqual<
  RequiredWorkspaceMethods,
  | "getWorkspace"
  | "getActiveFile"
  | "getSelection"
  | "searchFiles"
  | "readFile"
  | "openFile"
  | "openDiff"
  | "getGitDiff"
  | "getTerminalContext"
> = true;

void _runtimeMethodsOk;
void _workspaceMethodsOk;

function unimplemented(): never {
  throw new Error("not implemented");
}

function createStubPorts(): HostPorts {
  const runtime: AgentRuntimePort = {
    async initialize(_input: InitializeInput) {
      return createCapabilities({
        protocolVersion: 1,
        hostName: "stub",
        hostVersion: "0.0.0",
      });
    },
    startRun: unimplemented,
    steerRun: unimplemented,
    cancelRun: unimplemented,
    retryRun: unimplemented,
    respondToApproval: unimplemented,
    respondToClarification: unimplemented,
    compactContext: unimplemented,
    replayRun: unimplemented,
    async shutdown() {},
  };

  const sessions: SessionPort = {
    listSessions: unimplemented,
    getSession: unimplemented,
    createSession: unimplemented,
    renameSession: unimplemented,
    archiveSession: unimplemented,
    deleteSession: unimplemented,
    truncateSession: unimplemented,
    listMessages: unimplemented,
  };

  const workspace: WorkspacePort = {
    getWorkspace: unimplemented,
    getActiveFile: unimplemented,
    getSelection: unimplemented,
    searchFiles: unimplemented,
    readFile: unimplemented,
    openFile: unimplemented,
    openDiff: unimplemented,
    getGitDiff: unimplemented,
    getTerminalContext: unimplemented,
  };

  const settings: SettingsPort = {
    async getSettings(): Promise<AltaiSettings> {
      return {
        permissionMode: "ask",
        bypassEnabled: false,
      };
    },
    updateSettings: unimplemented,
    getProviderStatus: unimplemented,
    beginProviderConnection: unimplemented,
    clearProviderCredential: unimplemented,
    listModels: unimplemented,
    setPermissionMode: unimplemented,
  };

  const review: ReviewPort = {
    listCheckpoints: unimplemented,
    restoreCheckpoint: unimplemented,
    applyEditProposal: unimplemented,
    denyEditProposal: unimplemented,
  };

  const work: WorkPort = {
    listTaskRuns: unimplemented,
    createTaskRun: unimplemented,
    cancelTaskRun: unimplemented,
    retryTaskRun: unimplemented,
    removeTaskRun: unimplemented,
    listAutomations: unimplemented,
    createAutomation: unimplemented,
    updateAutomation: unimplemented,
    triggerAutomation: unimplemented,
    pauseAutomation: unimplemented,
    deleteAutomation: unimplemented,
  };

  const inbox: InboxPort = {
    listNotifications: unimplemented,
    markNotificationSeen: unimplemented,
    resolveNotification: unimplemented,
    dismissNotification: unimplemented,
  };

  const mcpSkills: McpSkillsPort = {
    listMcpServers: unimplemented,
    configureMcpServer: unimplemented,
    setMcpServerEnabled: unimplemented,
    restartMcpServer: unimplemented,
    listSkills: unimplemented,
    installSkill: unimplemented,
    setSkillEnabled: unimplemented,
  };

  const events: EventPort = {
    subscribe(_listener: (event: AgentEvent) => void) {
      return () => {};
    },
  };

  return {
    runtime,
    sessions,
    workspace,
    settings,
    review,
    work,
    inbox,
    mcpSkills,
    events,
  };
}

describe("HostPorts shape", () => {
  it("exposes every aggregate port key", () => {
    const ports = createStubPorts();
    expect(Object.keys(ports).sort()).toEqual(
      [
        "events",
        "inbox",
        "mcpSkills",
        "review",
        "runtime",
        "sessions",
        "settings",
        "work",
        "workspace",
      ].sort(),
    );
  });

  it("initialize returns a capabilities document", async () => {
    const ports = createStubPorts();
    const caps = await ports.runtime.initialize({
      protocolMin: 1,
      protocolMax: 1,
      clientName: "test",
      clientVersion: "0.0.0",
    });
    expect(caps.contractVersion).toBe(1);
    expect(caps.capabilities.length).toBeGreaterThan(0);
  });

  it("keeps RunRef identity fields required at the type level", () => {
    const ref: RunRef = { chatId: "c1", runId: "r1" };
    expect(ref.chatId).toBe("c1");
  });
});
