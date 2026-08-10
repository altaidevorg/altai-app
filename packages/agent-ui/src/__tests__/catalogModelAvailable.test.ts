import { describe, expect, it } from "vitest";
import {
  filterAvailableCatalogModels,
  isCatalogModelAvailable,
} from "../lib/catalogModelAvailable.js";

const needs = (provider: string) => provider !== "local";

describe("isCatalogModelAvailable", () => {
  it("hides models in the hidden set", () => {
    expect(
      isCatalogModelAvailable({
        modelId: "a",
        provider: "local",
        hiddenIds: ["a"],
        providerNeedsKey: needs,
        hasProviderKey: () => true,
      }),
    ).toBe(false);
  });
  it("requires keys when provider needs them", () => {
    expect(
      isCatalogModelAvailable({
        modelId: "a",
        provider: "openai",
        hiddenIds: [],
        providerNeedsKey: needs,
        hasProviderKey: () => false,
      }),
    ).toBe(false);
    expect(
      isCatalogModelAvailable({
        modelId: "a",
        provider: "openai",
        hiddenIds: [],
        providerNeedsKey: needs,
        hasProviderKey: () => true,
      }),
    ).toBe(true);
  });
});

describe("filterAvailableCatalogModels", () => {
  it("filters by key + hidden", () => {
    const models = [
      { id: "hidden", provider: "openai" },
      { id: "ok", provider: "openai" },
      { id: "local", provider: "local" },
    ];
    expect(
      filterAvailableCatalogModels(
        models,
        { openai: "sk" },
        ["hidden"],
        needs,
      ).map((m) => m.id),
    ).toEqual(["ok", "local"]);
  });
});
