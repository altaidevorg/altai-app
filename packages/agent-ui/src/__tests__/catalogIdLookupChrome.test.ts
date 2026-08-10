import { describe, expect, it } from "vitest";
import {
  catalogModelLabel,
  findCatalogEntryById,
} from "../lib/catalogIdLookupChrome.js";

describe("findCatalogEntryById", () => {
  it("returns the matching entry", () => {
    expect(
      findCatalogEntryById([{ id: "a" }, { id: "b" }], "b")?.id,
    ).toBe("b");
  });

  it("ignores empty ids", () => {
    expect(findCatalogEntryById([{ id: "a" }], null)).toBeUndefined();
    expect(findCatalogEntryById([{ id: "a" }], "")).toBeUndefined();
  });
});

describe("catalogModelLabel", () => {
  it("prefers label then raw id", () => {
    expect(
      catalogModelLabel([{ id: "m1", label: "Fast" }], "m1"),
    ).toBe("Fast");
    expect(catalogModelLabel([{ id: "m1" }], "m2")).toBe("m2");
    expect(catalogModelLabel([], null)).toBeUndefined();
  });
});
