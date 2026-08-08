import { describe, expect, it } from "vitest";
import {
  filterModelsByProviderKeys,
  mergeModelCatalog,
  providerIdForModel,
} from "../lib/modelCatalogChrome.js";
import { AUTO_MODEL_ID } from "../lib/modelPickerChrome.js";

const SAMPLE_CATALOG = [
  {
    id: "claude-sonnet-4-6",
    label: "Claude Sonnet 4.6",
    providerId: "anthropic",
  },
  {
    id: "gpt-5.5",
    label: "GPT-5.5",
    providerId: "openai",
  },
] as const;

describe("modelCatalogChrome", () => {
  it("includes Auto and catalog entries when host is empty", () => {
    const catalog = mergeModelCatalog([], SAMPLE_CATALOG);
    expect(catalog.some((m) => m.id === AUTO_MODEL_ID)).toBe(true);
    expect(catalog.some((m) => m.id === "claude-sonnet-4-6")).toBe(true);
    expect(catalog.some((m) => m.providerId === "openai")).toBe(true);
    expect(catalog[0]?.id).toBe(AUTO_MODEL_ID);
  });

  it("lets host entries override labels", () => {
    const catalog = mergeModelCatalog(
      [
        {
          id: "claude-sonnet-4-6",
          label: "Custom Sonnet",
          providerId: "anthropic",
        },
      ],
      SAMPLE_CATALOG,
    );
    expect(catalog.find((m) => m.id === "claude-sonnet-4-6")?.label).toBe(
      "Custom Sonnet",
    );
  });

  it("filters still keep Auto and catalog when keys empty", () => {
    const models = mergeModelCatalog([], SAMPLE_CATALOG);
    const filtered = filterModelsByProviderKeys(models, new Set());
    expect(filtered.length).toBe(models.length);
  });

  it("resolves provider id for a model", () => {
    const models = mergeModelCatalog([], SAMPLE_CATALOG);
    expect(providerIdForModel("gpt-5.5", models)).toBe("openai");
    expect(providerIdForModel(AUTO_MODEL_ID, models)).toBeUndefined();
  });
});
