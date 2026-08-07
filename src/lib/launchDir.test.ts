import { describe, expect, it } from "vitest";
import { getStudioFolderFromUrl } from "./launchDir";

describe("getStudioFolderFromUrl", () => {
  it("returns null when the folder query is absent", () => {
    expect(getStudioFolderFromUrl("?mode=studio")).toBeNull();
    expect(getStudioFolderFromUrl("")).toBeNull();
  });

  it("decodes a percent-encoded workspace path", () => {
    expect(
      getStudioFolderFromUrl("?mode=studio&folder=/Users/me/My%20Project"),
    ).toBe("/Users/me/My Project");
  });

  it("normalizes Windows separators", () => {
    expect(
      getStudioFolderFromUrl("?folder=C%3A%5CUsers%5Cme%5Crepo"),
    ).toBe("C:/Users/me/repo");
  });
});
