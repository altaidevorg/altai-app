import { createElement, type ReactNode } from "react";
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import {
  createCapabilities,
  type HostPorts,
} from "@altai/host-contract";
import {
  HostPortsProvider,
  useCapability,
  useHostPorts,
  withUnsupportedDefaults,
} from "../index.js";

function createMinimalPorts(): HostPorts {
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
        async initialize() {
          return createCapabilities({
            protocolVersion: 1,
            hostName: "test",
            hostVersion: "0.0.0",
            overrides: {
              "runtime.startRun": "available",
              "sessions.list": "deferred",
            },
          });
        },
        async shutdown() {},
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
      {},
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
      {},
    ),
    review: withUnsupportedDefaults(
      "review",
      [
        "listCheckpoints",
        "restoreCheckpoint",
        "applyEditProposal",
        "denyEditProposal",
      ],
      {},
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
        "listWorkInbox",
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
      subscribe() {
        return () => {};
      },
    },
  };
}

function Probe({ children }: { children?: ReactNode }) {
  const ports = useHostPorts();
  const startRun = useCapability("runtime.startRun");
  const sessionsList = useCapability("sessions.list");
  return createElement(
    "div",
    {
      "data-has-runtime": String(Boolean(ports.runtime)),
      "data-start-run": String(startRun),
      "data-sessions-list": String(sessionsList),
    },
    children,
  );
}

describe("@altai/agent-ui HostPortsProvider", () => {
  it("injects ports and gates capabilities fail-closed until loaded", () => {
    const ports = createMinimalPorts();
    const html = renderToStaticMarkup(
      createElement(HostPortsProvider, {
        ports,
        children: createElement(Probe),
      }),
    );
    expect(html).toContain('data-has-runtime="true"');
    expect(html).toContain('data-start-run="false"');
    expect(html).toContain('data-sessions-list="false"');
  });

  it("enables available capabilities after initialize result is provided", async () => {
    const ports = createMinimalPorts();
    const capabilities = await ports.runtime.initialize({
      protocolMin: 1,
      protocolMax: 1,
      clientName: "test",
      clientVersion: "0.0.0",
    });
    const html = renderToStaticMarkup(
      createElement(HostPortsProvider, {
        ports,
        capabilities,
        children: createElement(Probe),
      }),
    );
    expect(html).toContain('data-start-run="true"');
    expect(html).toContain('data-sessions-list="false"');
  });
});
