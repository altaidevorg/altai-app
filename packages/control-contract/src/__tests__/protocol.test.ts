import { describe, it, expect } from "vitest";
import {
  CURRENT_PROTOCOL_VERSION,
  isProtocolCompatible,
  defaultCapabilities,
  supportsCapability,
  evaluateCapabilityNegotiation,
  createPageRequest,
  type ProtocolRequest,
  type ProtocolResponse,
  type PageResponse,
} from "../protocol.js";
import { ControlErrorCode } from "../error.js";

describe("control protocol contracts", () => {
  it("verifies protocol version compatibility", () => {
    expect(
      isProtocolCompatible(CURRENT_PROTOCOL_VERSION, { major: 1, minor: 1 }),
    ).toBe(true);
    expect(
      isProtocolCompatible(CURRENT_PROTOCOL_VERSION, { major: 2, minor: 0 }),
    ).toBe(false);
  });

  it("checks default capabilities support", () => {
    const caps = defaultCapabilities();
    expect(supportsCapability(caps, "organizations")).toBe(true);
    expect(supportsCapability(caps, "work_graph")).toBe(true);
    expect(supportsCapability(caps, "attempts")).toBe(true);
    expect(supportsCapability(caps, "routines")).toBe(true);
    expect(supportsCapability(caps, "non_existent_cap")).toBe(false);
  });

  it("evaluates capability negotiation with all capabilities present", () => {
    const caps = defaultCapabilities();
    const result = evaluateCapabilityNegotiation(
      CURRENT_PROTOCOL_VERSION,
      "local_daemon",
      caps,
      {
        client_version: CURRENT_PROTOCOL_VERSION,
        client_name: "altai-desktop",
        required_capabilities: ["organizations", "projects", "work_graph"],
      },
    );

    expect(result.compatible).toBe(true);
    expect(result.missing_capabilities).toHaveLength(0);
    expect(result.deployment_mode).toBe("local_daemon");
  });

  it("evaluates capability negotiation with missing capabilities", () => {
    const caps = {
      ...defaultCapabilities(),
      budgets: false,
      event_replay: false,
    };
    const result = evaluateCapabilityNegotiation(
      CURRENT_PROTOCOL_VERSION,
      "local_daemon",
      caps,
      {
        client_version: CURRENT_PROTOCOL_VERSION,
        client_name: "altai-cli",
        required_capabilities: ["organizations", "budgets", "event_replay"],
      },
    );

    expect(result.compatible).toBe(false);
    expect(result.missing_capabilities).toEqual(["budgets", "event_replay"]);
  });

  it("clamps page request limit bounds", () => {
    const reqZero = createPageRequest(null, 0);
    expect(reqZero.limit).toBe(1);

    const reqHuge = createPageRequest(null, 500);
    expect(reqHuge.limit).toBe(250);

    const reqNormal = createPageRequest("cur_abc", 25);
    expect(reqNormal.limit).toBe(25);
    expect(reqNormal.cursor).toBe("cur_abc");
  });

  it("shapes protocol responses accurately", () => {
    const successRes: ProtocolResponse<{ status: string }> = {
      id: "req_01",
      result: {
        Ok: { status: "ready" },
      },
    };
    expect("Ok" in successRes.result).toBe(true);

    const errorRes: ProtocolResponse<never> = {
      id: "req_02",
      result: {
        Err: {
          code: ControlErrorCode.PolicyDenied,
          message: "Operation not permitted",
        },
      },
    };
    expect("Err" in errorRes.result).toBe(true);
    if ("Err" in errorRes.result) {
      expect(errorRes.result.Err.code).toBe(ControlErrorCode.PolicyDenied);
    }
  });

  it("handles paginated response structure", () => {
    const page: PageResponse<string> = {
      items: ["item1", "item2"],
      next_cursor: "cur_next",
      has_more: true,
      total_count: 10,
    };
    expect(page.items).toHaveLength(2);
    expect(page.has_more).toBe(true);
    expect(page.next_cursor).toBe("cur_next");
  });
});
