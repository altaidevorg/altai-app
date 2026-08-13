import { describe, expect, it } from "vitest";
import {
  LegacyMappingError,
  mapLegacyAssignment,
} from "../legacy-mapping.js";

function legacyAssignment(status: unknown): Record<string, unknown> {
  return {
    id: "legacy-task-42",
    title: "Fix the thing",
    status,
    createdAt: "2026-08-03T12:00:00Z",
  };
}

describe("mapLegacyAssignment (CAL-03)", () => {
  it("maps all known statuses per the amended table", () => {
    const cases: Array<[string, string, string]> = [
      ["queued", "todo", "queued"],
      ["running", "in_progress", "running"],
      ["succeeded", "done", "terminal"],
      ["failed", "in_progress", "failed"],
      ["cancelled", "cancelled", "terminal"],
    ];
    for (const [status, expectedWorkStatus, expectedExecutionPhase] of cases) {
      const mapped = mapLegacyAssignment(legacyAssignment(status));
      expect(mapped.work_status).toBe(expectedWorkStatus);
      expect(mapped.execution_phase).toBe(expectedExecutionPhase);
    }
  });

  it("maps a sample legacy assignment to the canonical WorkItem shape", () => {
    const mapped = mapLegacyAssignment(legacyAssignment("queued"));
    expect(mapped).toEqual({
      work_item_id: null, // pure mapping never invents durable IDs
      title: "Fix the thing",
      work_status: "todo",
      execution_phase: "queued",
      legacy_compat_id: "legacy-task-42",
      created_at: "2026-08-03T12:00:00Z",
    });
  });

  it("preserves the legacy ID in legacy_compat_id", () => {
    const mapped = mapLegacyAssignment(legacyAssignment("running"));
    expect(mapped.legacy_compat_id).toBe("legacy-task-42");
  });

  it("is pure: mapping the same input twice produces the same output", () => {
    const input = legacyAssignment("failed");
    const first = mapLegacyAssignment(input);
    const second = mapLegacyAssignment(input);
    expect(first).toEqual(second);
  });

  it('rejects unknown status "foobar" with a typed error', () => {
    expect(() => mapLegacyAssignment(legacyAssignment("foobar"))).toThrow(LegacyMappingError);
    expect(() => mapLegacyAssignment(legacyAssignment("foobar"))).toThrowError(
      expect.objectContaining({ kind: "unknown_legacy_status", field: "status", value: "foobar" }),
    );
  });

  it("rejects a missing title with a typed error", () => {
    const input = legacyAssignment("queued");
    delete input.title;
    expect(() => mapLegacyAssignment(input)).toThrowError(
      expect.objectContaining({ kind: "missing_required_field", field: "title" }),
    );
  });

  it("rejects a missing id with a typed error", () => {
    const input = legacyAssignment("queued");
    delete input.id;
    expect(() => mapLegacyAssignment(input)).toThrowError(
      expect.objectContaining({ kind: "missing_required_field", field: "id" }),
    );
  });

  it("rejects an empty id with a typed error", () => {
    const input = legacyAssignment("queued");
    input.id = "";
    expect(() => mapLegacyAssignment(input)).toThrowError(
      expect.objectContaining({ kind: "invalid_legacy_id", field: "id" }),
    );
  });

  it("rejects a null status with a typed error", () => {
    expect(() => mapLegacyAssignment(legacyAssignment(null))).toThrowError(
      expect.objectContaining({ kind: "missing_required_field", field: "status" }),
    );
  });
});
