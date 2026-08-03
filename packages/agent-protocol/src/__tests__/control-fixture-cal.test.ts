import { describe, expect, it } from "vitest";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseWorkItemId, serializeWorkItemId, WorkItemIdError } from "../control-fixture-cal.js";

const fixturesDirectory = fileURLToPath(
  new URL("../../../../shared/control-protocol/v1/fixtures/", import.meta.url),
);

const CANONICAL_JSON = '{"type":"work_item_id","value":"wi_01923abc-def0-7abc-8def-0123456789ab"}';

describe("shared control-protocol v1 work-item-id fixture", () => {
  it("round-trips the golden fixture byte-identically", async () => {
    const fixture = JSON.parse(await readFile(join(fixturesDirectory, "work-item-id.json"), "utf8")) as unknown;
    const parsed = parseWorkItemId(fixture);
    const serialized = serializeWorkItemId(parsed);
    expect(serialized).toBe(CANONICAL_JSON);
    expect(parseWorkItemId(JSON.parse(serialized))).toEqual(parsed);
    expect(serializeWorkItemId(parseWorkItemId(JSON.parse(serialized)))).toBe(serialized);
  });

  it("rejects an empty value with a typed error", () => {
    expect(() => parseWorkItemId({ type: "work_item_id", value: "" })).toThrow(WorkItemIdError);
    expect(() => parseWorkItemId({ type: "work_item_id", value: "" })).toThrowError(
      expect.objectContaining({ kind: "empty_value" }),
    );
  });

  it("rejects a missing prefix with a typed error", () => {
    expect(() => parseWorkItemId({ type: "work_item_id", value: "no-prefix" })).toThrow(WorkItemIdError);
    expect(() => parseWorkItemId({ type: "work_item_id", value: "no-prefix" })).toThrowError(
      expect.objectContaining({ kind: "missing_prefix" }),
    );
  });

  it("rejects the wrong type discriminator with a typed error", () => {
    expect(() =>
      parseWorkItemId({ type: "goal_id", value: "wi_01923abc-def0-7abc-8def-0123456789ab" }),
    ).toThrowError(expect.objectContaining({ kind: "wrong_type" }));
  });
});
