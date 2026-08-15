import { describe, it, expect } from "vitest";
import {
  OrganizationId,
  GoalId,
  ProjectId,
  WorkspaceId,
  AgentInstanceId,
  AgentProfileId,
  AgentProfileRevisionId,
  WorkItemId,
  AttemptId,
  RunId,
  SessionId,
  RoutineId,
  RoutineRevisionId,
  RoutineRunId,
  ApprovalId,
  ExternalObjectId,
  PluginId,
  ALL_ID_KINDS,
  IdError,
  serializeId,
  type TypedId,
} from "../ids.js";

describe("typed IDs", () => {
  it("creates with prefix prepended", () => {
    const id = WorkItemId.create("abc123");
    expect(id.type).toBe("work_item_id");
    expect(id.value).toBe("wi_abc123");
  });

  it("preserves existing prefix", () => {
    const id = WorkItemId.create("wi_abc123");
    expect(id.value).toBe("wi_abc123");
  });

  it("round-trips through JSON", () => {
    const id = WorkItemId.create("01923abc-def0-7abc-8def-0123456789ab");
    const json = serializeId(id);
    expect(json).toBe(
      '{"type":"work_item_id","value":"wi_01923abc-def0-7abc-8def-0123456789ab"}',
    );
    const parsed = WorkItemId.parse(JSON.parse(json));
    expect(parsed).toEqual(id);
  });

  it("all 17 kinds are distinct", () => {
    const kinds = new Set(ALL_ID_KINDS);
    expect(kinds.size).toBe(17);
  });

  it("each kind has a distinct prefix", () => {
    const ids: TypedId[] = [
      OrganizationId.create("x"),
      GoalId.create("x"),
      ProjectId.create("x"),
      WorkspaceId.create("x"),
      AgentInstanceId.create("x"),
      AgentProfileId.create("x"),
      AgentProfileRevisionId.create("x"),
      WorkItemId.create("x"),
      AttemptId.create("x"),
      RunId.create("x"),
      SessionId.create("x"),
      RoutineId.create("x"),
      RoutineRevisionId.create("x"),
      RoutineRunId.create("x"),
      ApprovalId.create("x"),
      ExternalObjectId.create("x"),
      PluginId.create("x"),
    ];
    expect(ids).toHaveLength(17);
    const values = new Set(ids.map((id) => id.value));
    expect(values.size).toBe(17);
    const types = new Set(ids.map((id) => id.type));
    expect(types.size).toBe(17);
  });

  it("rejects wrong type", () => {
    const json = { type: "organization_id", value: "org_test" };
    expect(() => WorkItemId.parse(json)).toThrow(IdError);
    try {
      WorkItemId.parse(json);
      throw new Error("should have thrown");
    } catch (e) {
      expect(e).toBeInstanceOf(IdError);
      expect((e as IdError).kind).toBe("wrong_type");
    }
  });

  it("rejects empty value", () => {
    expect(() => WorkItemId.parse({ type: "work_item_id", value: "" })).toThrow();
    try {
      WorkItemId.parse({ type: "work_item_id", value: "" });
    } catch (e) {
      expect((e as IdError).kind).toBe("empty_value");
    }
  });

  it("rejects missing prefix", () => {
    try {
      WorkItemId.parse({ type: "work_item_id", value: "no_prefix" });
    } catch (e) {
      expect((e as IdError).kind).toBe("missing_prefix");
    }
  });

  it("rejects non-object", () => {
    expect(() => WorkItemId.parse("not an object")).toThrow();
    try {
      WorkItemId.parse(42);
    } catch (e) {
      expect((e as IdError).kind).toBe("invalid_shape");
    }
  });

  it("rejects missing type", () => {
    try {
      WorkItemId.parse({ value: "wi_test" });
    } catch (e) {
      expect((e as IdError).kind).toBe("missing_type");
    }
  });

  it("is() type guard works", () => {
    const id = WorkItemId.create("test");
    expect(WorkItemId.is(id)).toBe(true);
    expect(WorkItemId.is("not an id")).toBe(false);
    expect(WorkItemId.is({ type: "organization_id", value: "org_x" })).toBe(false);
  });

  it("organization id round-trips", () => {
    const id = OrganizationId.create("test-org");
    const json = serializeId(id);
    expect(json).toBe('{"type":"organization_id","value":"org_test-org"}');
    expect(OrganizationId.parse(JSON.parse(json))).toEqual(id);
  });

  it("TypedId type is readonly", () => {
    const id: TypedId = WorkItemId.create("x");
    expect(id.type).toBe("work_item_id");
    expect(id.value).toBe("wi_x");
  });
});
