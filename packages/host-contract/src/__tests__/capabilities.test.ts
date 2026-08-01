import { describe, expect, it } from "vitest";
import {
  CHAT_PANEL_ACTIONS,
  DEFAULT_CAPABILITY_MATRIX,
  capabilityForAction,
  createCapabilities,
  isCapabilityEnabled,
} from "../capabilities.js";
import { HOST_CONTRACT_VERSION } from "../types.js";
import {
  parseCapabilities,
  parsePermissionMode,
} from "../validate.js";

describe("capability catalog", () => {
  it("covers every chat-panel action with a known capability", () => {
    const known = new Set(DEFAULT_CAPABILITY_MATRIX.map((entry) => entry.id));
    expect(CHAT_PANEL_ACTIONS.length).toBeGreaterThan(40);

    for (const action of CHAT_PANEL_ACTIONS) {
      expect(known.has(action.capability), action.action).toBe(true);
      expect(capabilityForAction(action.action)).toBe(action.capability);
    }
  });

  it("never marks desktop-only IDE surfaces as available by default", () => {
    const caps = createCapabilities({
      protocolVersion: 1,
      hostName: "test",
      hostVersion: "0.0.0",
    });

    expect(isCapabilityEnabled(caps, "desktop.gitPanelMutations")).toBe(false);
    expect(isCapabilityEnabled(caps, "desktop.orchestration")).toBe(false);
    expect(isCapabilityEnabled(caps, "desktop.studioWindow")).toBe(false);
    expect(isCapabilityEnabled(caps, "runtime.initialize")).toBe(true);
  });

  it("applies availability overrides", () => {
    const caps = createCapabilities({
      protocolVersion: 1,
      hostName: "vscode",
      hostVersion: "0.1.0",
      overrides: {
        "runtime.startRun": "available",
        "sessions.list": "available",
      },
    });

    expect(isCapabilityEnabled(caps, "runtime.startRun")).toBe(true);
    expect(isCapabilityEnabled(caps, "sessions.list")).toBe(true);
    expect(isCapabilityEnabled(caps, "runtime.steerRun")).toBe(false);
  });
});

describe("parseCapabilities", () => {
  it("accepts a valid document", () => {
    const parsed = parseCapabilities({
      contractVersion: HOST_CONTRACT_VERSION,
      protocolVersion: 1,
      hostName: "altai-desktop",
      hostVersion: "0.6.5",
      capabilities: [
        { id: "runtime.initialize", availability: "available" },
        {
          id: "runtime.startRun",
          availability: "deferred",
          note: "pending",
        },
      ],
    });

    expect(parsed.ok).toBe(true);
    if (parsed.ok) {
      expect(parsed.value.capabilities).toHaveLength(2);
      expect(parsed.value.capabilities[1]?.note).toBe("pending");
    }
  });

  it("rejects wrong contract version and bad availability", () => {
    expect(
      parseCapabilities({
        contractVersion: 999,
        protocolVersion: 1,
        hostName: "x",
        hostVersion: "1",
        capabilities: [],
      }).ok,
    ).toBe(false);

    expect(
      parseCapabilities({
        contractVersion: HOST_CONTRACT_VERSION,
        protocolVersion: 1,
        hostName: "x",
        hostVersion: "1",
        capabilities: [{ id: "runtime.initialize", availability: "maybe" }],
      }).ok,
    ).toBe(false);
  });
});

describe("parsePermissionMode", () => {
  it("accepts canonical modes only", () => {
    expect(parsePermissionMode("ask")).toEqual({ ok: true, value: "ask" });
    expect(parsePermissionMode("auto-edit")).toEqual({
      ok: true,
      value: "auto-edit",
    });
    expect(parsePermissionMode("plan").ok).toBe(true);
    expect(parsePermissionMode("bypass").ok).toBe(true);
    expect(parsePermissionMode("yolo").ok).toBe(false);
  });
});
