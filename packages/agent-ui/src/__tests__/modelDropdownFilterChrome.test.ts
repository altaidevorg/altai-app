import { describe, expect, it } from "vitest";
import {
  filterCatalogModelsForDropdown,
  modelDropdownShowSections,
  partitionModelsByFavRecent,
} from "../lib/modelDropdownFilterChrome.js";

const models = [
  {
    id: "a",
    label: "Alpha",
    hint: "fast",
    description: "chat",
    provider: "openai",
  },
  {
    id: "b",
    label: "Beta",
    hint: "smart",
    description: "code",
    provider: "anthropic",
  },
  {
    id: "c",
    label: "Gamma",
    hint: "local",
    description: "offline",
    provider: "openai",
  },
];

describe("modelDropdownShowSections", () => {
  it("only when no search and no provider filter", () => {
    expect(modelDropdownShowSections("", null)).toBe(true);
    expect(modelDropdownShowSections("  ", null)).toBe(true);
    expect(modelDropdownShowSections("x", null)).toBe(false);
    expect(modelDropdownShowSections("", "openai")).toBe(false);
  });
});

describe("filterCatalogModelsForDropdown", () => {
  it("filters by provider and free text", () => {
    expect(
      filterCatalogModelsForDropdown(models, {
        search: "",
        provider: "openai",
      }).map((m) => m.id),
    ).toEqual(["a", "c"]);
    expect(
      filterCatalogModelsForDropdown(models, {
        search: "smart",
        provider: null,
      }).map((m) => m.id),
    ).toEqual(["b"]);
  });

  it("applies compatibility predicate", () => {
    expect(
      filterCatalogModelsForDropdown(models, {
        search: "",
        isCompatible: (m) => m.id !== "b",
      }).map((m) => m.id),
    ).toEqual(["a", "c"]);
  });
});

describe("partitionModelsByFavRecent", () => {
  it("orders fav then recent then remaining", () => {
    const sections = partitionModelsByFavRecent(
      models,
      ["c", "missing"],
      ["a", "c"],
      true,
    );
    expect(sections.pinned.map((m) => m.id)).toEqual(["c"]);
    expect(sections.recent.map((m) => m.id)).toEqual(["a"]);
    expect(sections.remaining.map((m) => m.id)).toEqual(["b"]);
  });

  it("flattens when sections hidden", () => {
    const sections = partitionModelsByFavRecent(models, ["c"], ["a"], false);
    expect(sections.pinned).toEqual([]);
    expect(sections.recent).toEqual([]);
    expect(sections.remaining.map((m) => m.id)).toEqual(["a", "b", "c"]);
  });
});
