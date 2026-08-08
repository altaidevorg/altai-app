import { describe, expect, it } from "vitest";
import {
  AUTO_MODEL_ID,
  canMountModelPicker,
  filterModels,
  modelIdForStartRun,
  modelTriggerLabel,
  resolveSelectedModelId,
} from "../lib/modelPickerChrome.js";

describe("canMountModelPicker", () => {
  it("requires list, select, and settings get", () => {
    expect(
      canMountModelPicker({ list: true, select: true, settingsGet: true }),
    ).toBe(true);
    expect(
      canMountModelPicker({ list: true, select: false, settingsGet: true }),
    ).toBe(false);
  });
});

describe("resolveSelectedModelId", () => {
  it("defaults to auto when unset", () => {
    expect(resolveSelectedModelId(undefined)).toBe(AUTO_MODEL_ID);
    expect(resolveSelectedModelId("  ")).toBe(AUTO_MODEL_ID);
    expect(resolveSelectedModelId("openai/gpt")).toBe("openai/gpt");
  });
});

describe("filterModels", () => {
  const models = [
    { id: "a", label: "Alpha", providerId: "openai" },
    { id: "b", label: "Beta", providerId: "anthropic" },
  ];

  it("filters by label, id, or provider", () => {
    expect(filterModels(models, "alpha").map((m) => m.id)).toEqual(["a"]);
    expect(filterModels(models, "anthropic").map((m) => m.id)).toEqual(["b"]);
    expect(filterModels(models, "").map((m) => m.id)).toEqual(["a", "b"]);
  });
});

describe("modelTriggerLabel", () => {
  it("prefers model labels and Auto for the sentinel", () => {
    expect(
      modelTriggerLabel("a", [
        { id: "a", label: "Alpha", providerId: "openai" },
      ]),
    ).toBe("Alpha");
    expect(modelTriggerLabel(AUTO_MODEL_ID, [])).toBe("Auto");
  });
});

describe("modelIdForStartRun", () => {
  it("omits auto and empty", () => {
    expect(modelIdForStartRun(null)).toBeUndefined();
    expect(modelIdForStartRun(AUTO_MODEL_ID)).toBeUndefined();
    expect(modelIdForStartRun("  ")).toBeUndefined();
    expect(modelIdForStartRun("openai/gpt")).toBe("openai/gpt");
  });
});
